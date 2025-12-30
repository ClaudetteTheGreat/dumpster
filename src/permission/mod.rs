pub mod category;
pub mod category_values;
pub mod collection;
pub mod collection_values;
pub mod error;
pub mod flag;
pub mod item;
pub mod item_values;
pub mod mask;
pub mod resource;
mod test;

pub use category::Category;
pub use category_values::CategoryValues;
pub use flag::Flag;
pub use item::Item;

// Values sort like this:
// Item:       Y/N
// Category:   {Yes,No,Never} * u64[array of Item flags]
// Collection: u32[array of Category values]

/// Maximum number of permission categories
pub const GROUP_LIMIT: u32 = 16;
/// Maximum number of permissions per category (64 bits)
pub const PERM_LIMIT: u32 = u64::BITS;
/// Total maximum number of permissions defined as GROUP_LIMIT*PERM_LIMIT
pub const MAX_PERMS: u32 = GROUP_LIMIT * PERM_LIMIT;

use crate::middleware::ClientCtx;
use dashmap::DashMap;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::sync::RwLock;

/// Global permission data store
static PERMISSION_DATA: OnceCell<RwLock<PermissionData>> = OnceCell::new();

/// Get a read guard to the global permission data
pub fn get_permission_data() -> std::sync::RwLockReadGuard<'static, PermissionData> {
    PERMISSION_DATA
        .get()
        .expect("Permission data not initialized")
        .read()
        .expect("Permission data lock poisoned")
}

/// Initialize the global permission data (call once at startup)
pub fn init_permission_data(data: PermissionData) {
    PERMISSION_DATA
        .set(RwLock::new(data))
        .expect("Permission data already initialized");
}

