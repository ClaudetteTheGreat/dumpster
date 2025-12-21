/// Integration tests for password reset functionality
/// Tests password reset token creation, validation, expiration, and password update flow
mod common;
use serial_test::serial;

use chrono::Utc;
use common::*;
use ruforo::orm::{password_reset_tokens, users};
use sea_orm::{entity::*, query::*, ActiveValue::Set, DatabaseConnection, DbErr};

/// Create a password reset token for a user
async fn create_reset_token(
    db: &DatabaseConnection,
    user_id: i32,
    token: &str,
    expires_minutes: i64,
) -> Result<password_reset_tokens::Model, DbErr> {
    let expires_at = Utc::now().naive_utc() + chrono::Duration::minutes(expires_minutes);

    let token_model = password_reset_tokens::ActiveModel {
        token: Set(token.to_string()),
        user_id: Set(user_id),
        created_at: Set(Utc::now().naive_utc()),
        expires_at: Set(expires_at),
        used: Set(false),
    };

    token_model.insert(db).await
}

#[actix_rt::test]
#[serial]
async fn test_create_reset_token() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create user");

    let token = create_reset_token(&db, user.id, "reset_token_12345", 60)
        .await
        .expect("Failed to create token");

    assert_eq!(token.token, "reset_token_12345", "Token should match");
    assert_eq!(token.user_id, user.id, "User ID should match");
    assert!(!token.used, "Token should not be used yet");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_find_reset_token_by_value() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create user");

    let _token = create_reset_token(&db, user.id, "unique_reset_token", 60)
        .await
        .expect("Failed to create token");

    // Find token by value
    let found_token = password_reset_tokens::Entity::find_by_id("unique_reset_token".to_string())
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
async fn test_reset_token_expiration() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create user");

    // Create an expired token (expires in the past)
    let expired_token = create_reset_token(&db, user.id, "expired_token", -60)
        .await
        .expect("Failed to create token");

    // Check if token is expired
    let is_expired = expired_token.expires_at < Utc::now().naive_utc();
    assert!(is_expired, "Token should be expired");

    // Create a valid token (expires in the future)
    let valid_token = create_reset_token(&db, user.id, "valid_token", 60)
        .await
        .expect("Failed to create token");

    let is_valid = valid_token.expires_at > Utc::now().naive_utc();
    assert!(is_valid, "Token should be valid");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_mark_reset_token_as_used() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create user");

    let token = create_reset_token(&db, user.id, "test_token_xyz", 60)
        .await
        .expect("Failed to create token");

    assert!(!token.used, "Token should start as unused");

    // Mark token as used
    let mut active_token: password_reset_tokens::ActiveModel = token.into();
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
async fn test_password_update_after_reset() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "oldpassword")
        .await
        .expect("Failed to create user");

    // Get original password hash
    let original_user = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .expect("Failed to query user")
        .expect("User should exist");

    let original_hash = original_user.password.clone();

    // Hash new password
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    let salt = SaltString::generate(&mut OsRng);
    let new_hash = ruforo::session::get_argon2()
        .hash_password("newpassword123".as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();

    // Update user password
    let mut active_user: users::ActiveModel = original_user.into();
    active_user.password = Set(new_hash.clone());
    let updated_user = active_user
        .update(&db)
        .await
        .expect("Failed to update password");

    assert_ne!(
        updated_user.password, original_hash,
        "Password hash should be different after reset"
    );
    assert_eq!(
        updated_user.password, new_hash,
        "Password hash should match new hash"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_find_unused_valid_reset_token() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create user");

    // Create an expired token
    let _expired = create_reset_token(&db, user.id, "expired_reset_token", -60)
        .await
        .expect("Failed to create expired token");

    // Create a used token
    let used_token = create_reset_token(&db, user.id, "used_reset_token", 60)
        .await
        .expect("Failed to create used token");

    let mut active_used: password_reset_tokens::ActiveModel = used_token.into();
    active_used.used = Set(true);
    active_used
        .update(&db)
        .await
        .expect("Failed to mark token as used");

    // Create a valid, unused token
    let _valid = create_reset_token(&db, user.id, "valid_reset_token", 60)
        .await
        .expect("Failed to create valid token");

    // Find only unused, non-expired tokens
    let now = Utc::now().naive_utc();
    let valid_tokens = password_reset_tokens::Entity::find()
        .filter(password_reset_tokens::Column::UserId.eq(user.id))
        .filter(password_reset_tokens::Column::Used.eq(false))
        .filter(password_reset_tokens::Column::ExpiresAt.gt(now))
        .all(&db)
        .await
        .expect("Failed to query valid tokens");

    assert_eq!(
        valid_tokens.len(),
        1,
        "Should find exactly 1 valid, unused token"
    );
    assert_eq!(
        valid_tokens[0].token, "valid_reset_token",
        "Should find the correct token"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_multiple_reset_tokens_for_user() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create user");

    // Create multiple tokens for the same user (simulates multiple reset requests)
    let _token1 = create_reset_token(&db, user.id, "reset_token_1", 60)
        .await
        .expect("Failed to create token 1");

    let _token2 = create_reset_token(&db, user.id, "reset_token_2", 60)
        .await
        .expect("Failed to create token 2");

    let _token3 = create_reset_token(&db, user.id, "reset_token_3", 60)
        .await
        .expect("Failed to create token 3");

    // Query all tokens for this user
    let user_tokens = password_reset_tokens::Entity::find()
        .filter(password_reset_tokens::Column::UserId.eq(user.id))
        .all(&db)
        .await
        .expect("Failed to query tokens");

    assert_eq!(user_tokens.len(), 3, "User should have 3 tokens");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_token_one_hour_expiration() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create user");

    // Create token that expires in 60 minutes (standard 1-hour expiration)
    let token = create_reset_token(&db, user.id, "one_hour_token", 60)
        .await
        .expect("Failed to create token");

    let now = Utc::now().naive_utc();
    let one_hour_from_now = now + chrono::Duration::hours(1);

    // Token should expire approximately 1 hour from now
    // Allow for some drift (within 5 minutes)
    let expires_at = token.expires_at;
    let diff = (expires_at - now).num_minutes();

    assert!(
        diff >= 55 && diff <= 65,
        "Token should expire in approximately 60 minutes, got {} minutes",
        diff
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_user_by_email_lookup() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "emailuser", "test@example.com", true)
        .await
        .expect("Failed to create user");

    // Look up user by email
    let found_user = users::Entity::find()
        .filter(users::Column::Email.eq("test@example.com"))
        .one(&db)
        .await
        .expect("Failed to query user");

    assert!(found_user.is_some(), "User should be found by email");
    assert_eq!(found_user.unwrap().id, user.id, "Should find correct user");

    // Look up non-existent email
    let not_found = users::Entity::find()
        .filter(users::Column::Email.eq("nonexistent@example.com"))
        .one(&db)
        .await
        .expect("Failed to query user");

    assert!(not_found.is_none(), "Should not find non-existent email");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_used_token_cannot_be_reused() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create user");

    // Create and use a token
    let token = create_reset_token(&db, user.id, "single_use_token", 60)
        .await
        .expect("Failed to create token");

    let mut active_token: password_reset_tokens::ActiveModel = token.into();
    active_token.used = Set(true);
    active_token
        .update(&db)
        .await
        .expect("Failed to mark as used");

    // Try to find unused tokens
    let now = Utc::now().naive_utc();
    let valid_tokens = password_reset_tokens::Entity::find()
        .filter(password_reset_tokens::Column::Token.eq("single_use_token"))
        .filter(password_reset_tokens::Column::Used.eq(false))
        .filter(password_reset_tokens::Column::ExpiresAt.gt(now))
        .all(&db)
        .await
        .expect("Failed to query tokens");

    assert_eq!(
        valid_tokens.len(),
        0,
        "Used token should not be returned as valid"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
