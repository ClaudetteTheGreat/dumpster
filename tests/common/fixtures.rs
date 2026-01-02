//! Test fixtures for creating test data
#![allow(dead_code)]
#![allow(clippy::needless_update)]

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use chrono::Utc;
use sea_orm::{entity::*, ActiveValue::Set, ConnectionTrait, DatabaseConnection, DbErr};

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
        theme: Set(Some("light".to_string())),
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
        theme: Set(Some("light".to_string())),
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
        theme: Set(Some("light".to_string())),
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

/// Create an IP ban
/// Note: Uses raw SQL due to PostgreSQL INET type requirements
pub async fn create_ip_ban(
    db: &DatabaseConnection,
    ip_address: &str,
    ban_reason: &str,
    is_permanent: bool,
    minutes_until_unban: Option<i64>,
    is_range_ban: bool,
) -> Result<ruforo::orm::ip_bans::Model, DbErr> {
    use ruforo::orm::ip_bans;
    use sea_orm::Statement;

    // Calculate expiration
    let expires_at = if is_permanent {
        None
    } else {
        Some(Utc::now().naive_utc() + chrono::Duration::minutes(minutes_until_unban.unwrap_or(60)))
    };

    let now = Utc::now().naive_utc();

    // Build the SQL with proper casting for INET type
    let (expires_sql, expires_param) = if let Some(exp) = expires_at {
        (
            "$4::TIMESTAMP",
            format!("{}", exp.format("%Y-%m-%d %H:%M:%S")),
        )
    } else {
        ("NULL", String::new())
    };

    let insert_sql = format!(
        r#"
        INSERT INTO ip_bans (ip_address, banned_by, reason, expires_at, is_permanent, is_range_ban, created_at)
        VALUES ($1::INET, NULL, $2, {}, $3, $5, $6::TIMESTAMP)
        RETURNING id, ip_address::TEXT, banned_by, reason, expires_at, created_at, is_permanent, is_range_ban
        "#,
        expires_sql
    );

    // Use raw query to handle INET type properly
    let result = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            &insert_sql,
            vec![
                ip_address.into(),
                ban_reason.into(),
                is_permanent.into(),
                expires_param.into(),
                is_range_ban.into(),
                format!("{}", now.format("%Y-%m-%d %H:%M:%S")).into(),
            ],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("Failed to insert IP ban".to_string()))?;

    // Parse the returned row into our model
    let id: i32 = result.try_get("", "id")?;
    let ip_addr: String = result.try_get("", "ip_address")?;
    let reason: String = result.try_get("", "reason")?;
    let expires: Option<chrono::NaiveDateTime> = result.try_get("", "expires_at")?;
    let created: chrono::NaiveDateTime = result.try_get("", "created_at")?;
    let permanent: bool = result.try_get("", "is_permanent")?;
    let range: bool = result.try_get("", "is_range_ban")?;

    Ok(ip_bans::Model {
        id,
        ip_address: ip_addr,
        banned_by: None,
        reason,
        expires_at: expires,
        created_at: created,
        is_permanent: permanent,
        is_range_ban: range,
    })
}

/// Check if an IP address is currently banned
/// Uses raw SQL due to PostgreSQL INET type requirements
pub async fn is_ip_banned(db: &DatabaseConnection, ip_address: &str) -> Result<bool, DbErr> {
    use sea_orm::Statement;

    let now = Utc::now().naive_utc();
    let now_str = format!("{}", now.format("%Y-%m-%d %H:%M:%S"));

    let sql = r#"
        SELECT COUNT(*) as count FROM ip_bans
        WHERE ip_address = $1::INET
        AND (is_permanent = true OR expires_at > $2::TIMESTAMP)
    "#;

    let result = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            sql,
            vec![ip_address.into(), now_str.into()],
        ))
        .await?;

    if let Some(row) = result {
        let count: i64 = row.try_get("", "count")?;
        Ok(count > 0)
    } else {
        Ok(false)
    }
}