/// Reload forum permissions from database
/// Call this after modifying forum permissions via admin UI
pub async fn reload_forum_permissions() -> Result<(), sea_orm::error::DbErr> {
    use crate::db::get_db_pool;
    use crate::orm::forum_permissions;
    use crate::orm::forums;
    use crate::orm::permission_collections;
    use crate::orm::permission_values;
    use collection_values::CollectionValues;
    use sea_orm::entity::*;
    use sea_orm::QueryFilter;

    log::info!("Reloading forum permissions from database...");

    // Get write lock
    let mut perm_data = PERMISSION_DATA
        .get()
        .expect("Permission data not initialized")
        .write()
        .expect("Permission data lock poisoned");

    // Load forum permissions
    let forum_perm_rows = forum_permissions::Entity::find()
        .find_with_related(permission_collections::Entity)
        .all(get_db_pool())
        .await?;

    let mut forum_perms_map: HashMap<i32, DashMap<(i32, i32), CollectionValues>> = HashMap::new();

    for (fp, collections) in forum_perm_rows {
        let forum_id = fp.forum_id;

        for pc in collections {
            // Load permission values for this collection
            let pvs = permission_values::Entity::find()
                .filter(permission_values::Column::CollectionId.eq(pc.id))
                .all(get_db_pool())
                .await?;

            let mut cv = CollectionValues::default();

            for pv in pvs {
                if let Some(pindices) = perm_data.collection.lookup.get(&pv.permission_id) {
                    cv.set_flag(pindices.0, pindices.1, pv.value);
                }
            }

            let val_key = (pc.group_id.unwrap_or(0), pc.user_id.unwrap_or(0));

            let forum_vals = forum_perms_map.entry(forum_id).or_insert_with(DashMap::new);

            if forum_vals.contains_key(&val_key) {
                forum_vals.alter(&val_key, |_, v| cv.join(&v));
            } else {
                forum_vals.insert(val_key, cv);
            }
        }
    }

    // Load forum parent relationships
    let forum_rows = forums::Entity::find().all(get_db_pool()).await?;

    let forum_parents: HashMap<i32, Option<i32>> = forum_rows
        .into_iter()
        .map(|f| (f.id, f.parent_id))
        .collect();

    // Update the permission data
    perm_data.forum_permissions = forum_perms_map;
    perm_data.forum_parents = forum_parents;

    log::info!("Forum permissions reloaded successfully");

    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct PermissionData {
    /// Threadsafe Data Structure
    collection: collection::Collection,
    /// (Group, User) -> CollectionValues Relationship
    collection_values: DashMap<(i32, i32), collection_values::CollectionValues>,
    /// Forum-specific permissions: forum_id -> (group_id, user_id) -> CollectionValues
    forum_permissions: HashMap<i32, DashMap<(i32, i32), collection_values::CollectionValues>>,
    /// Forum parent relationships for inheritance: forum_id -> parent_id
    forum_parents: HashMap<i32, Option<i32>>,
}

impl PermissionData {
    /// Accepts Client/Guest and Permission Name for permission check.
    pub fn can(&self, client: &ClientCtx, permission: &str) -> bool {
        // Look up the permissions's indices by name.
        if let Some(pindices) = self.collection.dictionary.get(permission) {
            self.can_by_indices(client, &pindices)
        } else {
            log::warn!(
                "Bad permission check on name '{:?}', which is not present in our dictionary.",
                permission
            );
            false
        }
    }

    /// Accepts Client/Guest and Permission ID for permission check.
    pub fn can_by_id(&self, client: &ClientCtx, permission_id: i32) -> bool {
        // Look up the permissions's indices by id.
        if let Some(pindices) = self.collection.lookup.get(&permission_id) {
            self.can_by_indices(client, &pindices)
        } else {
            log::warn!(
                "Bad permission check on id {:?}, which is not present in our dictionary.",
                permission_id
            );
            false
        }
    }

    /// Accepts Client/Guest and specific permission indices for permission check.
    pub fn can_by_indices(&self, client: &ClientCtx, indices: &(u8, u8)) -> bool {
        let groups = client.get_groups();
        let values = match client.get_id() {
            Some(id) => {
                let group_values = self.join_for_groups(&groups);
                let user_values = self.join_for_user(id);
                group_values.join(&user_values)
            }
            None => self.join_for_groups(&groups),
        };

        let mask = mask::Mask::from(values);
        mask.can(indices.0 as usize, indices.1 as i32)
    }

    pub fn join_for_groups(&self, groups: &Vec<i32>) -> collection_values::CollectionValues {
        use collection_values::CollectionValues;
        let mut return_values = CollectionValues::default();

        for group in groups {
            let val_key = (group.to_owned(), 0);

            if let Some(group_values) = self.collection_values.get(&val_key) {
                return_values = return_values.join(&group_values);
            }
        }

        return_values
    }

    pub fn join_for_user(&self, id: i32) -> collection_values::CollectionValues {
        use collection_values::CollectionValues;
        let mut return_values = CollectionValues::default();
        let val_key = (0, id);

        if let Some(group_values) = self.collection_values.get(&val_key) {
            return_values = return_values.join(&group_values);
        }

        return_values
    }

    /// Check permission in forum context with parent inheritance.
    /// Walks up the forum hierarchy until an override is found.
    /// Uses global permission store for forum data to support live reloading.
    pub fn can_in_forum(&self, client: &ClientCtx, forum_id: i32, permission: &str) -> bool {
        // Look up the permission's indices by name
        let pindices = match self.collection.dictionary.get(permission) {
            Some(indices) => *indices,
            None => {
                log::warn!(
                    "Bad permission check on name '{:?}', which is not present in our dictionary.",
                    permission
                );
                return false;
            }
        };

        let groups = client.get_groups();
        let user_id = client.get_id();
        let mut current_forum_id = Some(forum_id);

        // Access the global permission data for forum-specific checks
        // This allows live reloading of forum permissions without server restart
        let global_perm_data = get_permission_data();

        // Walk up the forum hierarchy
        while let Some(fid) = current_forum_id {
            // Check if this forum has permission overrides
            if let Some(forum_perms) = global_perm_data.forum_permissions.get(&fid) {
                // Build values from forum-specific group permissions
                let mut forum_values = collection_values::CollectionValues::default();
                let mut has_override = false;

                // Check group permissions for this forum
                for group in &groups {
                    let val_key = (*group, 0);
                    if let Some(group_values) = forum_perms.get(&val_key) {
                        forum_values = forum_values.join(&group_values);
                        has_override = true;
                    }
                }

                // Check user-specific permissions for this forum
                if let Some(uid) = user_id {
                    let val_key = (0, uid);
                    if let Some(user_values) = forum_perms.get(&val_key) {
                        forum_values = forum_values.join(&user_values);
                        has_override = true;
                    }
                }

                // If we found any overrides for this forum, check if this permission is explicitly set
                if has_override && forum_values.has_explicit_value(pindices.0 as usize, pindices.1)
                {
                    return forum_values.can(pindices.0 as usize, pindices.1);
                }
            }

            // Move to parent forum
            current_forum_id = global_perm_data.forum_parents.get(&fid).copied().flatten();
        }

        // No forum overrides in chain - fall back to global permissions
        self.can_by_indices(client, &pindices)
    }

    /// Get the parent forum ID for a given forum
    pub fn get_forum_parent(&self, forum_id: i32) -> Option<i32> {
        // Use global store for live reloading support
        get_permission_data()
            .forum_parents
            .get(&forum_id)
            .copied()
            .flatten()
    }
}

pub async fn new() -> Result<PermissionData, sea_orm::error::DbErr> {
    use crate::db::get_db_pool;
    use crate::orm::forum_permissions;
    use crate::orm::forums;
    use crate::orm::permission_collections;
    use crate::orm::permission_values;
    use crate::orm::permissions;
    use collection_values::CollectionValues;
    use sea_orm::entity::*;
    use sea_orm::QueryFilter;

    // Build structure tree
    let mut col = collection::Collection::default();

    // Import permissions
    let items = permissions::Entity::find().all(get_db_pool()).await?;

    // Pull unique category id list from permissions.
    let mut ucid: Vec<i32> = items.iter().map(|i| i.category_id).collect();
    ucid.sort_unstable();
    ucid.dedup();

    // Add categories to collection and order them.
    for (i, cid) in ucid.iter().enumerate() {
        col.categories[i].id = *cid;
        col.categories[i].position = i as u8;

        // Add permissions belonging to this category.
        for item in items.iter() {
            if *cid == item.category_id {
                match col.categories[i].add_item(item.id, &item.label) {
                    Ok(item) => {
                        col.dictionary
                            .insert(item.label.to_owned(), (i as u8, item.position));
                        col.lookup.insert(item.id, (i as u8, item.position));
                    }
                    Err(_) => {
                        println!("Category overflow adding {:?}", item);
                    }
                }
            }
        }
    }

    // Import data
    let vals: DashMap<(i32, i32), CollectionValues> = Default::default();
    let perm_collections = permission_collections::Entity::find()
        .find_with_related(permission_values::Entity)
        .all(get_db_pool())
        .await?;

    // convert ORM data into permission system structs
    // loop through the collection-<values relations
    for (perm_collection, pvs) in perm_collections.iter() {
        // Create collection values record to set flags on
        let mut cv = CollectionValues::default();

        // loop through the values
        for pv in pvs.iter() {
            // Look up the permissions's indices by id.
            if let Some(pindices) = col.lookup.get(&pv.permission_id) {
                // Assign each flag to the CollectionValues.
                cv.set_flag(pindices.0, pindices.1, pv.value);
            } else {
                println!(
                    "Failed to lookup indices for permission_values {:?},{:?}",
                    pv.collection_id, pv.permission_id
                );
            }
        }

        // Resolve (group,user) tuple key
        let val_key: (i32, i32) = (
            perm_collection.group_id.unwrap_or(0),
            perm_collection.user_id.unwrap_or(0),
        );

        if vals.contains_key(&val_key) {
            // Join permission with same key.
            vals.alter(&val_key, |_, v| cv.join(&v));
        } else {
            // Add to values lookup.
            vals.insert(val_key, cv);
        }
    }

    // Load forum permissions
    let forum_perm_rows = forum_permissions::Entity::find()
        .find_with_related(permission_collections::Entity)
        .all(get_db_pool())
        .await?;

    let mut forum_perms_map: HashMap<i32, DashMap<(i32, i32), CollectionValues>> = HashMap::new();

    for (fp, collections) in forum_perm_rows {
        let forum_id = fp.forum_id;

        for pc in collections {
            // Load permission values for this collection
            let pvs = permission_values::Entity::find()
                .filter(permission_values::Column::CollectionId.eq(pc.id))
                .all(get_db_pool())
                .await?;

            let mut cv = CollectionValues::default();

            for pv in pvs {
                if let Some(pindices) = col.lookup.get(&pv.permission_id) {
                    cv.set_flag(pindices.0, pindices.1, pv.value);
                }
            }

            let val_key = (pc.group_id.unwrap_or(0), pc.user_id.unwrap_or(0));

            let forum_vals = forum_perms_map.entry(forum_id).or_insert_with(DashMap::new);

            if forum_vals.contains_key(&val_key) {
                forum_vals.alter(&val_key, |_, v| cv.join(&v));
            } else {
                forum_vals.insert(val_key, cv);
            }
        }
    }

    // Load forum parent relationships
    let forum_rows = forums::Entity::find().all(get_db_pool()).await?;

    let forum_parents: HashMap<i32, Option<i32>> = forum_rows
        .into_iter()
        .map(|f| (f.id, f.parent_id))
        .collect();

    Ok(PermissionData {
        collection: col,
        collection_values: vals,
        forum_permissions: forum_perms_map,
        forum_parents,
    })
}
