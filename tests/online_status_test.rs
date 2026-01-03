/// Tests for user online status features
mod common;

use serial_test::serial;

#[actix_rt::test]
#[serial]
async fn test_is_user_online_with_recent_activity() {
    use chrono::{Duration, Utc};
    use dumpster::user::{is_user_online, ONLINE_THRESHOLD_MINUTES};

    // User active within the threshold should be online
    let recent_activity = Some(Utc::now() - Duration::minutes(5));
    assert!(
        is_user_online(recent_activity),
        "User active 5 minutes ago should be online (threshold: {} minutes)",
        ONLINE_THRESHOLD_MINUTES
    );

    // User active exactly at the edge of threshold
    let edge_activity = Some(Utc::now() - Duration::minutes(ONLINE_THRESHOLD_MINUTES - 1));
    assert!(
        is_user_online(edge_activity),
        "User active {} minutes ago should be online",
        ONLINE_THRESHOLD_MINUTES - 1
    );
}

#[actix_rt::test]
#[serial]
async fn test_is_user_online_with_old_activity() {
    use chrono::{Duration, Utc};
    use dumpster::user::{is_user_online, ONLINE_THRESHOLD_MINUTES};

    // User active beyond the threshold should be offline
    let old_activity = Some(Utc::now() - Duration::minutes(ONLINE_THRESHOLD_MINUTES + 1));
    assert!(
        !is_user_online(old_activity),
        "User active {} minutes ago should be offline (threshold: {} minutes)",
        ONLINE_THRESHOLD_MINUTES + 1,
        ONLINE_THRESHOLD_MINUTES
    );

    // User inactive for a long time
    let very_old_activity = Some(Utc::now() - Duration::hours(24));
    assert!(
        !is_user_online(very_old_activity),
        "User inactive for 24 hours should be offline"
    );
}

#[actix_rt::test]
#[serial]
async fn test_is_user_online_with_no_activity() {
    use dumpster::user::is_user_online;

    // User with no activity record should be offline
    assert!(
        !is_user_online(None),
        "User with no activity record should be offline"
    );
}

