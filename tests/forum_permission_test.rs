/// Integration tests for forum-specific permissions
/// Tests forum permission overrides and sub-forum inheritance
mod common;
use serial_test::serial;

use common::database::*;
use ruforo::orm::{
    forum_permissions, forums, groups, permission_collections, permission_values, permissions,
};
use ruforo::permission::flag::Flag;
use sea_orm::{entity::*, query::*, ActiveValue::Set, DatabaseConnection, DbErr};

/// Create a test forum with optional parent
async fn create_forum(
    db: &DatabaseConnection,
    name: &str,
    parent_id: Option<i32>,
) -> Result<forums::Model, DbErr> {
    let forum = forums::ActiveModel {
        label: Set(name.to_string()),
        description: Set(Some("Test forum".to_string())),
        parent_id: Set(parent_id),
        display_order: Set(0),
        ..Default::default()
    };
    forum.insert(db).await
}

/// Create a test group
async fn create_test_group(db: &DatabaseConnection, name: &str) -> Result<groups::Model, DbErr> {
    let group = groups::ActiveModel {
        label: Set(name.to_string()),
        group_type: Set(ruforo::group::GroupType::Normal),
        ..Default::default()
    };
    group.insert(db).await
}

/// Create a permission collection for forum-specific permissions
async fn create_forum_permission_collection(
    db: &DatabaseConnection,
    group_id: i32,
) -> Result<permission_collections::Model, DbErr> {
    let collection = permission_collections::ActiveModel {
        group_id: Set(Some(group_id)),
        user_id: Set(None),
        ..Default::default()
    };
    collection.insert(db).await
}

/// Link a permission collection to a forum
async fn link_collection_to_forum(
    db: &DatabaseConnection,
    forum_id: i32,
    collection_id: i32,
) -> Result<forum_permissions::Model, DbErr> {
    let fp = forum_permissions::ActiveModel {
        forum_id: Set(forum_id),
        collection_id: Set(collection_id),
    };
    fp.insert(db).await
}

/// Set a permission value in a collection
async fn set_permission_value(
    db: &DatabaseConnection,
    collection_id: i32,
    permission_id: i32,
    value: Flag,
) -> Result<permission_values::Model, DbErr> {
    let pv = permission_values::ActiveModel {
        collection_id: Set(collection_id),
        permission_id: Set(permission_id),
        value: Set(value),
    };
    pv.insert(db).await
}

/// Find a permission by name
async fn find_permission_by_name(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Option<permissions::Model>, DbErr> {
    permissions::Entity::find()
        .filter(permissions::Column::Label.eq(name))
        .one(db)
        .await
}

#[actix_rt::test]
#[serial]
async fn test_forum_parent_relationship() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create parent forum
    let parent_forum = create_forum(&db, "Parent Forum", None)
        .await
        .expect("Failed to create parent forum");

    // Create child forum
    let child_forum = create_forum(&db, "Child Forum", Some(parent_forum.id))
        .await
        .expect("Failed to create child forum");

    // Verify parent-child relationship
    assert_eq!(child_forum.parent_id, Some(parent_forum.id));
    assert!(parent_forum.parent_id.is_none());

    // Fetch child and verify parent
    let fetched_child = forums::Entity::find_by_id(child_forum.id)
        .one(&db)
        .await
        .expect("Failed to fetch child forum")
        .expect("Child forum not found");

    assert_eq!(fetched_child.parent_id, Some(parent_forum.id));
}

#[actix_rt::test]
#[serial]
async fn test_forum_permission_collection_link() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a forum
    let forum = create_forum(&db, "Test Forum", None)
        .await
        .expect("Failed to create forum");

    // Create a test group first
    let group = create_test_group(&db, "Test Group")
        .await
        .expect("Failed to create test group");

    // Create a permission collection for the group
    let collection = create_forum_permission_collection(&db, group.id)
        .await
        .expect("Failed to create permission collection");

    // Link collection to forum
    let fp = link_collection_to_forum(&db, forum.id, collection.id)
        .await
        .expect("Failed to link collection to forum");

    assert_eq!(fp.forum_id, forum.id);
    assert_eq!(fp.collection_id, collection.id);

    // Verify the link exists
    let found = forum_permissions::Entity::find()
        .filter(forum_permissions::Column::ForumId.eq(forum.id))
        .filter(forum_permissions::Column::CollectionId.eq(collection.id))
        .one(&db)
        .await
        .expect("Failed to query forum_permissions")
        .expect("Forum permission not found");

    assert_eq!(found.forum_id, forum.id);
    assert_eq!(found.collection_id, collection.id);
}

#[actix_rt::test]
#[serial]
async fn test_forum_permission_value_set() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Find the thread.create permission
    let permission = find_permission_by_name(&db, "thread.create")
        .await
        .expect("Failed to query permission");

    // Skip test if permission not found (not seeded)
    let permission = match permission {
        Some(p) => p,
        None => {
            println!("Skipping test: thread.create permission not found in database");
            return;
        }
    };

    // Create forum
    let forum = create_forum(&db, "Test Forum", None)
        .await
        .expect("Failed to create forum");

    // Create a test group first
    let group = create_test_group(&db, "Test Group")
        .await
        .expect("Failed to create test group");

    // Create permission collection
    let collection = create_forum_permission_collection(&db, group.id)
        .await
        .expect("Failed to create permission collection");

    // Link to forum
    link_collection_to_forum(&db, forum.id, collection.id)
        .await
        .expect("Failed to link collection to forum");

    // Set permission value to NO
    let pv = set_permission_value(&db, collection.id, permission.id, Flag::NO)
        .await
        .expect("Failed to set permission value");

    assert_eq!(pv.value, Flag::NO);

    // Verify the value persists
    let found = permission_values::Entity::find()
        .filter(permission_values::Column::CollectionId.eq(collection.id))
        .filter(permission_values::Column::PermissionId.eq(permission.id))
        .one(&db)
        .await
        .expect("Failed to query permission_values")
        .expect("Permission value not found");

    assert_eq!(found.value, Flag::NO);
}

#[actix_rt::test]
#[serial]
async fn test_deep_forum_hierarchy() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create 3-level hierarchy
    let level1 = create_forum(&db, "Level 1", None)
        .await
        .expect("Failed to create level 1 forum");

    let level2 = create_forum(&db, "Level 2", Some(level1.id))
        .await
        .expect("Failed to create level 2 forum");

    let level3 = create_forum(&db, "Level 3", Some(level2.id))
        .await
        .expect("Failed to create level 3 forum");

    // Verify hierarchy
    assert!(level1.parent_id.is_none());
    assert_eq!(level2.parent_id, Some(level1.id));
    assert_eq!(level3.parent_id, Some(level2.id));

    // Walk up from level3 to root
    let mut current_id = level3.parent_id;
    let mut depth = 0;

    while let Some(parent_id) = current_id {
        let parent = forums::Entity::find_by_id(parent_id)
            .one(&db)
            .await
            .expect("Failed to fetch parent")
            .expect("Parent not found");
        current_id = parent.parent_id;
        depth += 1;
    }

    assert_eq!(depth, 2, "Should have walked up 2 levels to reach root");
}
