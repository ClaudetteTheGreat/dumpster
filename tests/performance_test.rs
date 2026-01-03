/// Performance tests for thread page optimizations
/// These tests verify that batch loading and caching work correctly.
mod common;
use serial_test::serial;

use common::{database::*, fixtures::*};
use dumpster::user::Profile;
use dumpster::orm::users;
use sea_orm::{entity::*, ActiveValue::Set};

#[actix_rt::test]
#[serial]
async fn test_batch_user_profile_loading() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create multiple users
    let mut user_ids = Vec::new();
    for i in 0..5 {
        let user = create_test_user(&db, &format!("batchuser{}", i), "password123")
            .await
            .expect("Failed to create user");
        user_ids.push(user.id);
    }

    // Batch load users
    let profiles = Profile::get_by_ids(&db, &user_ids)
        .await
        .expect("Failed to batch load profiles");

    // Verify all users were loaded
    assert_eq!(profiles.len(), 5, "Should load all 5 users");

    // Verify each user is present and correct
    for (i, &user_id) in user_ids.iter().enumerate() {
        let profile = profiles.get(&user_id);
        assert!(profile.is_some(), "User {} should be loaded", user_id);
        let profile = profile.unwrap();
        assert_eq!(profile.name, format!("batchuser{}", i), "Username should match");
    }

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_batch_user_loading_empty() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    // Batch load with empty list
    let profiles = Profile::get_by_ids(&db, &[])
        .await
        .expect("Failed to batch load empty list");

    assert!(profiles.is_empty(), "Empty list should return empty map");
}

#[actix_rt::test]
#[serial]
async fn test_batch_user_loading_with_missing_ids() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create one user
    let user = create_test_user(&db, "realuser", "password123")
        .await
        .expect("Failed to create user");

    // Try to load with both real and fake IDs
    let ids = vec![user.id, 99999, 88888];
    let profiles = Profile::get_by_ids(&db, &ids)
        .await
        .expect("Failed to batch load profiles");

    // Should only have the one real user
    assert_eq!(profiles.len(), 1, "Should only load existing users");
    assert!(profiles.contains_key(&user.id), "Real user should be loaded");
    assert!(!profiles.contains_key(&99999), "Fake user should not exist");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_post_count_in_batch_loaded_profiles() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user with known post count
    let user = users::ActiveModel {
        created_at: Set(chrono::Utc::now().naive_utc()),
        password: Set("$argon2id$v=19$m=16,t=2,p=1$dGVzdHNhbHQ$test".to_string()),
        password_cipher: Set(users::Cipher::Argon2id),
        failed_login_attempts: Set(0),
        post_count: Set(42), // Known post count
        email_verified: Set(true),
        ..Default::default()
    };
    let user_model = user.insert(&db).await.expect("Failed to create user");

    // Create username
    use dumpster::orm::user_names;
    let user_name = user_names::ActiveModel {
        user_id: Set(user_model.id),
        name: Set("postcounter".to_string()),
        ..Default::default()
    };
    user_name.insert(&db).await.expect("Failed to create username");

    // Batch load
    let profiles = Profile::get_by_ids(&db, &[user_model.id])
        .await
        .expect("Failed to batch load profiles");

    let profile = profiles.get(&user_model.id).expect("User should be loaded");
    assert_eq!(profile.post_count, 42, "Post count should be correct");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
