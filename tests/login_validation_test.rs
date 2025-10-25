/// Integration tests for login input validation
/// Tests that malformed input is properly rejected

mod common;
use serial_test::serial;

use common::*;
use ruforo::web::login::{login, LoginResultStatus};

#[actix_rt::test]
#[serial]
async fn test_valid_credentials_accepted() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "validuser", "ValidPass123!")
        .await
        .expect("Failed to create test user");

    // Valid login should succeed
    let result = login("validuser", "ValidPass123!", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Valid credentials should be accepted"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_username_whitespace_trimmed() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create test user");

    // Login with whitespace around username
    let result = login("  testuser  ", "password123", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Whitespace should be trimmed from username"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_very_long_username_handled() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a very long username (within our 255 char limit)
    let long_username = "a".repeat(250);
    let user = create_test_user(&db, &long_username, "password123")
        .await
        .expect("Failed to create test user");

    // Should succeed with long but valid username
    let result = login(&long_username, "password123", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Long valid username should be accepted"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_nonexistent_very_long_username() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Try to login with a very long non-existent username (within limit)
    let long_username = "b".repeat(255);
    let result = login(&long_username, "password123", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::BadName),
        "Should handle long non-existent username gracefully"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_long_password_handled() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user with a long password (within 1000 char limit)
    let long_password = "SecurePass!".repeat(50); // 550 chars
    let user = create_test_user(&db, "testuser", &long_password)
        .await
        .expect("Failed to create test user");

    // Should succeed with long password
    let result = login("testuser", &long_password, &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Long valid password should be accepted"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_unicode_username_supported() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user with unicode characters
    let unicode_username = "用户名test";
    let user = create_test_user(&db, unicode_username, "password123")
        .await
        .expect("Failed to create test user");

    // Should support unicode usernames
    let result = login(unicode_username, "password123", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Unicode usernames should be supported"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_special_characters_in_password() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user with special characters in password
    let special_password = r#"P@ssw0rd!#$%^&*()_+-=[]{}|;':",.<>?/\`~"#;
    let user = create_test_user(&db, "testuser", special_password)
        .await
        .expect("Failed to create test user");

    // Should handle special characters in password
    let result = login("testuser", special_password, &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Special characters in password should be supported"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
