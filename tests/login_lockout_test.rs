/// Integration tests for account lockout functionality
/// Tests the security feature that locks accounts after too many failed login attempts
mod common;
use serial_test::serial;

use common::*;
use ruforo::web::login::{login, LoginResultStatus};

#[actix_rt::test]
#[serial]
async fn test_failed_login_increments_counter() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    // Cleanup any existing test data
    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test user
    let user = create_test_user(&db, "testuser1", "correct_password")
        .await
        .expect("Failed to create test user");

    // Attempt login with wrong password
    let result = login("testuser1", "wrong_password", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(matches!(result.result, LoginResultStatus::BadPassword));

    // Check that failed attempts incremented to 1
    let attempts = get_failed_attempts(&db, user.id)
        .await
        .expect("Failed to get attempts");
    assert_eq!(attempts, 1);

    // Attempt again with wrong password
    let result = login("testuser1", "wrong_password_2", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(matches!(result.result, LoginResultStatus::BadPassword));

    // Check that failed attempts incremented to 2
    let attempts = get_failed_attempts(&db, user.id)
        .await
        .expect("Failed to get attempts");
    assert_eq!(attempts, 2);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_account_locks_after_max_attempts() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser2", "correct_password")
        .await
        .expect("Failed to create test user");

    // Make 4 failed login attempts
    for i in 1..=4 {
        let result = login("testuser2", "wrong_password", &None::<String>)
            .await
            .expect("Login function failed");

        assert!(matches!(result.result, LoginResultStatus::BadPassword));

        let attempts = get_failed_attempts(&db, user.id)
            .await
            .expect("Failed to get attempts");
        assert_eq!(attempts, i);
    }

    // Verify account is NOT locked yet
    let locked = is_user_locked(&db, user.id)
        .await
        .expect("Failed to check lock status");
    assert!(!locked, "Account should not be locked after 4 attempts");

    // Make 5th failed login attempt
    let result = login("testuser2", "wrong_password", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(matches!(result.result, LoginResultStatus::BadPassword));

    // Verify account IS now locked
    let locked = is_user_locked(&db, user.id)
        .await
        .expect("Failed to check lock status");
    assert!(locked, "Account should be locked after 5 attempts");

    let attempts = get_failed_attempts(&db, user.id)
        .await
        .expect("Failed to get attempts");
    assert_eq!(attempts, 5);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_locked_account_rejects_login() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a user that is already locked (15 minutes from now)
    let user = create_locked_test_user(&db, "testuser3", "correct_password", 15)
        .await
        .expect("Failed to create locked user");

    // Verify user is locked
    let locked = is_user_locked(&db, user.id)
        .await
        .expect("Failed to check lock status");
    assert!(locked, "User should be locked");

    // Attempt login with CORRECT password
    let result = login("testuser3", "correct_password", &None::<String>)
        .await
        .expect("Login function failed");

    // Should return AccountLocked, not Success
    assert!(
        matches!(result.result, LoginResultStatus::AccountLocked),
        "Locked account should reject login even with correct password"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_successful_login_resets_counter() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser4", "correct_password")
        .await
        .expect("Failed to create test user");

    // Make 3 failed login attempts
    for _ in 1..=3 {
        login("testuser4", "wrong_password", &None::<String>)
            .await
            .expect("Login function failed");
    }

    // Verify counter is at 3
    let attempts = get_failed_attempts(&db, user.id)
        .await
        .expect("Failed to get attempts");
    assert_eq!(attempts, 3);

    // Login with correct password
    let result = login("testuser4", "correct_password", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Login should succeed with correct password"
    );

    // Verify counter is reset to 0
    let attempts = get_failed_attempts(&db, user.id)
        .await
        .expect("Failed to get attempts");
    assert_eq!(
        attempts, 0,
        "Failed attempts should be reset to 0 on success"
    );

    // Verify locked_until is None
    let locked = is_user_locked(&db, user.id)
        .await
        .expect("Failed to check lock status");
    assert!(!locked, "User should not be locked after successful login");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_expired_lock_allows_login() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a user locked until 1 minute in the PAST (lock already expired)
    let user = create_locked_test_user(&db, "testuser5", "correct_password", -1)
        .await
        .expect("Failed to create locked user");

    // Verify user is NOT locked (lock has expired)
    let locked = is_user_locked(&db, user.id)
        .await
        .expect("Failed to check lock status");
    assert!(!locked, "Expired lock should not prevent login");

    // Attempt login with correct password
    let result = login("testuser5", "correct_password", &None::<String>)
        .await
        .expect("Login function failed");

    // Should succeed
    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Login should succeed when lock has expired"
    );

    // Verify failed_attempts is reset to 0
    let attempts = get_failed_attempts(&db, user.id)
        .await
        .expect("Failed to get attempts");
    assert_eq!(
        attempts, 0,
        "Failed attempts should be reset after expired lock"
    );

    // Verify locked_until is None
    let locked = is_user_locked(&db, user.id)
        .await
        .expect("Failed to check lock status");
    assert!(!locked, "Lock should be cleared after successful login");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_nonexistent_user_returns_bad_name() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Attempt login with non-existent username
    let result = login("nonexistent_user", "any_password", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::BadName),
        "Should return BadName for non-existent user"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
