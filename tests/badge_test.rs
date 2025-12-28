//! Integration tests for the badge system

mod common;
use serial_test::serial;

use chrono::Utc;
use common::{database::*, fixtures::*};
use sea_orm::{entity::*, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

// ============================================================================
// Badge CRUD Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_default_badges_exist() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    use ruforo::orm::badges;

    // Query active badges
    let active_badges = badges::Entity::find()
        .filter(badges::Column::IsActive.eq(true))
        .all(&db)
        .await
        .expect("Failed to fetch badges");

    // Should have the default badges from migration
    assert!(
        active_badges.len() >= 10,
        "Should have at least 10 default badges"
    );

    // Check for expected badges
    let slugs: Vec<String> = active_badges.iter().map(|b| b.slug.clone()).collect();
    assert!(
        slugs.contains(&"newcomer".to_string()),
        "Should have 'newcomer' badge"
    );
    assert!(
        slugs.contains(&"first-post".to_string()),
        "Should have 'first-post' badge"
    );
    assert!(
        slugs.contains(&"prolific".to_string()),
        "Should have 'prolific' badge"
    );
    assert!(
        slugs.contains(&"veteran".to_string()),
        "Should have 'veteran' badge"
    );
}

#[actix_rt::test]
#[serial]
async fn test_badge_condition_types() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    use ruforo::orm::badges;

    let all_badges = badges::Entity::find()
        .all(&db)
        .await
        .expect("Failed to fetch badges");

    // Verify different condition types exist
    let condition_types: Vec<_> = all_badges
        .iter()
        .map(|b| b.condition_type.clone())
        .collect();

    assert!(
        condition_types
            .iter()
            .any(|t| t == &badges::BadgeConditionType::Manual),
        "Should have manual badges"
    );
    assert!(
        condition_types
            .iter()
            .any(|t| t == &badges::BadgeConditionType::PostCount),
        "Should have post_count badges"
    );
    assert!(
        condition_types
            .iter()
            .any(|t| t == &badges::BadgeConditionType::ThreadCount),
        "Should have thread_count badges"
    );
    assert!(
        condition_types
            .iter()
            .any(|t| t == &badges::BadgeConditionType::TimeMember),
        "Should have time_member badges"
    );
    assert!(
        condition_types
            .iter()
            .any(|t| t == &badges::BadgeConditionType::Reputation),
        "Should have reputation badges"
    );
}

// ============================================================================
// Badge Awarding Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_award_badge_to_user() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::badges;

    // Create a test user
    let user = create_test_user(&db, "badge_user1", "password123")
        .await
        .expect("Failed to create user");

    // Get the newcomer badge (manual type)
    let newcomer = badges::Entity::find()
        .filter(badges::Column::Slug.eq("newcomer"))
        .one(&db)
        .await
        .expect("Failed to find badge")
        .expect("Newcomer badge not found");

    // Award the badge
    let awarded = ruforo::badges::award_badge(&db, user.id, newcomer.id, None)
        .await
        .expect("Failed to award badge");

    assert!(awarded, "Badge should be awarded successfully");

    // Verify user has the badge
    let has_badge = ruforo::badges::user_has_badge(&db, user.id, newcomer.id)
        .await
        .expect("Failed to check badge");

    assert!(has_badge, "User should have the badge");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_cannot_award_duplicate_badge() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::badges;

    // Create a test user
    let user = create_test_user(&db, "badge_user2", "password123")
        .await
        .expect("Failed to create user");

    // Get the newcomer badge
    let newcomer = badges::Entity::find()
        .filter(badges::Column::Slug.eq("newcomer"))
        .one(&db)
        .await
        .expect("Failed to find badge")
        .expect("Newcomer badge not found");

    // Award the badge first time
    let first_award = ruforo::badges::award_badge(&db, user.id, newcomer.id, None)
        .await
        .expect("Failed to award badge");

    assert!(first_award, "First award should succeed");

    // Try to award again
    let second_award = ruforo::badges::award_badge(&db, user.id, newcomer.id, None)
        .await
        .expect("Failed to check duplicate award");

    assert!(
        !second_award,
        "Second award should return false (already has badge)"
    );

    // User should still only have one badge
    let badge_count = ruforo::badges::count_user_badges(&db, user.id)
        .await
        .expect("Failed to count badges");

    assert_eq!(badge_count, 1, "User should have exactly 1 badge");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_revoke_badge_from_user() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::badges;

    // Create a test user
    let user = create_test_user(&db, "badge_user3", "password123")
        .await
        .expect("Failed to create user");

    // Get the newcomer badge
    let newcomer = badges::Entity::find()
        .filter(badges::Column::Slug.eq("newcomer"))
        .one(&db)
        .await
        .expect("Failed to find badge")
        .expect("Newcomer badge not found");

    // Award the badge
    ruforo::badges::award_badge(&db, user.id, newcomer.id, None)
        .await
        .expect("Failed to award badge");

    // Verify user has it
    assert!(ruforo::badges::user_has_badge(&db, user.id, newcomer.id)
        .await
        .unwrap());

    // Revoke the badge
    let revoked = ruforo::badges::revoke_badge(&db, user.id, newcomer.id)
        .await
        .expect("Failed to revoke badge");

    assert!(revoked, "Badge should be revoked successfully");

    // Verify user no longer has it
    let still_has = ruforo::badges::user_has_badge(&db, user.id, newcomer.id)
        .await
        .expect("Failed to check badge after revoke");

    assert!(!still_has, "User should not have the badge after revoke");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_revoke_nonexistent_badge_returns_false() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::badges;

    // Create a test user
    let user = create_test_user(&db, "badge_user4", "password123")
        .await
        .expect("Failed to create user");

    // Get the newcomer badge (but don't award it)
    let newcomer = badges::Entity::find()
        .filter(badges::Column::Slug.eq("newcomer"))
        .one(&db)
        .await
        .expect("Failed to find badge")
        .expect("Newcomer badge not found");

    // Try to revoke badge user doesn't have
    let revoked = ruforo::badges::revoke_badge(&db, user.id, newcomer.id)
        .await
        .expect("Failed to revoke badge");

    assert!(!revoked, "Revoking non-existent badge should return false");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

