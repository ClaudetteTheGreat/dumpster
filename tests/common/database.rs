//! Test database setup and management
#![allow(dead_code)]

use sea_orm::{Database, DatabaseConnection, DbErr};
use std::env;
use std::sync::Once;

static INIT_SYNC: Once = Once::new();

/// Initialize synchronous global state (SALT, ARGON2, SESSIONS)
fn init_sync_globals() {
    INIT_SYNC.call_once(|| {
        // Set SALT environment variable if not already set
        // Must be a valid base64 string for Argon2
        if env::var("SALT").is_err() {
            env::set_var("SALT", "testsaltfortestingonly1234567890AB");
        }

        // Initialize session module (ARGON2, SALT, SESSIONS, START_TIME)
        ruforo::session::init();
    });
}

/// Initialize async global state (DB_POOL)
/// Must be called from an async context
async fn init_async_globals() {
    // Ensure sync globals are initialized first
    init_sync_globals();

    // Use a static flag to ensure this only runs once
    // We can't use the regular Once::call_once because it's not async-friendly
    use std::sync::atomic::{AtomicBool, Ordering};
    static DB_INITIALIZED: AtomicBool = AtomicBool::new(false);

    if !DB_INITIALIZED.swap(true, Ordering::SeqCst) {
        let database_url = env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5433/ruforo_test".to_string()
        });

        ruforo::db::init_db(database_url).await;
    }
}

/// Get a test database connection
/// Uses TEST_DATABASE_URL environment variable or falls back to default test DB
pub async fn get_test_db() -> Result<DatabaseConnection, DbErr> {
    let database_url = env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        // Default to test database on port 5433
        "postgres://postgres:postgres@localhost:5433/ruforo_test".to_string()
    });

    Database::connect(&database_url).await
}

/// Setup test database - initialize globals and return connection
pub async fn setup_test_database() -> Result<DatabaseConnection, DbErr> {
    // Initialize all global state (both sync and async)
    init_async_globals().await;

    let db = get_test_db().await?;

    // Note: In production tests, you'd want to run migrations here
    // For now, we assume the test database already has migrations applied

    Ok(db)
}

/// Cleanup function to remove test data
///
/// Truncates all tables that might contain test data in the correct order
/// to avoid foreign key constraint violations.
pub async fn cleanup_test_data(db: &DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::*;

    // Clean up tables in reverse dependency order
    // Using CASCADE ensures child records are also removed
    // RESTART IDENTITY resets sequences (id counters) to 1
    //
    // Order matters: child tables (with foreign keys) must be listed before parent tables
    db.execute(Statement::from_string(
        db.get_database_backend(),
        "TRUNCATE TABLE
            chat_messages,
            chat_rooms,
            forum_permissions,
            permission_values,
            permission_collections,
            user_groups,
            user_name_history,
            user_names,
            user_2fa,
            user_avatars,
            sessions,
            posts,
            threads,
            profile_posts,
            ugc_deletions,
            ugc_attachments,
            ugc_revisions,
            attachments,
            attachment_thumbnails,
            password_reset_tokens,
            email_verification_tokens,
            user_bans,
            ip_bans,
            user_badges,
            user_social_links,
            users,
            forums,
            groups,
            permissions,
            permission_categories,
            ip
        RESTART IDENTITY CASCADE;"
            .to_string(),
    ))
    .await?;

    Ok(())
}

/// Create a test transaction that will be rolled back
/// This allows tests to run in isolation without affecting the database
pub async fn get_test_transaction() -> Result<DatabaseConnection, DbErr> {
    let db = get_test_db().await?;

    // Note: For true transaction-based testing, we'd want to start a transaction here
    // and roll it back after each test. SeaORM's current API makes this challenging
    // for integration tests, so we use cleanup_test_data instead.

    Ok(db)
}
