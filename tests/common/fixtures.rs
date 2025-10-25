/// Test fixtures for creating test data
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
    let user = users::ActiveModel {
        created_at: Set(Utc::now().naive_utc()),
        password: Set(password_hash),
        password_cipher: Set(users::Cipher::Argon2id),
        failed_login_attempts: Set(0),
        locked_until: Set(None),
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
    let user = users::ActiveModel {
        created_at: Set(Utc::now().naive_utc()),
        password: Set(password_hash),
        password_cipher: Set(users::Cipher::Argon2id),
        failed_login_attempts: Set(5), // Max attempts reached
        locked_until: Set(Some(lock_until)),
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
