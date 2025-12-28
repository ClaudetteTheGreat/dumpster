//! Integration tests for user social links

mod common;
use serial_test::serial;

use chrono::Utc;
use common::{database::*, fixtures::*};
use sea_orm::{entity::*, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

// ============================================================================
// Social Platform Tests
// ============================================================================

#[test]
fn test_social_platform_display_names() {
    use ruforo::orm::user_social_links::SocialPlatform;

    assert_eq!(SocialPlatform::Twitter.display_name(), "Twitter/X");
    assert_eq!(SocialPlatform::Discord.display_name(), "Discord");
    assert_eq!(SocialPlatform::Github.display_name(), "GitHub");
    assert_eq!(SocialPlatform::Youtube.display_name(), "YouTube");
    assert_eq!(SocialPlatform::Website.display_name(), "Website");
}

#[test]
fn test_social_platform_icons() {
    use ruforo::orm::user_social_links::SocialPlatform;

    // Each platform should have an icon
    assert!(!SocialPlatform::Twitter.icon().is_empty());
    assert!(!SocialPlatform::Discord.icon().is_empty());
    assert!(!SocialPlatform::Github.icon().is_empty());
}

#[test]
fn test_social_platform_url_patterns() {
    use ruforo::orm::user_social_links::SocialPlatform;

    // Platforms with standard URL patterns
    assert!(SocialPlatform::Twitter.url_pattern().is_some());
    assert!(SocialPlatform::Github.url_pattern().is_some());
    assert!(SocialPlatform::Youtube.url_pattern().is_some());

    // Platforms without standard URL patterns
    assert!(SocialPlatform::Discord.url_pattern().is_none());
    assert!(SocialPlatform::Website.url_pattern().is_none());
    assert!(SocialPlatform::Other.url_pattern().is_none());
}

#[test]
fn test_social_platform_generate_url() {
    use ruforo::orm::user_social_links::SocialPlatform;

    // Twitter URL generation
    let twitter_url = SocialPlatform::Twitter.generate_url("testuser");
    assert_eq!(
        twitter_url,
        Some("https://twitter.com/testuser".to_string())
    );

    // GitHub URL generation
    let github_url = SocialPlatform::Github.generate_url("octocat");
    assert_eq!(github_url, Some("https://github.com/octocat".to_string()));

    // Reddit URL generation
    let reddit_url = SocialPlatform::Reddit.generate_url("spez");
    assert_eq!(reddit_url, Some("https://reddit.com/u/spez".to_string()));

    // Discord returns None (no standard URL pattern)
    let discord_url = SocialPlatform::Discord.generate_url("user#1234");
    assert!(discord_url.is_none());
}

#[test]
fn test_social_platform_parse() {
    use ruforo::orm::user_social_links::SocialPlatform;

    assert_eq!(
        SocialPlatform::parse("twitter"),
        Some(SocialPlatform::Twitter)
    );
    assert_eq!(
        SocialPlatform::parse("TWITTER"),
        Some(SocialPlatform::Twitter)
    );
    assert_eq!(
        SocialPlatform::parse("Twitter"),
        Some(SocialPlatform::Twitter)
    );
    assert_eq!(
        SocialPlatform::parse("github"),
        Some(SocialPlatform::Github)
    );
    assert_eq!(
        SocialPlatform::parse("discord"),
        Some(SocialPlatform::Discord)
    );
    assert_eq!(SocialPlatform::parse("invalid"), None);
}

#[test]
fn test_social_platform_all() {
    use ruforo::orm::user_social_links::SocialPlatform;

    let all = SocialPlatform::all();

    // Should have all 14 platforms
    assert_eq!(all.len(), 14);

    // Check that common platforms are included
    assert!(all.contains(&SocialPlatform::Twitter));
    assert!(all.contains(&SocialPlatform::Discord));
    assert!(all.contains(&SocialPlatform::Github));
    assert!(all.contains(&SocialPlatform::Website));
    assert!(all.contains(&SocialPlatform::Other));
}

// ============================================================================
// Social Link CRUD Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_create_social_link() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::user_social_links::{self, SocialPlatform};

    // Create a test user
    let user = create_test_user(&db, "social_user1", "password123")
        .await
        .expect("Failed to create user");

    // Create a social link
    let link = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Twitter),
        username: Set("test_twitter".to_string()),
        url: Set(None),
        display_order: Set(0),
        is_visible: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    let inserted = link
        .insert(&db)
        .await
        .expect("Failed to insert social link");

    assert_eq!(inserted.user_id, user.id);
    assert_eq!(inserted.platform, SocialPlatform::Twitter);
    assert_eq!(inserted.username, "test_twitter");
    assert!(inserted.is_visible);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_unique_platform_per_user() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::user_social_links::{self, SocialPlatform};

    // Create a test user
    let user = create_test_user(&db, "social_user2", "password123")
        .await
        .expect("Failed to create user");

    // Create first Twitter link
    let link1 = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Twitter),
        username: Set("first_twitter".to_string()),
        url: Set(None),
        display_order: Set(0),
        is_visible: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    link1
        .insert(&db)
        .await
        .expect("Failed to insert first link");

    // Try to create duplicate Twitter link (should fail)
    let link2 = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Twitter),
        username: Set("second_twitter".to_string()),
        url: Set(None),
        display_order: Set(1),
        is_visible: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    let result = link2.insert(&db).await;

    assert!(
        result.is_err(),
        "Duplicate platform for same user should fail"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_multiple_platforms_per_user() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::user_social_links::{self, SocialPlatform};

    // Create a test user
    let user = create_test_user(&db, "social_user3", "password123")
        .await
        .expect("Failed to create user");

    // Create links for different platforms
    let platforms = vec![
        (SocialPlatform::Twitter, "twitter_user"),
        (SocialPlatform::Github, "github_user"),
        (SocialPlatform::Discord, "discord_user"),
    ];

    for (i, (platform, username)) in platforms.iter().enumerate() {
        let link = user_social_links::ActiveModel {
            user_id: Set(user.id),
            platform: Set(platform.clone()),
            username: Set(username.to_string()),
            url: Set(None),
            display_order: Set(i as i32),
            is_visible: Set(true),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };
        link.insert(&db).await.expect("Failed to insert link");
    }

    // Verify all links exist
    let links = user_social_links::Entity::find()
        .filter(user_social_links::Column::UserId.eq(user.id))
        .all(&db)
        .await
        .expect("Failed to fetch links");

    assert_eq!(links.len(), 3, "User should have 3 social links");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_delete_social_link() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::user_social_links::{self, SocialPlatform};

    // Create a test user
    let user = create_test_user(&db, "social_user4", "password123")
        .await
        .expect("Failed to create user");

    // Create a social link
    let link = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Github),
        username: Set("github_user".to_string()),
        url: Set(None),
        display_order: Set(0),
        is_visible: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    link.insert(&db).await.expect("Failed to insert link");

    // Delete the link
    let result = user_social_links::Entity::delete_many()
        .filter(user_social_links::Column::UserId.eq(user.id))
        .filter(user_social_links::Column::Platform.eq(SocialPlatform::Github))
        .exec(&db)
        .await
        .expect("Failed to delete link");

    assert_eq!(result.rows_affected, 1, "Should delete 1 row");

    // Verify link is gone
    let links = user_social_links::Entity::find()
        .filter(user_social_links::Column::UserId.eq(user.id))
        .all(&db)
        .await
        .expect("Failed to fetch links");

    assert!(
        links.is_empty(),
        "User should have no social links after deletion"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

// ============================================================================
// Social Link URL Generation Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_social_link_get_url_with_pattern() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::user_social_links::{self, SocialPlatform};

    // Create a test user
    let user = create_test_user(&db, "social_user5", "password123")
        .await
        .expect("Failed to create user");

    // Create a Twitter link without custom URL
    let link = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Twitter),
        username: Set("my_handle".to_string()),
        url: Set(None), // No custom URL
        display_order: Set(0),
        is_visible: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    let inserted = link.insert(&db).await.expect("Failed to insert link");

    // Should generate URL from pattern
    assert_eq!(inserted.get_url(), "https://twitter.com/my_handle");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_social_link_get_url_with_custom_url() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::user_social_links::{self, SocialPlatform};

    // Create a test user
    let user = create_test_user(&db, "social_user6", "password123")
        .await
        .expect("Failed to create user");

    // Create a Website link with custom URL
    let link = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Website),
        username: Set("My Site".to_string()),
        url: Set(Some("https://example.com/my-page".to_string())),
        display_order: Set(0),
        is_visible: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    let inserted = link.insert(&db).await.expect("Failed to insert link");

    // Should use custom URL
    assert_eq!(inserted.get_url(), "https://example.com/my-page");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

