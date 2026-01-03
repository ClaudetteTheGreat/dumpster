/// Integration tests for email verification functionality
/// Tests email verification tokens, verification flow, and email requirements
mod common;
use serial_test::serial;

use chrono::Utc;
use common::{database::*, fixtures::*};
use dumpster::orm::{email_verification_tokens, users};
use sea_orm::{entity::*, query::*, ActiveValue::Set, DatabaseConnection, DbErr};

/// Create a test user with unverified email
async fn create_unverified_user(
    db: &DatabaseConnection,
    username: &str,
    email: &str,
) -> Result<users::Model, DbErr> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use dumpster::orm::user_names;

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = dumpster::session::get_argon2()
        .hash_password("password123".as_bytes(), &salt)
        .map_err(|e| DbErr::Custom(format!("Password hashing failed: {}", e)))?
        .to_string();

    let user = users::ActiveModel {
        created_at: Set(Utc::now().naive_utc()),
        password: Set(password_hash),
        password_cipher: Set(users::Cipher::Argon2id),
        failed_login_attempts: Set(0),
        locked_until: Set(None),
        email: Set(Some(email.to_string())),
        email_verified: Set(false), // Unverified
        post_count: Set(0),
        ..Default::default()
    };
    let user_model = user.insert(db).await?;

    // Create username
    let user_name = user_names::ActiveModel {
        user_id: Set(user_model.id),
        name: Set(username.to_string()),
        ..Default::default()
    };
    user_name.insert(db).await?;

    Ok(user_model)
}