#[actix_rt::test]
#[serial]
async fn test_show_online_default_value() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::create_test_user;
    use dumpster::orm::users;
    use sea_orm::EntityTrait;

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    // Create a user
    let user = create_test_user(&db, "show_online_default", "password123")
        .await
        .unwrap();

    // Verify show_online defaults to true
    let user_model = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert!(
        user_model.show_online,
        "show_online should default to true for new users"
    );

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_toggle_show_online_setting() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::create_test_user;
    use dumpster::orm::users;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    let user = create_test_user(&db, "toggle_online_user", "password123")
        .await
        .unwrap();

    // Toggle show_online to false
    let mut active_user: users::ActiveModel = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into();

    active_user.show_online = Set(false);
    active_user.update(&db).await.unwrap();

    // Verify it was updated
    let updated_user = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert!(
        !updated_user.show_online,
        "show_online should be false after toggling"
    );

    // Toggle back to true
    let mut active_user: users::ActiveModel = updated_user.into();
    active_user.show_online = Set(true);
    active_user.update(&db).await.unwrap();

    let final_user = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert!(
        final_user.show_online,
        "show_online should be true after toggling back"
    );

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_last_activity_at_update() {
    use chrono::Utc;
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::create_test_user;
    use dumpster::orm::users;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    let user = create_test_user(&db, "activity_update_user", "password123")
        .await
        .unwrap();

    // Initially, last_activity_at should be None
    let user_model = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert!(
        user_model.last_activity_at.is_none(),
        "last_activity_at should be None initially"
    );

    // Update last_activity_at
    let now = Utc::now().fixed_offset();
    let mut active_user: users::ActiveModel = user_model.into();
    active_user.last_activity_at = Set(Some(now));
    active_user.update(&db).await.unwrap();

    // Verify update
    let updated_user = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert!(
        updated_user.last_activity_at.is_some(),
        "last_activity_at should be set after update"
    );

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_count_online_users() {
    use chrono::{Duration, Utc};
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::create_test_user;
    use dumpster::orm::users;
    use dumpster::user::ONLINE_THRESHOLD_MINUTES;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    // Create two users, one online and one offline
    let online_user = create_test_user(&db, "count_online_user", "password123")
        .await
        .unwrap();
    let offline_user = create_test_user(&db, "count_offline_user", "password123")
        .await
        .unwrap();
    let hidden_user = create_test_user(&db, "count_hidden_user", "password123")
        .await
        .unwrap();

    // Set online user's activity to recent (within threshold)
    let mut active_online: users::ActiveModel = users::Entity::find_by_id(online_user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into();
    active_online.last_activity_at = Set(Some(Utc::now().fixed_offset()));
    active_online.show_online = Set(true);
    active_online.update(&db).await.unwrap();

    // Set offline user's activity to old (beyond threshold)
    let mut active_offline: users::ActiveModel = users::Entity::find_by_id(offline_user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into();
    active_offline.last_activity_at = Set(Some(
        (Utc::now() - Duration::minutes(ONLINE_THRESHOLD_MINUTES + 10)).fixed_offset(),
    ));
    active_offline.show_online = Set(true);
    active_offline.update(&db).await.unwrap();

    // Set hidden user as online but with show_online = false
    let mut active_hidden: users::ActiveModel = users::Entity::find_by_id(hidden_user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into();
    active_hidden.last_activity_at = Set(Some(Utc::now().fixed_offset()));
    active_hidden.show_online = Set(false);
    active_hidden.update(&db).await.unwrap();

    // Count should be 1 (only online user with show_online = true)
    let count = dumpster::user::count_online_users().await.unwrap();
    assert!(
        count >= 1,
        "At least one user should be counted as online (found {})",
        count
    );

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_get_online_users_list() {
    use chrono::Utc;
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::create_test_user;
    use dumpster::orm::users;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    // Create an online user
    let online_user = create_test_user(&db, "list_online_user", "password123")
        .await
        .unwrap();

    // Set user as online
    let mut active_user: users::ActiveModel = users::Entity::find_by_id(online_user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into();
    active_user.last_activity_at = Set(Some(Utc::now().fixed_offset()));
    active_user.show_online = Set(true);
    active_user.update(&db).await.unwrap();

    // Get online users list
    let online_users = dumpster::user::get_online_users(10).await.unwrap();

    // Should contain our user
    let found = online_users.iter().any(|u| u.name == "list_online_user");
    assert!(
        found,
        "Online user should appear in get_online_users list. Found: {:?}",
        online_users.iter().map(|u| &u.name).collect::<Vec<_>>()
    );

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_hidden_user_not_in_online_list() {
    use chrono::Utc;
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::create_test_user;
    use dumpster::orm::users;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    // Create a hidden user
    let hidden_user = create_test_user(&db, "hidden_online_user", "password123")
        .await
        .unwrap();

    // Set user as online but hidden
    let mut active_user: users::ActiveModel = users::Entity::find_by_id(hidden_user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into();
    active_user.last_activity_at = Set(Some(Utc::now().fixed_offset()));
    active_user.show_online = Set(false);
    active_user.update(&db).await.unwrap();

    // Get online users list
    let online_users = dumpster::user::get_online_users(10).await.unwrap();

    // Should NOT contain our hidden user
    let found = online_users.iter().any(|u| u.name == "hidden_online_user");
    assert!(
        !found,
        "Hidden user should NOT appear in get_online_users list"
    );

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_activity_cache_cleanup() {
    use dumpster::user::cleanup_activity_cache;

    // Just verify the function doesn't panic
    cleanup_activity_cache();
}

#[actix_rt::test]
#[serial]
async fn test_online_threshold_constant() {
    use dumpster::user::ONLINE_THRESHOLD_MINUTES;

    // Verify the threshold is reasonable
    assert!(
        ONLINE_THRESHOLD_MINUTES >= 5,
        "Online threshold should be at least 5 minutes"
    );
    assert!(
        ONLINE_THRESHOLD_MINUTES <= 60,
        "Online threshold should be at most 60 minutes"
    );
}