// ============================================================================
// Display Order Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_social_links_ordered_by_display_order() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::user_social_links::{self, SocialPlatform};

    // Create a test user
    let user = create_test_user(&db, "social_user7", "password123")
        .await
        .expect("Failed to create user");

    // Create links in reverse order
    let link3 = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Reddit),
        username: Set("reddit_user".to_string()),
        url: Set(None),
        display_order: Set(2),
        is_visible: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    link3.insert(&db).await.expect("Failed to insert link");

    let link1 = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Twitter),
        username: Set("twitter_user".to_string()),
        url: Set(None),
        display_order: Set(0),
        is_visible: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    link1.insert(&db).await.expect("Failed to insert link");

    let link2 = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Github),
        username: Set("github_user".to_string()),
        url: Set(None),
        display_order: Set(1),
        is_visible: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    link2.insert(&db).await.expect("Failed to insert link");

    // Fetch links ordered by display_order
    let links = user_social_links::Entity::find()
        .filter(user_social_links::Column::UserId.eq(user.id))
        .order_by_asc(user_social_links::Column::DisplayOrder)
        .all(&db)
        .await
        .expect("Failed to fetch links");

    assert_eq!(links.len(), 3);
    assert_eq!(links[0].platform, SocialPlatform::Twitter);
    assert_eq!(links[1].platform, SocialPlatform::Github);
    assert_eq!(links[2].platform, SocialPlatform::Reddit);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

