/// Integration tests for notification preferences functionality
/// Tests getting and updating notification preferences
mod common;
use serial_test::serial;

use common::{database::*, fixtures::*};
use ruforo::notifications;
use ruforo::orm::notification_preferences;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

#[actix_rt::test]
#[serial]
async fn test_get_all_user_preferences_with_defaults() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create user");

    // Get preferences (should return defaults)
    let prefs = notifications::get_all_user_preferences(user.id)
        .await
        .expect("Failed to get preferences");

    // Should have all 5 notification types
    assert_eq!(prefs.len(), 5);

    // Check that all have default values
    for pref in &prefs {
        assert_eq!(pref.in_app, true);
        assert_eq!(pref.email, true);
        assert_eq!(pref.frequency, "immediate");
    }

    // Verify types are present
    let types: Vec<&str> = prefs.iter().map(|p| p.notification_type.as_str()).collect();
    assert!(types.contains(&"reply"));
    assert!(types.contains(&"mention"));
    assert!(types.contains(&"pm"));
    assert!(types.contains(&"quote"));
    assert!(types.contains(&"thread_watch"));
}

#[actix_rt::test]
#[serial]
async fn test_update_preference() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create user");

    // Update a preference
    notifications::update_preference(user.id, "mention", false, true, "daily")
        .await
        .expect("Failed to update preference");

    // Verify the update
    let pref = notification_preferences::Entity::find()
        .filter(notification_preferences::Column::UserId.eq(user.id))
        .filter(notification_preferences::Column::NotificationType.eq("mention"))
        .one(&db)
        .await
        .expect("Failed to find preference")
        .expect("Preference not found");

    assert_eq!(pref.in_app, false);
    assert_eq!(pref.email, true);
    assert_eq!(pref.frequency, "daily");
}

#[actix_rt::test]
#[serial]
async fn test_update_preference_creates_if_not_exists() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "charlie", "charlie@example.com", true)
        .await
        .expect("Failed to create user");

    // Verify no preference exists initially
    let count_before = notification_preferences::Entity::find()
        .filter(notification_preferences::Column::UserId.eq(user.id))
        .filter(notification_preferences::Column::NotificationType.eq("reply"))
        .count(&db)
        .await
        .expect("Failed to count");

    assert_eq!(count_before, 0);

    // Update preference (should create it)
    notifications::update_preference(user.id, "reply", true, false, "hourly")
        .await
        .expect("Failed to update preference");

    // Verify it was created
    let pref = notification_preferences::Entity::find()
        .filter(notification_preferences::Column::UserId.eq(user.id))
        .filter(notification_preferences::Column::NotificationType.eq("reply"))
        .one(&db)
        .await
        .expect("Failed to find preference")
        .expect("Preference not found");

    assert_eq!(pref.in_app, true);
    assert_eq!(pref.email, false);
    assert_eq!(pref.frequency, "hourly");
}

#[actix_rt::test]
#[serial]
async fn test_update_preference_multiple_times() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "dave", "dave@example.com", true)
        .await
        .expect("Failed to create user");

    // Update preference multiple times
    notifications::update_preference(user.id, "pm", true, true, "immediate")
        .await
        .expect("Failed to update preference 1");

    notifications::update_preference(user.id, "pm", false, false, "never")
        .await
        .expect("Failed to update preference 2");

    notifications::update_preference(user.id, "pm", true, false, "daily")
        .await
        .expect("Failed to update preference 3");

    // Verify final state
    let pref = notification_preferences::Entity::find()
        .filter(notification_preferences::Column::UserId.eq(user.id))
        .filter(notification_preferences::Column::NotificationType.eq("pm"))
        .one(&db)
        .await
        .expect("Failed to find preference")
        .expect("Preference not found");

    assert_eq!(pref.in_app, true);
    assert_eq!(pref.email, false);
    assert_eq!(pref.frequency, "daily");

    // Verify only one record exists
    let count = notification_preferences::Entity::find()
        .filter(notification_preferences::Column::UserId.eq(user.id))
        .filter(notification_preferences::Column::NotificationType.eq("pm"))
        .count(&db)
        .await
        .expect("Failed to count");

    assert_eq!(count, 1);
}

