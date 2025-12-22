//! Test fixtures for creating test data
#![allow(dead_code)]
#![allow(clippy::needless_update)]

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use chrono::Utc;
use sea_orm::{entity::*, ActiveValue::Set, DatabaseConnection, DbErr};

/// Test user fixture
pub struct TestUser {
    pub id: i32,
    pub username: String,
    pub password: String, // Plain text password for testing
}

/// Create a test user with known credentials
pub async fn create_test_user(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
) -> Result<TestUser, DbErr> {
    use ruforo::orm::{user_names, users};

    // Hash the password using Argon2
    // Use the same Argon2 instance that the login function uses
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = ruforo::session::get_argon2()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| DbErr::Custom(format!("Password hashing failed: {}", e)))?
        .to_string();

    // Create user
    // Truncate username for email to avoid exceeding 255 char limit
    let email_username = if username.len() > 240 {
        &username[..240]
    } else {
        username
    };

    let user = users::ActiveModel {
        created_at: Set(Utc::now().naive_utc()),
        password: Set(password_hash),
        password_cipher: Set(users::Cipher::Argon2id),
        failed_login_attempts: Set(0),
        locked_until: Set(None),
        email: Set(Some(format!("{}@test.com", email_username))),
        email_verified: Set(true), // Auto-verify test users
        posts_per_page: Set(25),
        theme: Set("light".to_string()),
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

    Ok(TestUser {
        id: user_model.id,
        username: username.to_string(),
        password: password.to_string(),
    })
}

/// Create a test user with custom email and verification status
pub async fn create_test_user_with_email(
    db: &DatabaseConnection,
    username: &str,
    email: &str,
    email_verified: bool,
) -> Result<ruforo::orm::users::Model, DbErr> {
    use ruforo::orm::{user_names, users};

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = ruforo::session::get_argon2()
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
        email_verified: Set(email_verified),
        posts_per_page: Set(25),
        theme: Set("light".to_string()),
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

/// Create a test user with 2FA enabled
pub async fn create_test_user_with_2fa(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
    totp_secret: &str,
) -> Result<TestUser, DbErr> {
    use ruforo::orm::user_2fa;

    let user = create_test_user(db, username, password).await?;

    // Add 2FA secret
    let user_2fa = user_2fa::ActiveModel {
        user_id: Set(user.id),
        secret: Set(totp_secret.to_string()),
        email_reset: Set(false),
        ..Default::default()
    };
    user_2fa.insert(db).await?;

    Ok(user)
}

/// Create a locked test user (already has failed attempts and is locked)
pub async fn create_locked_test_user(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
    minutes_until_unlock: i64,
) -> Result<TestUser, DbErr> {
    use ruforo::orm::{user_names, users};

    // Hash the password using the same Argon2 instance as login
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = ruforo::session::get_argon2()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| DbErr::Custom(format!("Password hashing failed: {}", e)))?
        .to_string();

    // Create user with lockout set
    let lock_until = Utc::now().naive_utc() + chrono::Duration::minutes(minutes_until_unlock);

    // Truncate username for email to avoid exceeding 255 char limit
    let email_username = if username.len() > 240 {
        &username[..240]
    } else {
        username
    };

    let user = users::ActiveModel {
        created_at: Set(Utc::now().naive_utc()),
        password: Set(password_hash),
        password_cipher: Set(users::Cipher::Argon2id),
        failed_login_attempts: Set(5), // Max attempts reached
        locked_until: Set(Some(lock_until)),
        email: Set(Some(format!("{}@test.com", email_username))),
        email_verified: Set(true), // Auto-verify test users
        posts_per_page: Set(25),
        theme: Set("light".to_string()),
        ..Default::default()
    };
    let user = user.insert(db).await?;

    // Create username
    let user_name = user_names::ActiveModel {
        user_id: Set(user.id),
        name: Set(username.to_string()),
        ..Default::default()
    };
    user_name.insert(db).await?;

    Ok(TestUser {
        id: user.id,
        username: username.to_string(),
        password: password.to_string(),
    })
}

/// Get user's current failed login attempts count
pub async fn get_failed_attempts(db: &DatabaseConnection, user_id: i32) -> Result<i32, DbErr> {
    use ruforo::orm::users;

    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("User not found".to_string()))?;

    Ok(user.failed_login_attempts)
}

/// Check if user account is currently locked
pub async fn is_user_locked(db: &DatabaseConnection, user_id: i32) -> Result<bool, DbErr> {
    use ruforo::orm::users;

    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("User not found".to_string()))?;

    if let Some(locked_until) = user.locked_until {
        Ok(locked_until > Utc::now().naive_utc())
    } else {
        Ok(false)
    }
}

/// Create a banned test user (has an active ban)
pub async fn create_banned_test_user(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
    ban_reason: &str,
    is_permanent: bool,
    minutes_until_unban: Option<i64>,
) -> Result<TestUser, DbErr> {
    use ruforo::orm::user_bans;

    let user = create_test_user(db, username, password).await?;

    // Create ban
    let expires_at = if is_permanent {
        None
    } else {
        Some(Utc::now().naive_utc() + chrono::Duration::minutes(minutes_until_unban.unwrap_or(60)))
    };

    let ban = user_bans::ActiveModel {
        user_id: Set(user.id),
        banned_by: Set(None), // System ban
        reason: Set(ban_reason.to_string()),
        expires_at: Set(expires_at),
        is_permanent: Set(is_permanent),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    ban.insert(db).await?;

    Ok(user)
}

/// Check if user is currently banned
pub async fn is_user_banned(db: &DatabaseConnection, user_id: i32) -> Result<bool, DbErr> {
    use ruforo::orm::user_bans;
    use sea_orm::query::*;

    let now = Utc::now().naive_utc();
    let active_ban = user_bans::Entity::find()
        .filter(user_bans::Column::UserId.eq(user_id))
        .filter(
            user_bans::Column::IsPermanent
                .eq(true)
                .or(user_bans::Column::ExpiresAt.gt(now)),
        )
        .one(db)
        .await?;

    Ok(active_ban.is_some())
}

/// Create a test forum and thread for testing
pub async fn create_test_forum_and_thread(
    db: &DatabaseConnection,
    user_id: i32,
    thread_title: &str,
) -> Result<(ruforo::orm::forums::Model, ruforo::orm::threads::Model), DbErr> {
    use ruforo::orm::{forums, threads};

    // Create a forum
    let forum = forums::ActiveModel {
        label: Set("Test Forum".to_string()),
        description: Set(Some("A test forum".to_string())),
        last_post_id: Set(None),
        last_thread_id: Set(None),
        ..Default::default()
    };
    let forum_model = forum.insert(db).await?;

    // Create a thread
    let thread = threads::ActiveModel {
        forum_id: Set(forum_model.id),
        title: Set(thread_title.to_string()),
        user_id: Set(Some(user_id)),
        post_count: Set(0),
        view_count: Set(0),
        created_at: Set(Utc::now().naive_utc()),
        is_locked: Set(false),
        is_pinned: Set(false),
        ..Default::default()
    };
    let thread_model = thread.insert(db).await?;

    Ok((forum_model, thread_model))
}
