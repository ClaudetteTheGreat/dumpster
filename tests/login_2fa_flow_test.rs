/// Integration tests for two-step 2FA login flow
/// Tests that users with 2FA enabled go through proper authentication flow
mod common;
use serial_test::serial;

use common::*;
use ruforo::web::login::{login, LoginResultStatus};

#[actix_rt::test]
#[serial]
async fn test_2fa_user_without_code_returns_missing_2fa() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user with 2FA enabled
    let user = create_test_user_with_2fa(&db, "2fauser", "password123", "JBSWY3DPEHPK3PXP")
        .await
        .expect("Failed to create 2FA user");

    // Login without TOTP code should return Missing2FA
    let result = login("2fauser", "password123", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Missing2FA),
        "Should return Missing2FA when user has 2FA but no code provided"
    );

    // Should still return the user_id for session creation later
    assert!(
        result.user_id.is_some(),
        "Should include user_id for pending auth"
    );
    assert_eq!(result.user_id.unwrap(), user.id);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_2fa_user_with_wrong_code_returns_bad_2fa() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user with 2FA enabled
    let user = create_test_user_with_2fa(&db, "2fauser2", "password123", "JBSWY3DPEHPK3PXP")
        .await
        .expect("Failed to create 2FA user");

    // Login with wrong TOTP code
    let wrong_code = "000000".to_string();
    let result = login("2fauser2", "password123", &Some(wrong_code))
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Bad2FA),
        "Should return Bad2FA with incorrect TOTP code"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_2fa_user_with_valid_code_succeeds() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user with 2FA enabled
    let secret = "JBSWY3DPEHPK3PXP";
    let user = create_test_user_with_2fa(&db, "2fauser3", "password123", secret)
        .await
        .expect("Failed to create 2FA user");

    // Generate a valid TOTP code
    use google_authenticator::GoogleAuthenticator;
    let auth = GoogleAuthenticator::new();
    let valid_code = auth.get_code(secret, 0).expect("Failed to generate TOTP");

    // Login with correct TOTP code
    let result = login("2fauser3", "password123", &Some(valid_code))
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Should succeed with valid TOTP code, got: {:?}",
        result.result
    );

    assert_eq!(result.user_id.unwrap(), user.id);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_2fa_flow_resets_failed_attempts_on_success() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user with 2FA enabled
    let secret = "JBSWY3DPEHPK3PXP";
    let user = create_test_user_with_2fa(&db, "2fauser4", "password123", secret)
        .await
        .expect("Failed to create 2FA user");

    // Make a few failed login attempts (wrong password)
    for _ in 0..3 {
        let _ = login("2fauser4", "wrongpassword", &None::<String>).await;
    }

    // Verify failed attempts were recorded
    let attempts = get_failed_attempts(&db, user.id)
        .await
        .expect("Failed to get attempts");
    assert_eq!(attempts, 3);

    // Now login with correct credentials and 2FA
    use google_authenticator::GoogleAuthenticator;
    let auth = GoogleAuthenticator::new();
    let valid_code = auth.get_code(secret, 0).expect("Failed to generate TOTP");

    let result = login("2fauser4", "password123", &Some(valid_code))
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Should succeed with valid credentials and TOTP"
    );

    // Verify failed attempts were reset
    let attempts = get_failed_attempts(&db, user.id)
        .await
        .expect("Failed to get attempts");
    assert_eq!(
        attempts, 0,
        "Failed attempts should be reset after successful 2FA login"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_2fa_wrong_password_increments_failed_attempts() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user with 2FA enabled
    let user = create_test_user_with_2fa(&db, "2fauser5", "password123", "JBSWY3DPEHPK3PXP")
        .await
        .expect("Failed to create 2FA user");

    // Attempt login with wrong password
    let result = login("2fauser5", "wrongpassword", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::BadPassword),
        "Should return BadPassword for wrong password"
    );

    // Check that failed attempts incremented
    let attempts = get_failed_attempts(&db, user.id)
        .await
        .expect("Failed to get attempts");
    assert_eq!(
        attempts, 1,
        "Failed attempts should increment even for 2FA users"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