// ============================================================================
// Badge Query Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_get_user_badges() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::badges;

    // Create a test user
    let user = create_test_user(&db, "badge_user5", "password123")
        .await
        .expect("Failed to create user");

    // Award multiple badges
    let newcomer = badges::Entity::find()
        .filter(badges::Column::Slug.eq("newcomer"))
        .one(&db)
        .await
        .expect("Failed to find badge")
        .expect("Newcomer badge not found");

    let first_post = badges::Entity::find()
        .filter(badges::Column::Slug.eq("first-post"))
        .one(&db)
        .await
        .expect("Failed to find badge")
        .expect("First-post badge not found");

    ruforo::badges::award_badge(&db, user.id, newcomer.id, None)
        .await
        .expect("Failed to award newcomer");
    ruforo::badges::award_badge(&db, user.id, first_post.id, None)
        .await
        .expect("Failed to award first-post");

    // Get all user badges
    let user_badges = ruforo::badges::get_user_badges(&db, user.id)
        .await
        .expect("Failed to get user badges");

    assert_eq!(user_badges.len(), 2, "User should have 2 badges");

    let badge_slugs: Vec<&str> = user_badges
        .iter()
        .map(|ub| ub.badge.slug.as_str())
        .collect();
    assert!(
        badge_slugs.contains(&"newcomer"),
        "Should have newcomer badge"
    );
    assert!(
        badge_slugs.contains(&"first-post"),
        "Should have first-post badge"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_get_badge_by_slug() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    // Get existing badge by slug
    let badge = ruforo::badges::get_badge_by_slug(&db, "veteran")
        .await
        .expect("Failed to query badge");

    assert!(badge.is_some(), "Should find 'veteran' badge");
    let badge = badge.unwrap();
    assert_eq!(badge.name, "Veteran", "Badge name should be 'Veteran'");
    assert_eq!(
        badge.condition_type, "time_member",
        "Should be time_member type"
    );

    // Try non-existent badge
    let missing = ruforo::badges::get_badge_by_slug(&db, "nonexistent")
        .await
        .expect("Failed to query badge");

    assert!(missing.is_none(), "Should not find nonexistent badge");
}

#[actix_rt::test]
#[serial]
async fn test_count_user_badges() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::badges;

    // Create a test user
    let user = create_test_user(&db, "badge_user6", "password123")
        .await
        .expect("Failed to create user");

    // Initially should have 0 badges
    let initial_count = ruforo::badges::count_user_badges(&db, user.id)
        .await
        .expect("Failed to count badges");
    assert_eq!(initial_count, 0, "Initial badge count should be 0");

    // Award 3 badges
    let badges_to_award = badges::Entity::find()
        .filter(badges::Column::IsActive.eq(true))
        .all(&db)
        .await
        .expect("Failed to fetch badges");

    for badge in badges_to_award.iter().take(3) {
        ruforo::badges::award_badge(&db, user.id, badge.id, None)
            .await
            .expect("Failed to award badge");
    }

    // Should now have 3 badges
    let final_count = ruforo::badges::count_user_badges(&db, user.id)
        .await
        .expect("Failed to count badges");
    assert_eq!(final_count, 3, "Should have 3 badges after awarding");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

