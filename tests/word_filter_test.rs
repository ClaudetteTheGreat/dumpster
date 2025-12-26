//! Word filter integration tests

mod common;

use common::database::setup_test_database;
use common::fixtures::create_word_filter;
use ruforo::word_filter::{apply_filters, reload_filters};
use serial_test::serial;

/// Helper to clean up word filters between tests
async fn cleanup_filters(db: &sea_orm::DatabaseConnection) {
    use ruforo::orm::word_filters;
    use sea_orm::EntityTrait;

    let _ = word_filters::Entity::delete_many().exec(db).await;
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_replace_basic() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Create a replacement filter: Solana -> Salona
    create_word_filter(&db, "Solana", Some("Salona"), "replace", false, false, true)
        .await
        .expect("Failed to create filter");

    // Reload filters from database
    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Test replacement
    let result = apply_filters("I love Solana cryptocurrency");
    assert!(!result.blocked, "Content should not be blocked");
    assert!(!result.flagged, "Content should not be flagged");
    assert_eq!(
        result.content, "I love Salona cryptocurrency",
        "Solana should be replaced with Salona"
    );
    assert!(
        result.matched_patterns.contains(&"Solana".to_string()),
        "Pattern should be in matched patterns"
    );

    cleanup_filters(&db).await;
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_replace_case_insensitive() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Create a case-insensitive replacement filter
    create_word_filter(&db, "Solana", Some("Salona"), "replace", false, false, true)
        .await
        .expect("Failed to create filter");

    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Test lowercase match
    let result = apply_filters("Check out solana today");
    assert_eq!(
        result.content, "Check out Salona today",
        "Lowercase 'solana' should be replaced with titlecase 'Salona'"
    );

    // Test uppercase match
    let result = apply_filters("SOLANA IS GREAT");
    assert_eq!(
        result.content, "SALONA IS GREAT",
        "Uppercase 'SOLANA' should be replaced with uppercase 'SALONA'"
    );

    cleanup_filters(&db).await;
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_replace_case_sensitive() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Create a case-sensitive replacement filter
    create_word_filter(&db, "Solana", Some("Salona"), "replace", false, true, true)
        .await
        .expect("Failed to create filter");

    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Exact case should match
    let result = apply_filters("I love Solana");
    assert_eq!(result.content, "I love Salona", "Exact case should match");

    // Different case should NOT match
    let result = apply_filters("I love solana");
    assert_eq!(
        result.content, "I love solana",
        "Different case should not match"
    );

    cleanup_filters(&db).await;
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_whole_word() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Create a whole-word filter
    create_word_filter(&db, "cat", Some("dog"), "replace", false, false, true)
        .await
        .expect("Failed to create filter");

    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Whole word should match
    let result = apply_filters("The cat sat on the mat");
    assert_eq!(
        result.content, "The dog sat on the mat",
        "Whole word 'cat' should match"
    );

    // Word within another word should NOT match
    let result = apply_filters("category and scatter");
    assert_eq!(
        result.content, "category and scatter",
        "cat within other words should not match"
    );

    cleanup_filters(&db).await;
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_partial_match() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Create a partial-match filter (is_whole_word = false)
    create_word_filter(&db, "bad", Some("***"), "replace", false, false, false)
        .await
        .expect("Failed to create filter");

    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Should match even within words
    let result = apply_filters("This is badword and verybad");
    assert_eq!(
        result.content, "This is ***word and very***",
        "Partial matches should be replaced"
    );

    cleanup_filters(&db).await;
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_block() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Create a blocking filter
    create_word_filter(&db, "spam", None, "block", false, false, true)
        .await
        .expect("Failed to create filter");

    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Content with blocked word should be blocked
    let result = apply_filters("Buy cheap spam pills");
    assert!(result.blocked, "Content with blocked word should be blocked");
    assert!(
        result.block_reason.is_some(),
        "Block reason should be provided"
    );
    assert!(
        result.matched_patterns.contains(&"spam".to_string()),
        "Pattern should be in matched patterns"
    );

    // Content without blocked word should pass
    let result = apply_filters("Hello world");
    assert!(!result.blocked, "Clean content should not be blocked");

    cleanup_filters(&db).await;
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_flag() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Create a flagging filter
    create_word_filter(&db, "suspicious", None, "flag", false, false, true)
        .await
        .expect("Failed to create filter");

    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Content with flagged word should be flagged but not blocked
    let result = apply_filters("This is suspicious content");
    assert!(!result.blocked, "Content should not be blocked");
    assert!(result.flagged, "Content should be flagged for review");
    assert!(
        result.matched_patterns.contains(&"suspicious".to_string()),
        "Pattern should be in matched patterns"
    );

    cleanup_filters(&db).await;
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_regex() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Create a regex filter for email addresses
    create_word_filter(
        &db,
        r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b",
        Some("[email]"),
        "replace",
        true,
        false,
        false,
    )
    .await
    .expect("Failed to create filter");

    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Email addresses should be replaced
    let result = apply_filters("Contact me at test@example.com for info");
    assert_eq!(
        result.content, "Contact me at [email] for info",
        "Email should be replaced"
    );

    cleanup_filters(&db).await;
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_multiple_matches() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Create a replacement filter
    create_word_filter(&db, "foo", Some("bar"), "replace", false, false, true)
        .await
        .expect("Failed to create filter");

    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Multiple occurrences should all be replaced
    let result = apply_filters("foo and foo and foo");
    assert_eq!(
        result.content, "bar and bar and bar",
        "All occurrences should be replaced"
    );

    cleanup_filters(&db).await;
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_priority_block_over_replace() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Create both replace and block filters
    create_word_filter(&db, "bad", Some("good"), "replace", false, false, true)
        .await
        .expect("Failed to create replace filter");
    create_word_filter(&db, "worse", None, "block", false, false, true)
        .await
        .expect("Failed to create block filter");

    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Content with blocked word should be blocked (not just replaced)
    let result = apply_filters("This is bad but worse is here");
    assert!(result.blocked, "Block should take priority");

    // Content with only replace word should be replaced
    let result = apply_filters("This is bad but not terrible");
    assert!(!result.blocked, "Should not be blocked");
    assert_eq!(
        result.content, "This is good but not terrible",
        "Should be replaced"
    );

    cleanup_filters(&db).await;
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_no_filters() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Reload with no filters
    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Content should pass through unchanged
    let result = apply_filters("Any content is fine");
    assert!(!result.blocked, "Should not be blocked");
    assert!(!result.flagged, "Should not be flagged");
    assert_eq!(result.content, "Any content is fine", "Should be unchanged");
    assert!(
        result.matched_patterns.is_empty(),
        "No patterns should match"
    );
}

#[actix_rt::test]
#[serial]
async fn test_word_filter_empty_content() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");
    cleanup_filters(&db).await;

    // Create a filter
    create_word_filter(&db, "test", Some("replaced"), "replace", false, false, true)
        .await
        .expect("Failed to create filter");

    reload_filters(&db)
        .await
        .expect("Failed to reload filters");

    // Empty content should pass
    let result = apply_filters("");
    assert!(!result.blocked, "Empty content should not be blocked");
    assert_eq!(result.content, "", "Empty content should remain empty");
}