/// Create a word filter
pub async fn create_word_filter(
    db: &DatabaseConnection,
    pattern: &str,
    replacement: Option<&str>,
    action: &str,
    is_regex: bool,
    is_case_sensitive: bool,
    is_whole_word: bool,
) -> Result<ruforo::orm::word_filters::Model, DbErr> {
    use ruforo::orm::word_filters::{self, FilterAction};

    let action_enum = match action {
        "block" => FilterAction::Block,
        "flag" => FilterAction::Flag,
        _ => FilterAction::Replace,
    };

    let filter = word_filters::ActiveModel {
        pattern: Set(pattern.to_string()),
        replacement: Set(replacement.map(|s| s.to_string())),
        is_regex: Set(is_regex),
        is_case_sensitive: Set(is_case_sensitive),
        is_whole_word: Set(is_whole_word),
        action: Set(action_enum),
        is_enabled: Set(true),
        created_by: Set(None),
        created_at: Set(Utc::now().naive_utc()),
        notes: Set(None),
        ..Default::default()
    };
    filter.insert(db).await
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

/// Create a test post in a thread
pub async fn create_test_post(
    db: &DatabaseConnection,
    thread_id: i32,
    user_id: i32,
    content: &str,
    position: i32,
) -> Result<ruforo::orm::posts::Model, DbErr> {
    use ruforo::orm::{posts, ugc, ugc_revisions};

    // Create UGC entry
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(db).await?;

    // Create UGC revision with content
    let revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(Some(user_id)),
        ip_id: Set(None),
        created_at: Set(Utc::now().naive_utc()),
        content: Set(content.to_string()),
        ..Default::default()
    };
    let revision_model = revision.insert(db).await?;

    // Update UGC to point to the revision
    let mut ugc_update: ugc::ActiveModel = ugc_model.into();
    ugc_update.ugc_revision_id = Set(Some(revision_model.id));
    let ugc_model = ugc_update.update(db).await?;

    // Create the post
    let post = posts::ActiveModel {
        thread_id: Set(thread_id),
        position: Set(position),
        ugc_id: Set(ugc_model.id),
        user_id: Set(Some(user_id)),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    post.insert(db).await
}

/// Create a test chat room
pub async fn create_test_chat_room(
    db: &DatabaseConnection,
    title: &str,
) -> Result<ruforo::orm::chat_rooms::Model, DbErr> {
    use ruforo::orm::chat_rooms;

    let room = chat_rooms::ActiveModel {
        title: Set(title.to_string()),
        description: Set(None),
        display_order: Set(0),
        min_posts_required: Set(0),
        min_account_age_hours: Set(0),
        is_staff_only: Set(false),
        ..Default::default()
    };
    room.insert(db).await
}

/// Create a test chat message
pub async fn create_test_chat_message(
    db: &DatabaseConnection,
    room_id: i32,
    user_id: i32,
    message: &str,
) -> Result<ruforo::orm::chat_messages::Model, DbErr> {
    use ruforo::orm::{chat_messages, ugc, ugc_revisions};

    // Create UGC entry
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(db).await?;

    // Create UGC revision with content
    let revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(Some(user_id)),
        ip_id: Set(None),
        created_at: Set(Utc::now().naive_utc()),
        content: Set(message.to_string()),
        ..Default::default()
    };
    let revision_model = revision.insert(db).await?;

    // Update UGC to point to the revision
    let mut ugc_update: ugc::ActiveModel = ugc_model.into();
    ugc_update.ugc_revision_id = Set(Some(revision_model.id));
    let ugc_model = ugc_update.update(db).await?;

    // Create the chat message
    let chat_msg = chat_messages::ActiveModel {
        chat_room_id: Set(room_id),
        ugc_id: Set(ugc_model.id),
        user_id: Set(Some(user_id)),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    chat_msg.insert(db).await
}

/// Update a site setting for testing
pub async fn set_test_setting(
    db: &DatabaseConnection,
    key: &str,
    value: &str,
) -> Result<(), DbErr> {
    use sea_orm::Statement;
    db.execute(Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "UPDATE settings SET value = $1 WHERE key = $2",
        vec![value.into(), key.into()],
    ))
    .await?;
    Ok(())
}

/// Get a user's default_chat_room preference
pub async fn get_user_default_chat_room(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<Option<i32>, DbErr> {
    use ruforo::orm::users;
    use sea_orm::EntityTrait;

    let user = users::Entity::find_by_id(user_id).one(db).await?;
    Ok(user.and_then(|u| u.default_chat_room))
}

/// Set a user's default_chat_room preference
pub async fn set_user_default_chat_room(
    db: &DatabaseConnection,
    user_id: i32,
    room_id: Option<i32>,
) -> Result<(), DbErr> {
    use ruforo::orm::users;
    use sea_orm::EntityTrait;

    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("User not found".to_string()))?;

    let mut user: users::ActiveModel = user.into();
    user.default_chat_room = Set(room_id);
    user.update(db).await?;
    Ok(())
}