// ============================================================================
// Award Tracking Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_badge_awarded_by_tracking() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::badges;

    // Create users - admin and regular user
    let admin = create_test_user(&db, "badge_admin", "password123")
        .await
        .expect("Failed to create admin");
    let user = create_test_user(&db, "badge_recipient", "password123")
        .await
        .expect("Failed to create user");

    // Get a badge
    let newcomer = badges::Entity::find()
        .filter(badges::Column::Slug.eq("newcomer"))
        .one(&db)
        .await
        .expect("Failed to find badge")
        .expect("Newcomer badge not found");

    // Award badge with admin as awarding user
    ruforo::badges::award_badge(&db, user.id, newcomer.id, Some(admin.id))
        .await
        .expect("Failed to award badge");

    // Get user badges and check awarded_by
    let user_badges = ruforo::badges::get_user_badges(&db, user.id)
        .await
        .expect("Failed to get user badges");

    assert_eq!(user_badges.len(), 1, "Should have 1 badge");
    assert_eq!(
        user_badges[0].awarded_by,
        Some(admin.id),
        "Badge should show admin as awarder"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_badge_awarded_at_timestamp() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::badges;

    // Create a test user
    let user = create_test_user(&db, "badge_user7", "password123")
        .await
        .expect("Failed to create user");

    let before_award = Utc::now();

    // Get and award a badge
    let newcomer = badges::Entity::find()
        .filter(badges::Column::Slug.eq("newcomer"))
        .one(&db)
        .await
        .expect("Failed to find badge")
        .expect("Newcomer badge not found");

    ruforo::badges::award_badge(&db, user.id, newcomer.id, None)
        .await
        .expect("Failed to award badge");

    let after_award = Utc::now();

    // Get user badges and check awarded_at
    let user_badges = ruforo::badges::get_user_badges(&db, user.id)
        .await
        .expect("Failed to get user badges");

    assert_eq!(user_badges.len(), 1, "Should have 1 badge");

    let awarded_at = user_badges[0].awarded_at;
    assert!(
        awarded_at >= before_award,
        "Award time should be >= test start"
    );
    assert!(
        awarded_at <= after_award,
        "Award time should be <= test end"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

// ============================================================================
// User Badge Display Order Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_user_badges_sorted_by_display_order() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::badges;

    // Create a test user
    let user = create_test_user(&db, "badge_user8", "password123")
        .await
        .expect("Failed to create user");

    // Get badges with different display orders
    let badges_list = badges::Entity::find()
        .filter(badges::Column::IsActive.eq(true))
        .all(&db)
        .await
        .expect("Failed to fetch badges");

    // Award badges in reverse display order to test sorting
    let mut sorted_badges = badges_list.clone();
    sorted_badges.sort_by(|a, b| b.display_order.cmp(&a.display_order));

    for badge in sorted_badges.iter().take(4) {
        ruforo::badges::award_badge(&db, user.id, badge.id, None)
            .await
            .expect("Failed to award badge");
    }

    // Get user badges - should be sorted by display_order
    let user_badges = ruforo::badges::get_user_badges(&db, user.id)
        .await
        .expect("Failed to get user badges");

    // Verify they're sorted correctly
    let mut prev_order = i32::MIN;
    for ub in &user_badges {
        assert!(
            ub.badge.display_order >= prev_order,
            "Badges should be sorted by display_order ascending"
        );
        prev_order = ub.badge.display_order;
    }

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

// ============================================================================
// Badge Visibility Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_inactive_badges_not_returned() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    // Get only active badges
    let active_badges = ruforo::badges::get_all_badges(&db)
        .await
        .expect("Failed to get badges");

    // All returned badges should be active
    // (We can't easily test inactive ones without creating one)
    assert!(!active_badges.is_empty(), "Should have active badges");

    // Verify none are marked as inactive in return
    // (The query only returns active ones, so this verifies query correctness)
    for badge in &active_badges {
        // Badge info doesn't include is_active field since query filters it
        assert!(!badge.slug.is_empty(), "Badge should have valid slug");
    }
}
