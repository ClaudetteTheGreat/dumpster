//! Tests for the three-tier content deletion system:
//! - Normal: Soft delete, visible to moderators, can be restored
//! - Permanent: Content purged, audit trail kept (for spam)
//! - Legal hold: Cannot be modified except by admin

mod common;
use serial_test::serial;

use common::{database::*, fixtures::*};
use ruforo::orm::ugc_deletions::DeletionType;
use sea_orm::{entity::*, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

/// Test that DeletionType enum has correct values
#[test]
fn test_deletion_type_enum_values() {
    let normal = DeletionType::Normal;
    let permanent = DeletionType::Permanent;
    let legal_hold = DeletionType::LegalHold;

    assert_eq!(normal, DeletionType::Normal);
    assert_eq!(permanent, DeletionType::Permanent);
    assert_eq!(legal_hold, DeletionType::LegalHold);
}

/// Test that DeletionType default is Normal
#[test]
fn test_deletion_type_default() {
    let default_type: DeletionType = Default::default();
    assert_eq!(default_type, DeletionType::Normal);
}

/// Test creating a normal (soft) deletion record
#[actix_rt::test]
#[serial]
async fn test_normal_deletion() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a test user
    let user = create_test_user(&db, "delete_normal_user", "password123")
        .await
        .expect("Failed to create user");

    // Create UGC first
    use ruforo::orm::{ugc, ugc_revisions};

    let ugc_model = ugc::ActiveModel {
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Should create UGC");

    let _revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(Some(user.id)),
        content: Set("Test content for deletion".to_string()),
        ip_id: Set(None),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Should create revision");

    // Create deletion record
    use ruforo::orm::ugc_deletions;

    let deletion = ugc_deletions::ActiveModel {
        id: Set(ugc_model.id),
        user_id: Set(Some(user.id)),
        deleted_at: Set(chrono::Utc::now().naive_utc()),
        reason: Set(Some("Spam".to_string())),
        deletion_type: Set(DeletionType::Normal),
        deleted_by_id: Set(Some(user.id)),
        legal_hold_at: Set(None),
        legal_hold_by: Set(None),
        legal_hold_reason: Set(None),
    };

    let result = deletion.insert(&db).await;
    assert!(
        result.is_ok(),
        "Should successfully create normal deletion record"
    );

    // Verify the deletion record
    let saved = ugc_deletions::Entity::find_by_id(ugc_model.id)
        .one(&db)
        .await
        .expect("Query should succeed")
        .expect("Should find the deletion record");

    assert_eq!(saved.deletion_type, DeletionType::Normal);
    assert_eq!(saved.reason, Some("Spam".to_string()));
    assert!(saved.legal_hold_at.is_none());

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

/// Test creating a permanent deletion record
#[actix_rt::test]
#[serial]
async fn test_permanent_deletion() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "delete_perm_user", "password123")
        .await
        .expect("Failed to create user");

    use ruforo::orm::{ugc, ugc_deletions, ugc_revisions};

    let ugc_model = ugc::ActiveModel {
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Should create UGC");

    let _revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(Some(user.id)),
        content: Set("Content to be permanently deleted".to_string()),
        ip_id: Set(None),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Should create revision");

    let deletion = ugc_deletions::ActiveModel {
        id: Set(ugc_model.id),
        user_id: Set(Some(user.id)),
        deleted_at: Set(chrono::Utc::now().naive_utc()),
        reason: Set(Some("Illegal content".to_string())),
        deletion_type: Set(DeletionType::Permanent),
        deleted_by_id: Set(Some(user.id)),
        legal_hold_at: Set(None),
        legal_hold_by: Set(None),
        legal_hold_reason: Set(None),
    };

    let result = deletion.insert(&db).await;
    assert!(result.is_ok());

    let saved = ugc_deletions::Entity::find_by_id(ugc_model.id)
        .one(&db)
        .await
        .expect("Query should succeed")
        .expect("Should find the deletion record");

    assert_eq!(saved.deletion_type, DeletionType::Permanent);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

/// Test creating a legal hold record
#[actix_rt::test]
#[serial]
async fn test_legal_hold_deletion() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "delete_legal_user", "password123")
        .await
        .expect("Failed to create user");
    let admin = create_test_user(&db, "admin_legal_hold", "adminpass")
        .await
        .expect("Failed to create admin");

    use ruforo::orm::{ugc, ugc_deletions, ugc_revisions};

    let ugc_model = ugc::ActiveModel {
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Should create UGC");

    let _revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(Some(user.id)),
        content: Set("Content under legal hold".to_string()),
        ip_id: Set(None),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Should create revision");

    let now = chrono::Utc::now().naive_utc();

    let deletion = ugc_deletions::ActiveModel {
        id: Set(ugc_model.id),
        user_id: Set(Some(user.id)),
        deleted_at: Set(now),
        reason: Set(Some("Court order".to_string())),
        deletion_type: Set(DeletionType::LegalHold),
        deleted_by_id: Set(Some(admin.id)),
        legal_hold_at: Set(Some(now)),
        legal_hold_by: Set(Some(admin.id)),
        legal_hold_reason: Set(Some("Preserve for litigation".to_string())),
    };

    let result = deletion.insert(&db).await;
    assert!(result.is_ok());

    let saved = ugc_deletions::Entity::find_by_id(ugc_model.id)
        .one(&db)
        .await
        .expect("Query should succeed")
        .expect("Should find the deletion record");

    assert_eq!(saved.deletion_type, DeletionType::LegalHold);
    assert!(saved.legal_hold_at.is_some());
    assert_eq!(saved.legal_hold_by, Some(admin.id));
    assert_eq!(
        saved.legal_hold_reason,
        Some("Preserve for litigation".to_string())
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

/// Test that deleted_by tracks who performed the deletion
#[actix_rt::test]
#[serial]
async fn test_deleted_by_tracking() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let author = create_test_user(&db, "content_author", "password123")
        .await
        .expect("Failed to create author");
    let moderator = create_test_user(&db, "moderator_user", "modpass")
        .await
        .expect("Failed to create moderator");

    use ruforo::orm::{ugc, ugc_deletions, ugc_revisions};

    let ugc_model = ugc::ActiveModel {
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Should create UGC");

    let _revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(Some(author.id)),
        content: Set("Content by author".to_string()),
        ip_id: Set(None),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Should create revision");

    // Moderator deletes content authored by someone else
    let deletion = ugc_deletions::ActiveModel {
        id: Set(ugc_model.id),
        user_id: Set(Some(author.id)), // Original author
        deleted_at: Set(chrono::Utc::now().naive_utc()),
        reason: Set(Some("Rule violation".to_string())),
        deletion_type: Set(DeletionType::Normal),
        deleted_by_id: Set(Some(moderator.id)), // Moderator who deleted
        legal_hold_at: Set(None),
        legal_hold_by: Set(None),
        legal_hold_reason: Set(None),
    };

    let result = deletion.insert(&db).await;
    assert!(result.is_ok());

    let saved = ugc_deletions::Entity::find_by_id(ugc_model.id)
        .one(&db)
        .await
        .expect("Query should succeed")
        .expect("Should find the deletion record");

    // user_id is the original author
    assert_eq!(saved.user_id, Some(author.id));
    // deleted_by_id is the moderator who performed the deletion
    assert_eq!(saved.deleted_by_id, Some(moderator.id));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

/// Test thread deletion fields
#[actix_rt::test]
#[serial]
async fn test_thread_deletion_fields() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "thread_delete_user", "password123")
        .await
        .expect("Failed to create user");

    let (_, thread) = create_test_forum_and_thread(&db, user.id, "Test Thread for Deletion")
        .await
        .expect("Failed to create forum and thread");

    // Verify thread was created with empty deletion fields
    assert!(thread.deleted_at.is_none());
    assert!(thread.deletion_type.is_none());

    // Now soft-delete the thread
    use ruforo::orm::threads;

    let now = chrono::Utc::now().naive_utc();
    let mut thread_active: threads::ActiveModel = thread.into();
    thread_active.deleted_at = Set(Some(now));
    thread_active.deleted_by = Set(Some(user.id));
    thread_active.deletion_type = Set(Some(DeletionType::Normal));
    thread_active.deletion_reason = Set(Some("Test deletion".to_string()));

    let updated = thread_active
        .update(&db)
        .await
        .expect("Should update thread");

    assert!(updated.deleted_at.is_some());
    assert_eq!(updated.deleted_by, Some(user.id));
    assert_eq!(updated.deletion_type, Some(DeletionType::Normal));
    assert_eq!(updated.deletion_reason, Some("Test deletion".to_string()));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

/// Test that deletion types can be queried/filtered
#[actix_rt::test]
#[serial]
async fn test_query_by_deletion_type() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "query_delete_user", "password123")
        .await
        .expect("Failed to create user");

    use ruforo::orm::{ugc, ugc_deletions, ugc_revisions};

    // Create multiple UGC entries with different deletion types
    let mut ugc_ids = Vec::new();
    for i in 0..3 {
        let ugc_model = ugc::ActiveModel {
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("Should create UGC");

        let _revision = ugc_revisions::ActiveModel {
            ugc_id: Set(ugc_model.id),
            user_id: Set(Some(user.id)),
            content: Set(format!("Content {}", i)),
            ip_id: Set(None),
            created_at: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("Should create revision");

        ugc_ids.push(ugc_model.id);
    }

    let now = chrono::Utc::now().naive_utc();

    // Create deletion records with different types
    for (ugc_id, del_type) in [
        (ugc_ids[0], DeletionType::Normal),
        (ugc_ids[1], DeletionType::Permanent),
        (ugc_ids[2], DeletionType::LegalHold),
    ] {
        ugc_deletions::ActiveModel {
            id: Set(ugc_id),
            user_id: Set(Some(user.id)),
            deleted_at: Set(now),
            reason: Set(None),
            deletion_type: Set(del_type),
            deleted_by_id: Set(Some(user.id)),
            legal_hold_at: Set(None),
            legal_hold_by: Set(None),
            legal_hold_reason: Set(None),
        }
        .insert(&db)
        .await
        .expect("Should create deletion record");
    }

    // Query only normal deletions
    let normal_deletions = ugc_deletions::Entity::find()
        .filter(ugc_deletions::Column::DeletionType.eq(DeletionType::Normal))
        .all(&db)
        .await
        .expect("Query should succeed");

    assert!(normal_deletions.iter().any(|d| d.id == ugc_ids[0]));
    assert!(normal_deletions
        .iter()
        .all(|d| d.deletion_type == DeletionType::Normal));

    // Query legal holds
    let legal_holds = ugc_deletions::Entity::find()
        .filter(ugc_deletions::Column::DeletionType.eq(DeletionType::LegalHold))
        .all(&db)
        .await
        .expect("Query should succeed");

    assert!(legal_holds.iter().any(|d| d.id == ugc_ids[2]));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
