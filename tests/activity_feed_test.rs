//! Integration tests for activity feed system

mod common;
use serial_test::serial;

use common::{database::*, fixtures::*};
use dumpster::activities::ActivityCursor;
use dumpster::orm::activities::{self, ActivityType};
use sea_orm::{entity::*, ActiveValue::Set, EntityTrait};

#[actix_rt::test]
#[serial]
async fn test_create_thread_activity() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test user and forum/thread
    let user = create_test_user(&db, "thread_creator", "password123")
        .await
        .expect("Failed to create user");

    let (forum, thread) = create_test_forum_and_thread(&db, user.id, "Test Thread Title")
        .await
        .expect("Failed to create forum");

    // Create thread activity directly via ORM (using actual thread)
    let activity = activities::ActiveModel {
        activity_type: Set(ActivityType::ThreadCreated),
        user_id: Set(user.id),
        target_thread_id: Set(Some(thread.id)),
        target_forum_id: Set(Some(forum.id)),
        title: Set(Some("Test Thread Title".to_string())),
        ..Default::default()
    };
    let activity_model = activity
        .insert(&db)
        .await
        .expect("Failed to create activity");

    // Verify activity was created
    assert!(activity_model.id > 0);
    assert_eq!(activity_model.user_id, user.id);
    assert_eq!(activity_model.target_thread_id, Some(thread.id));
    assert_eq!(activity_model.target_forum_id, Some(forum.id));
    assert_eq!(activity_model.title, Some("Test Thread Title".to_string()));
    assert_eq!(activity_model.activity_type, ActivityType::ThreadCreated);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_create_post_activity() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test user and forum/thread
    let user = create_test_user(&db, "post_creator", "password123")
        .await
        .expect("Failed to create user");

    let (forum, thread) = create_test_forum_and_thread(&db, user.id, "Thread Title")
        .await
        .expect("Failed to create forum");

    // Create post activity directly via ORM (target_post_id is optional FK, can be None)
    let activity = activities::ActiveModel {
        activity_type: Set(ActivityType::PostCreated),
        user_id: Set(user.id),
        target_thread_id: Set(Some(thread.id)),
        target_post_id: Set(None), // Posts require UGC, skip for this test
        target_forum_id: Set(Some(forum.id)),
        title: Set(Some("Thread Title".to_string())),
        content_preview: Set(Some("This is a preview of the post content...".to_string())),
        ..Default::default()
    };
    let activity_model = activity
        .insert(&db)
        .await
        .expect("Failed to create activity");

    // Verify activity was created
    assert!(activity_model.id > 0);
    assert_eq!(activity_model.user_id, user.id);
    assert_eq!(activity_model.target_thread_id, Some(thread.id));
    assert_eq!(activity_model.activity_type, ActivityType::PostCreated);
    assert!(activity_model.content_preview.is_some());

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_create_follow_activity() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create two users
    let follower = create_test_user(&db, "follower_activity", "password123")
        .await
        .expect("Failed to create follower");

    let following = create_test_user(&db, "following_activity", "password123")
        .await
        .expect("Failed to create following");

    // Create follow activity directly via ORM
    let activity = activities::ActiveModel {
        activity_type: Set(ActivityType::UserFollowed),
        user_id: Set(follower.id),
        target_user_id: Set(Some(following.id)),
        title: Set(Some("following_activity".to_string())),
        ..Default::default()
    };
    let activity_model = activity
        .insert(&db)
        .await
        .expect("Failed to create activity");

    // Verify activity was created
    assert!(activity_model.id > 0);
    assert_eq!(activity_model.user_id, follower.id);
    assert_eq!(activity_model.target_user_id, Some(following.id));
    assert_eq!(activity_model.activity_type, ActivityType::UserFollowed);
    assert_eq!(activity_model.title, Some("following_activity".to_string()));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_create_profile_post_activity() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create two users
    let author = create_test_user(&db, "profile_poster", "password123")
        .await
        .expect("Failed to create author");

    let profile_user = create_test_user(&db, "profile_owner", "password123")
        .await
        .expect("Failed to create profile owner");

    // Create profile post activity directly via ORM
    let activity = activities::ActiveModel {
        activity_type: Set(ActivityType::ProfilePostCreated),
        user_id: Set(author.id),
        target_user_id: Set(Some(profile_user.id)),
        title: Set(Some("profile_owner".to_string())),
        content_preview: Set(Some("This is a profile post content...".to_string())),
        ..Default::default()
    };
    let activity_model = activity
        .insert(&db)
        .await
        .expect("Failed to create activity");

    // Verify activity was created
    assert!(activity_model.id > 0);
    assert_eq!(activity_model.user_id, author.id);
    assert_eq!(activity_model.target_user_id, Some(profile_user.id));
    assert_eq!(
        activity_model.activity_type,
        ActivityType::ProfilePostCreated
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_create_reaction_activity() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test user and forum/thread
    let user = create_test_user(&db, "reactor", "password123")
        .await
        .expect("Failed to create user");

    let (forum, thread) = create_test_forum_and_thread(&db, user.id, "Thread for Reaction")
        .await
        .expect("Failed to create forum");

    // Create reaction activity directly via ORM
    let activity = activities::ActiveModel {
        activity_type: Set(ActivityType::ReactionGiven),
        user_id: Set(user.id),
        target_post_id: Set(None), // Posts require UGC, skip for this test
        target_thread_id: Set(Some(thread.id)),
        target_forum_id: Set(Some(forum.id)),
        reaction_emoji: Set(Some("👍".to_string())),
        title: Set(Some("Thread for Reaction".to_string())),
        ..Default::default()
    };
    let activity_model = activity
        .insert(&db)
        .await
        .expect("Failed to create activity");

    // Verify activity was created
    assert!(activity_model.id > 0);
    assert_eq!(activity_model.user_id, user.id);
    assert_eq!(activity_model.activity_type, ActivityType::ReactionGiven);
    assert_eq!(activity_model.reaction_emoji, Some("👍".to_string()));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_activity_type_descriptions() {
    // Test activity type descriptions
    assert_eq!(ActivityType::PostCreated.description(), "posted a reply");
    assert_eq!(
        ActivityType::ThreadCreated.description(),
        "started a new thread"
    );
    assert_eq!(
        ActivityType::ProfilePostCreated.description(),
        "posted on a profile"
    );
    assert_eq!(ActivityType::UserFollowed.description(), "followed");
    assert_eq!(
        ActivityType::ReactionGiven.description(),
        "reacted to a post"
    );
}

#[actix_rt::test]
#[serial]
async fn test_activity_type_icons() {
    // Test activity type icons
    assert_eq!(ActivityType::PostCreated.icon(), "💬");
    assert_eq!(ActivityType::ThreadCreated.icon(), "📝");
    assert_eq!(ActivityType::ProfilePostCreated.icon(), "📋");
    assert_eq!(ActivityType::UserFollowed.icon(), "👤");
    assert_eq!(ActivityType::ReactionGiven.icon(), "👍");
}

#[actix_rt::test]
#[serial]
async fn test_activity_cursor_parsing() {
    // Test valid cursor
    let timestamp = 1703692800i64; // 2023-12-27 12:00:00 UTC
    let id = 42;
    let cursor_str = format!("{}_{}", timestamp, id);

    let parsed = ActivityCursor::parse(&cursor_str);
    assert!(parsed.is_some());

    let cursor = parsed.unwrap();
    assert_eq!(cursor.created_at.timestamp(), timestamp);
    assert_eq!(cursor.id, id);

    // Test cursor to string
    let back_to_string = cursor.to_string();
    assert_eq!(back_to_string, cursor_str);

    // Test invalid cursors
    assert!(ActivityCursor::parse("invalid").is_none());
    assert!(ActivityCursor::parse("123").is_none());
    assert!(ActivityCursor::parse("abc_123").is_none());
    assert!(ActivityCursor::parse("123_abc").is_none());
}

#[actix_rt::test]
#[serial]
async fn test_multiple_activities_for_user() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test user
    let user = create_test_user(&db, "multi_activity_user", "password123")
        .await
        .expect("Failed to create user");

    // Create multiple activities (without thread FK to avoid constraint issues)
    for i in 1..=5 {
        let activity = activities::ActiveModel {
            activity_type: Set(ActivityType::UserFollowed),
            user_id: Set(user.id),
            target_user_id: Set(Some(user.id)), // Following self as placeholder
            target_thread_id: Set(None),
            target_forum_id: Set(None),
            title: Set(Some(format!("Activity {}", i))),
            ..Default::default()
        };
        activity
            .insert(&db)
            .await
            .expect("Failed to create activity");
    }

    // Query all activities for user
    use sea_orm::ColumnTrait;
    use sea_orm::QueryFilter;

    let user_activities = activities::Entity::find()
        .filter(activities::Column::UserId.eq(user.id))
        .all(&db)
        .await
        .expect("Failed to query activities");

    assert_eq!(user_activities.len(), 5);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_activity_cascade_delete_on_user_delete() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use dumpster::orm::{ugc_revisions, user_names, users};
    use sea_orm::ColumnTrait;
    use sea_orm::QueryFilter;

    // Create test user
    let user = create_test_user(&db, "cascade_activity_user", "password123")
        .await
        .expect("Failed to create user");

    // Create activity (without thread FK)
    let activity = activities::ActiveModel {
        activity_type: Set(ActivityType::UserFollowed),
        user_id: Set(user.id),
        target_user_id: Set(Some(user.id)),
        target_thread_id: Set(None),
        title: Set(Some("Test Activity".to_string())),
        ..Default::default()
    };
    let activity_model = activity
        .insert(&db)
        .await
        .expect("Failed to create activity");

    // Update any ugc_revisions to not reference this user before deletion
    ugc_revisions::Entity::update_many()
        .col_expr(
            ugc_revisions::Column::UserId,
            sea_orm::sea_query::Expr::value(Option::<i32>::None),
        )
        .filter(ugc_revisions::Column::UserId.eq(user.id))
        .exec(&db)
        .await
        .expect("Failed to update ugc_revisions");

    // Delete user_name entry first (FK constraint)
    user_names::Entity::delete_many()
        .filter(user_names::Column::UserId.eq(user.id))
        .exec(&db)
        .await
        .expect("Failed to delete user_name");

    // Delete the user
    users::Entity::delete_by_id(user.id)
        .exec(&db)
        .await
        .expect("Failed to delete user");

    // Verify activity was cascade deleted
    let activity_check = activities::Entity::find_by_id(activity_model.id)
        .one(&db)
        .await
        .expect("Query failed");
    assert!(activity_check.is_none());

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_activity_ordering() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test user
    let user = create_test_user(&db, "ordering_user", "password123")
        .await
        .expect("Failed to create user");

    // Create activities (without thread FK)
    for i in 1..=3 {
        let activity = activities::ActiveModel {
            activity_type: Set(ActivityType::UserFollowed),
            user_id: Set(user.id),
            target_user_id: Set(Some(user.id)),
            target_thread_id: Set(None),
            title: Set(Some(format!("Activity {}", i))),
            ..Default::default()
        };
        activity
            .insert(&db)
            .await
            .expect("Failed to create activity");
    }

    // Query activities ordered by created_at DESC
    use sea_orm::{ColumnTrait, QueryFilter, QueryOrder};

    let activities_list = activities::Entity::find()
        .filter(activities::Column::UserId.eq(user.id))
        .order_by_desc(activities::Column::CreatedAt)
        .order_by_desc(activities::Column::Id)
        .all(&db)
        .await
        .expect("Failed to query activities");

    assert_eq!(activities_list.len(), 3);

    // Verify ordering (most recent first)
    for i in 0..activities_list.len() - 1 {
        assert!(activities_list[i].id >= activities_list[i + 1].id);
    }

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