/// Create an email verification token for a user
async fn create_verification_token(
    db: &DatabaseConnection,
    user_id: i32,
    email: &str,
    token: &str,
    expires_minutes: i64,
) -> Result<email_verification_tokens::Model, DbErr> {
    let expires_at = Utc::now().naive_utc() + chrono::Duration::minutes(expires_minutes);

    let token_model = email_verification_tokens::ActiveModel {
        token: Set(token.to_string()),
        user_id: Set(user_id),
        email: Set(email.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        expires_at: Set(expires_at),
        used: Set(false),
        ..Default::default()
    };

    token_model.insert(db).await
}

#[actix_rt::test]
#[serial]
async fn test_create_unverified_user() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_unverified_user(&db, "newuser", "newuser@example.com")
        .await
        .expect("Failed to create unverified user");

    assert!(user.id > 0, "User should have valid ID");
    assert_eq!(
        user.email,
        Some("newuser@example.com".to_string()),
        "Email should be set"
    );
    assert!(!user.email_verified, "Email should not be verified");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_create_verification_token() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_unverified_user(&db, "testuser", "test@example.com")
        .await
        .expect("Failed to create user");

    let token =
        create_verification_token(&db, user.id, "test@example.com", "test_token_12345", 1440)
            .await
            .expect("Failed to create token");

    assert_eq!(token.token, "test_token_12345", "Token should match");
    assert_eq!(token.user_id, user.id, "User ID should match");
    assert!(!token.used, "Token should not be used yet");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_find_token_by_value() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_unverified_user(&db, "testuser", "test@example.com")
        .await
        .expect("Failed to create user");

    let _token =
        create_verification_token(&db, user.id, "test@example.com", "unique_token_abc", 1440)
            .await
            .expect("Failed to create token");

    // Find token by value
    let found_token = email_verification_tokens::Entity::find()
        .filter(email_verification_tokens::Column::Token.eq("unique_token_abc"))
        .one(&db)
        .await
        .expect("Failed to query token");

    assert!(found_token.is_some(), "Token should be found");
    let found_token = found_token.unwrap();
    assert_eq!(
        found_token.user_id, user.id,
        "Token should belong to correct user"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_mark_token_as_used() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_unverified_user(&db, "testuser", "test@example.com")
        .await
        .expect("Failed to create user");

    let token = create_verification_token(&db, user.id, "test@example.com", "test_token_xyz", 1440)
        .await
        .expect("Failed to create token");

    // Mark token as used
    let mut active_token: email_verification_tokens::ActiveModel = token.into();
    active_token.used = Set(true);
    let updated_token = active_token
        .update(&db)
        .await
        .expect("Failed to update token");

    assert!(updated_token.used, "Token should be marked as used");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_verify_user_email() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_unverified_user(&db, "testuser", "test@example.com")
        .await
        .expect("Failed to create user");

    assert!(!user.email_verified, "User should start unverified");

    // Verify the user
    let mut active_user: users::ActiveModel = user.into();
    active_user.email_verified = Set(true);
    let verified_user = active_user
        .update(&db)
        .await
        .expect("Failed to verify user");

    assert!(verified_user.email_verified, "User should now be verified");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_token_expiration() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_unverified_user(&db, "testuser", "test@example.com")
        .await
        .expect("Failed to create user");

    // Create an expired token (expires in the past)
    let expired_token =
        create_verification_token(&db, user.id, "test@example.com", "expired_token", -60)
            .await
            .expect("Failed to create token");

    // Check if token is expired
    let is_expired = expired_token.expires_at < Utc::now().naive_utc();
    assert!(is_expired, "Token should be expired");

    // Create a valid token (expires in the future)
    let valid_token =
        create_verification_token(&db, user.id, "test@example.com", "valid_token", 60)
            .await
            .expect("Failed to create token");

    let is_valid = valid_token.expires_at > Utc::now().naive_utc();
    assert!(is_valid, "Token should be valid");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_multiple_tokens_for_user() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_unverified_user(&db, "testuser", "test@example.com")
        .await
        .expect("Failed to create user");

    // Create multiple tokens for the same user
    let _token1 = create_verification_token(&db, user.id, "test@example.com", "token_1", 1440)
        .await
        .expect("Failed to create token 1");

    let _token2 = create_verification_token(&db, user.id, "test@example.com", "token_2", 1440)
        .await
        .expect("Failed to create token 2");

    // Query all tokens for this user
    let user_tokens = email_verification_tokens::Entity::find()
        .filter(email_verification_tokens::Column::UserId.eq(user.id))
        .all(&db)
        .await
        .expect("Failed to query tokens");

    assert_eq!(user_tokens.len(), 2, "User should have 2 tokens");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_find_unused_valid_token() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_unverified_user(&db, "testuser", "test@example.com")
        .await
        .expect("Failed to create user");

    // Create an expired token
    let _expired =
        create_verification_token(&db, user.id, "test@example.com", "expired_token", -60)
            .await
            .expect("Failed to create expired token");

    // Create a used token
    let used_token =
        create_verification_token(&db, user.id, "test@example.com", "used_token", 1440)
            .await
            .expect("Failed to create used token");

    let mut active_used: email_verification_tokens::ActiveModel = used_token.into();
    active_used.used = Set(true);
    active_used
        .update(&db)
        .await
        .expect("Failed to mark token as used");

    // Create a valid, unused token
    let _valid = create_verification_token(&db, user.id, "test@example.com", "valid_token", 1440)
        .await
        .expect("Failed to create valid token");

    // Find only unused, non-expired tokens
    let now = Utc::now().naive_utc();
    let valid_tokens = email_verification_tokens::Entity::find()
        .filter(email_verification_tokens::Column::UserId.eq(user.id))
        .filter(email_verification_tokens::Column::Used.eq(false))
        .filter(email_verification_tokens::Column::ExpiresAt.gt(now))
        .all(&db)
        .await
        .expect("Failed to query valid tokens");

    assert_eq!(
        valid_tokens.len(),
        1,
        "Should find exactly 1 valid, unused token"
    );
    assert_eq!(
        valid_tokens[0].token, "valid_token",
        "Should find the correct token"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_user_with_verified_email() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Fetch a user from the database to check email_verified flag
    let user_model = create_test_user(&db, "verifieduser", "password123")
        .await
        .expect("Failed to create user");

    let user_from_db = users::Entity::find_by_id(user_model.id)
        .one(&db)
        .await
        .expect("Failed to query user")
        .expect("User should exist");

    assert!(
        user_from_db.email_verified,
        "Test fixture users should be auto-verified"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
