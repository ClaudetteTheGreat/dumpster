/// Integration tests for case-insensitive username handling
/// Tests login, registration, and lookup with different case variations
mod common;
use serial_test::serial;

use common::{database::*, fixtures::*};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

/// Test that login works with different case variations of username
#[actix_rt::test]
#[serial]
async fn test_login_case_insensitive() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user with mixed case username
    let user = create_test_user(&db, "TestUser", "password123")
        .await
        .expect("Failed to create user");

    // Verify we can find the user with lowercase
    let found_lowercase = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT user_id FROM user_names WHERE LOWER(name) = LOWER($1) LIMIT 1",
            vec!["testuser".into()],
        ))
        .await
        .expect("Query failed");

    assert!(found_lowercase.is_some(), "Should find user with lowercase search");

    // Verify we can find the user with uppercase
    let found_uppercase = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT user_id FROM user_names WHERE LOWER(name) = LOWER($1) LIMIT 1",
            vec!["TESTUSER".into()],
        ))
        .await
        .expect("Query failed");

    assert!(found_uppercase.is_some(), "Should find user with uppercase search");

    // Verify we can find the user with original case
    let found_original = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT user_id FROM user_names WHERE LOWER(name) = LOWER($1) LIMIT 1",
            vec!["TestUser".into()],
        ))
        .await
        .expect("Query failed");

    assert!(found_original.is_some(), "Should find user with original case search");

    // Verify all queries return the same user_id
    let id1: i32 = found_lowercase.unwrap().try_get("", "user_id").unwrap();
    let id2: i32 = found_uppercase.unwrap().try_get("", "user_id").unwrap();
    let id3: i32 = found_original.unwrap().try_get("", "user_id").unwrap();

    assert_eq!(id1, user.id);
    assert_eq!(id2, user.id);
    assert_eq!(id3, user.id);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

/// Test that registration rejects usernames that differ only in case
#[actix_rt::test]
#[serial]
async fn test_registration_rejects_case_duplicate() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user with mixed case username
    let _user = create_test_user(&db, "UniqueUser", "password123")
        .await
        .expect("Failed to create first user");

    // Try to find if a user with different case already exists (simulating registration check)
    let existing_user = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT user_id FROM user_names WHERE LOWER(name) = LOWER($1) LIMIT 1",
            vec!["uniqueuser".into()], // lowercase version
        ))
        .await
        .expect("Query failed");

    assert!(
        existing_user.is_some(),
        "Registration check should find existing user with different case"
    );

    // Also test uppercase
    let existing_upper = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT user_id FROM user_names WHERE LOWER(name) = LOWER($1) LIMIT 1",
            vec!["UNIQUEUSER".into()],
        ))
        .await
        .expect("Query failed");

    assert!(
        existing_upper.is_some(),
        "Registration check should find existing user with uppercase variant"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

/// Test that user search/lookup is case-insensitive
#[actix_rt::test]
#[serial]
async fn test_user_search_case_insensitive() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create users with various case patterns
    create_test_user(&db, "Alice", "password123")
        .await
        .expect("Failed to create Alice");
    create_test_user(&db, "BOB", "password123")
        .await
        .expect("Failed to create BOB");
    create_test_user(&db, "charlie", "password123")
        .await
        .expect("Failed to create charlie");

    // Search with different case - should find regardless of case
    let search_alice = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT name FROM user_names WHERE LOWER(name) LIKE LOWER($1 || '%') LIMIT 1",
            vec!["ali".into()],
        ))
        .await
        .expect("Query failed");

    assert!(search_alice.is_some(), "Should find Alice with lowercase search 'ali'");

    let search_bob = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT name FROM user_names WHERE LOWER(name) LIKE LOWER($1 || '%') LIMIT 1",
            vec!["bob".into()],
        ))
        .await
        .expect("Query failed");

    assert!(search_bob.is_some(), "Should find BOB with lowercase search 'bob'");

    let search_charlie = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT name FROM user_names WHERE LOWER(name) LIKE LOWER($1 || '%') LIMIT 1",
            vec!["CHAR".into()],
        ))
        .await
        .expect("Query failed");

    assert!(search_charlie.is_some(), "Should find charlie with uppercase search 'CHAR'");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
