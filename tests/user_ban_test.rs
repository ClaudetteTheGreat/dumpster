/// Integration tests for user ban functionality
/// Tests that banned users cannot log in and bans are enforced correctly
mod common;
use serial_test::serial;

use common::{database::*, fixtures::*};
use dumpster::web::login::{login, LoginResultStatus};

#[actix_rt::test]
#[serial]
async fn test_permanently_banned_user_cannot_login() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a permanently banned user
    let user = create_banned_test_user(
        &db,
        "banned_user1",
        "correct_password",
        "Spamming and abuse",
        true, // permanent
        None,
    )
    .await
    .expect("Failed to create banned user");

    // Verify user is banned
    let banned = is_user_banned(&db, user.id)
        .await
        .expect("Failed to check ban status");
    assert!(banned, "User should be banned");

    // Attempt login with correct password
    let result = login("banned_user1", "correct_password", &None::<String>)
        .await
        .expect("Login function failed");

    // Should return Banned status
    assert!(
        matches!(result.result, LoginResultStatus::Banned(_)),
        "Permanently banned user should not be able to log in"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_temporarily_banned_user_cannot_login() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a temporarily banned user (banned for 60 minutes)
    let user = create_banned_test_user(
        &db,
        "banned_user2",
        "correct_password",
        "Violation of rules",
        false,    // not permanent
        Some(60), // 60 minutes
    )
    .await
    .expect("Failed to create banned user");

    // Verify user is banned
    let banned = is_user_banned(&db, user.id)
        .await
        .expect("Failed to check ban status");
    assert!(banned, "User should be banned");

    // Attempt login with correct password
    let result = login("banned_user2", "correct_password", &None::<String>)
        .await
        .expect("Login function failed");

    // Should return Banned status
    assert!(
        matches!(result.result, LoginResultStatus::Banned(_)),
        "Temporarily banned user should not be able to log in while ban is active"
    );

    // Verify ban info is populated
    if let LoginResultStatus::Banned(ban_info) = result.result {
        assert_eq!(ban_info.reason, "Violation of rules");
        assert!(!ban_info.is_permanent);
        assert!(ban_info.expires_at.is_some());
    }

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_expired_ban_allows_login() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a user with an expired ban (ban expired 1 minute ago)
    let user = create_banned_test_user(
        &db,
        "banned_user3",
        "correct_password",
        "Past violation",
        false,
        Some(-1), // expired 1 minute ago
    )
    .await
    .expect("Failed to create user");

    // Verify user is NOT banned (ban has expired)
    let banned = is_user_banned(&db, user.id)
        .await
        .expect("Failed to check ban status");
    assert!(!banned, "Expired ban should not prevent login");

    // Attempt login with correct password
    let result = login("banned_user3", "correct_password", &None::<String>)
        .await
        .expect("Login function failed");

    // Should succeed
    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "User with expired ban should be able to log in"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_banned_user_ban_reason_returned() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let ban_reason = "Multiple account violations - third strike";

    // Create a banned user with specific reason
    let _user = create_banned_test_user(
        &db,
        "banned_user4",
        "correct_password",
        ban_reason,
        true,
        None,
    )
    .await
    .expect("Failed to create banned user");

    // Attempt login
    let result = login("banned_user4", "correct_password", &None::<String>)
        .await
        .expect("Login function failed");

    // Verify ban reason is included in the response
    if let LoginResultStatus::Banned(ban_info) = result.result {
        assert_eq!(ban_info.reason, ban_reason);
        assert!(ban_info.is_permanent);
        assert!(ban_info.expires_at.is_none());
    } else {
        panic!("Expected Banned status");
    }

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_unbanned_user_can_login() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a regular (not banned) user
    let _user = create_test_user(&db, "normal_user", "correct_password")
        .await
        .expect("Failed to create user");

    // Attempt login
    let result = login("normal_user", "correct_password", &None::<String>)
        .await
        .expect("Login function failed");

    // Should succeed
    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Non-banned user should be able to log in"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_banned_user_wrong_password_still_banned() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a banned user
    let _user = create_banned_test_user(
        &db,
        "banned_user5",
        "correct_password",
        "Banned for testing",
        true,
        None,
    )
    .await
    .expect("Failed to create banned user");

    // Attempt login with wrong password
    let result = login("banned_user5", "wrong_password", &None::<String>)
        .await
        .expect("Login function failed");

    // Should still return Banned (ban is checked before password)
    assert!(
        matches!(result.result, LoginResultStatus::Banned(_)),
        "Ban check should happen before password verification"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