// ============================================================================
// Visibility Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_filter_visible_social_links() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::user_social_links::{self, SocialPlatform};

    // Create a test user
    let user = create_test_user(&db, "social_user8", "password123")
        .await
        .expect("Failed to create user");

    // Create visible link
    let visible = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Twitter),
        username: Set("visible_user".to_string()),
        url: Set(None),
        display_order: Set(0),
        is_visible: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    visible.insert(&db).await.expect("Failed to insert link");

    // Create hidden link
    let hidden = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Github),
        username: Set("hidden_user".to_string()),
        url: Set(None),
        display_order: Set(1),
        is_visible: Set(false),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    hidden.insert(&db).await.expect("Failed to insert link");

    // Fetch only visible links
    let visible_links = user_social_links::Entity::find()
        .filter(user_social_links::Column::UserId.eq(user.id))
        .filter(user_social_links::Column::IsVisible.eq(true))
        .all(&db)
        .await
        .expect("Failed to fetch links");

    assert_eq!(visible_links.len(), 1, "Should only have 1 visible link");
    assert_eq!(visible_links[0].platform, SocialPlatform::Twitter);

    // Fetch all links
    let all_links = user_social_links::Entity::find()
        .filter(user_social_links::Column::UserId.eq(user.id))
        .all(&db)
        .await
        .expect("Failed to fetch links");

    assert_eq!(all_links.len(), 2, "Should have 2 total links");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

// ============================================================================
// User Deletion Cascade Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_social_links_deleted_with_user() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{
        user_names,
        user_social_links::{self, SocialPlatform},
        users,
    };

    // Create a test user
    let user = create_test_user(&db, "social_user9", "password123")
        .await
        .expect("Failed to create user");

    // Create social links
    let link = user_social_links::ActiveModel {
        user_id: Set(user.id),
        platform: Set(SocialPlatform::Twitter),
        username: Set("twitter_user".to_string()),
        url: Set(None),
        display_order: Set(0),
        is_visible: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    link.insert(&db).await.expect("Failed to insert link");

    // Verify link exists
    let links_before = user_social_links::Entity::find()
        .filter(user_social_links::Column::UserId.eq(user.id))
        .all(&db)
        .await
        .expect("Failed to fetch links");
    assert_eq!(links_before.len(), 1);

    // Delete user_names first (not cascaded by default)
    user_names::Entity::delete_many()
        .filter(user_names::Column::UserId.eq(user.id))
        .exec(&db)
        .await
        .expect("Failed to delete user names");

    // Delete user (should cascade to social links)
    users::Entity::delete_by_id(user.id)
        .exec(&db)
        .await
        .expect("Failed to delete user");

    // Verify links are gone
    let links_after = user_social_links::Entity::find()
        .filter(user_social_links::Column::UserId.eq(user.id))
        .all(&db)
        .await
        .expect("Failed to fetch links");
    assert!(
        links_after.is_empty(),
        "Social links should be deleted with user"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