#[actix_rt::test]
#[serial]
async fn test_get_all_preferences_with_custom_settings() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "eve", "eve@example.com", true)
        .await
        .expect("Failed to create user");

    // Update several preferences
    notifications::update_preference(user.id, "reply", false, true, "hourly")
        .await
        .expect("Failed to update reply");

    notifications::update_preference(user.id, "mention", true, false, "daily")
        .await
        .expect("Failed to update mention");

    notifications::update_preference(user.id, "pm", false, false, "never")
        .await
        .expect("Failed to update pm");

    // Get all preferences
    let prefs = notifications::get_all_user_preferences(user.id)
        .await
        .expect("Failed to get preferences");

    // Find specific preferences and verify
    let reply_pref = prefs
        .iter()
        .find(|p| p.notification_type == "reply")
        .unwrap();
    assert_eq!(reply_pref.in_app, false);
    assert_eq!(reply_pref.email, true);
    assert_eq!(reply_pref.frequency, "hourly");

    let mention_pref = prefs
        .iter()
        .find(|p| p.notification_type == "mention")
        .unwrap();
    assert_eq!(mention_pref.in_app, true);
    assert_eq!(mention_pref.email, false);
    assert_eq!(mention_pref.frequency, "daily");

    let pm_pref = prefs.iter().find(|p| p.notification_type == "pm").unwrap();
    assert_eq!(pm_pref.in_app, false);
    assert_eq!(pm_pref.email, false);
    assert_eq!(pm_pref.frequency, "never");

    // Unchanged preferences should have defaults
    let quote_pref = prefs
        .iter()
        .find(|p| p.notification_type == "quote")
        .unwrap();
    assert_eq!(quote_pref.in_app, true);
    assert_eq!(quote_pref.email, true);
    assert_eq!(quote_pref.frequency, "immediate");
}

#[actix_rt::test]
#[serial]
async fn test_preferences_isolated_between_users() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "frank", "frank@example.com", true)
        .await
        .expect("Failed to create user1");

    let user2 = create_test_user_with_email(&db, "grace", "grace@example.com", true)
        .await
        .expect("Failed to create user2");

    // Update preferences for user1
    notifications::update_preference(user1.id, "reply", false, false, "never")
        .await
        .expect("Failed to update user1 preference");

    // Update different preferences for user2
    notifications::update_preference(user2.id, "reply", true, true, "immediate")
        .await
        .expect("Failed to update user2 preference");

    // Verify user1's preferences
    let user1_prefs = notifications::get_all_user_preferences(user1.id)
        .await
        .expect("Failed to get user1 preferences");

    let user1_reply = user1_prefs
        .iter()
        .find(|p| p.notification_type == "reply")
        .unwrap();
    assert_eq!(user1_reply.in_app, false);
    assert_eq!(user1_reply.email, false);
    assert_eq!(user1_reply.frequency, "never");

    // Verify user2's preferences
    let user2_prefs = notifications::get_all_user_preferences(user2.id)
        .await
        .expect("Failed to get user2 preferences");

    let user2_reply = user2_prefs
        .iter()
        .find(|p| p.notification_type == "reply")
        .unwrap();
    assert_eq!(user2_reply.in_app, true);
    assert_eq!(user2_reply.email, true);
    assert_eq!(user2_reply.frequency, "immediate");
}

#[actix_rt::test]
#[serial]
async fn test_preference_display_has_labels() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "henry", "henry@example.com", true)
        .await
        .expect("Failed to create user");

    let prefs = notifications::get_all_user_preferences(user.id)
        .await
        .expect("Failed to get preferences");

    // Verify all preferences have labels and descriptions
    for pref in prefs {
        assert!(
            !pref.type_label.is_empty(),
            "type_label should not be empty"
        );
        assert!(
            !pref.type_description.is_empty(),
            "type_description should not be empty"
        );
        assert!(
            !pref.notification_type.is_empty(),
            "notification_type should not be empty"
        );
    }
}
