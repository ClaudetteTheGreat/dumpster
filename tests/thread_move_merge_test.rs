/// Integration tests for thread move and merge features
mod common;
use serial_test::serial;

use chrono::Utc;
use common::{database::*, fixtures::*};
use dumpster::orm::{forums, posts, threads, ugc, ugc_revisions};
use sea_orm::{entity::*, query::*, ActiveValue::Set, DatabaseConnection, DbErr};

/// Create a test forum
async fn create_test_forum(db: &DatabaseConnection, name: &str) -> Result<forums::Model, DbErr> {
    let forum = forums::ActiveModel {
        label: Set(name.to_string()),
        description: Set(Some("Test forum".to_string())),
        ..Default::default()
    };
    forum.insert(db).await
}

/// Create a test thread
async fn create_test_thread(
    db: &DatabaseConnection,
    forum_id: i32,
    user_id: i32,
    title: &str,
) -> Result<threads::Model, DbErr> {
    let thread = threads::ActiveModel {
        forum_id: Set(forum_id),
        user_id: Set(Some(user_id)),
        title: Set(title.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        post_count: Set(0),
        view_count: Set(0),
        is_locked: Set(false),
        is_pinned: Set(false),
        is_announcement: Set(false),
        ..Default::default()
    };
    thread.insert(db).await
}

/// Create UGC (user-generated content) with content
async fn create_test_ugc(
    db: &DatabaseConnection,
    user_id: i32,
    content: &str,
) -> Result<ugc::Model, DbErr> {
    // Create UGC entry
    let ugc_entry = ugc::ActiveModel {
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(db).await?;

    // Create UGC revision with content
    let revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(Some(user_id)),
        content: Set(content.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ip_id: Set(None),
        ..Default::default()
    };
    revision.insert(db).await?;

    Ok(ugc_model)
}

/// Create a test post
async fn create_test_post(
    db: &DatabaseConnection,
    thread_id: i32,
    user_id: i32,
    position: i32,
    content: &str,
) -> Result<posts::Model, DbErr> {
    let ugc_model = create_test_ugc(db, user_id, content).await?;

    let post = posts::ActiveModel {
        thread_id: Set(thread_id),
        user_id: Set(Some(user_id)),
        ugc_id: Set(ugc_model.id),
        position: Set(position),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    post.insert(db).await
}

#[actix_rt::test]
#[serial]
async fn test_thread_move_updates_forum_id() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create test user");

    let source_forum = create_test_forum(&db, "Source Forum")
        .await
        .expect("Failed to create source forum");

    let target_forum = create_test_forum(&db, "Target Forum")
        .await
        .expect("Failed to create target forum");

    let thread = create_test_thread(&db, source_forum.id, user.id, "Thread to Move")
        .await
        .expect("Failed to create thread");

    assert_eq!(
        thread.forum_id, source_forum.id,
        "Thread should start in source forum"
    );

    // Move the thread by updating forum_id
    let mut active_thread: threads::ActiveModel = thread.clone().into();
    active_thread.forum_id = Set(target_forum.id);
    let moved_thread = active_thread
        .update(&db)
        .await
        .expect("Failed to move thread");

    assert_eq!(
        moved_thread.forum_id, target_forum.id,
        "Thread should be in target forum after move"
    );

    // Verify from database
    let fetched_thread = threads::Entity::find_by_id(thread.id)
        .one(&db)
        .await
        .expect("Failed to fetch thread")
        .expect("Thread not found");

    assert_eq!(
        fetched_thread.forum_id, target_forum.id,
        "Thread forum_id should persist"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_thread_move_preserves_other_properties() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create test user");

    let source_forum = create_test_forum(&db, "Source Forum")
        .await
        .expect("Failed to create source forum");

    let target_forum = create_test_forum(&db, "Target Forum")
        .await
        .expect("Failed to create target forum");

    // Create a pinned, locked thread
    let thread = threads::ActiveModel {
        forum_id: Set(source_forum.id),
        user_id: Set(Some(user.id)),
        title: Set("Locked Pinned Thread".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        post_count: Set(5),
        view_count: Set(100),
        is_locked: Set(true),
        is_pinned: Set(true),
        is_announcement: Set(false),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Failed to create thread");

    // Move the thread
    let mut active_thread: threads::ActiveModel = thread.clone().into();
    active_thread.forum_id = Set(target_forum.id);
    let moved_thread = active_thread
        .update(&db)
        .await
        .expect("Failed to move thread");

    // Verify properties are preserved
    assert_eq!(moved_thread.title, "Locked Pinned Thread");
    assert_eq!(moved_thread.post_count, 5);
    assert_eq!(moved_thread.view_count, 100);
    assert!(moved_thread.is_locked, "Thread should still be locked");
    assert!(moved_thread.is_pinned, "Thread should still be pinned");
    assert_eq!(moved_thread.user_id, Some(user.id));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_thread_merge_moves_posts() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create test user");

    let forum = create_test_forum(&db, "Test Forum")
        .await
        .expect("Failed to create forum");

    let source_thread = create_test_thread(&db, forum.id, user.id, "Source Thread")
        .await
        .expect("Failed to create source thread");

    let target_thread = create_test_thread(&db, forum.id, user.id, "Target Thread")
        .await
        .expect("Failed to create target thread");

    // Create posts in source thread
    let _post1 = create_test_post(&db, source_thread.id, user.id, 1, "Post 1")
        .await
        .expect("Failed to create post 1");

    let _post2 = create_test_post(&db, source_thread.id, user.id, 2, "Post 2")
        .await
        .expect("Failed to create post 2");

    // Create a post in target thread
    let _target_post = create_test_post(&db, target_thread.id, user.id, 1, "Target Post")
        .await
        .expect("Failed to create target post");

    // Move posts from source to target
    posts::Entity::update_many()
        .col_expr(
            posts::Column::ThreadId,
            sea_orm::sea_query::Expr::value(target_thread.id),
        )
        .filter(posts::Column::ThreadId.eq(source_thread.id))
        .exec(&db)
        .await
        .expect("Failed to move posts");

    // Verify posts are now in target thread
    let target_posts = posts::Entity::find()
        .filter(posts::Column::ThreadId.eq(target_thread.id))
        .all(&db)
        .await
        .expect("Failed to fetch target posts");

    assert_eq!(target_posts.len(), 3, "Target thread should have 3 posts");

    // Verify no posts remain in source thread
    let source_posts = posts::Entity::find()
        .filter(posts::Column::ThreadId.eq(source_thread.id))
        .all(&db)
        .await
        .expect("Failed to fetch source posts");

    assert_eq!(source_posts.len(), 0, "Source thread should have no posts");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_merged_thread_marked_as_merged() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create test user");

    let forum = create_test_forum(&db, "Test Forum")
        .await
        .expect("Failed to create forum");

    let source_thread = create_test_thread(&db, forum.id, user.id, "Source Thread")
        .await
        .expect("Failed to create source thread");

    let target_thread = create_test_thread(&db, forum.id, user.id, "Target Thread")
        .await
        .expect("Failed to create target thread");

    // Mark source thread as merged
    let mut active_thread: threads::ActiveModel = source_thread.clone().into();
    active_thread.merged_into_id = Set(Some(target_thread.id));
    active_thread.post_count = Set(0);
    let merged_thread = active_thread
        .update(&db)
        .await
        .expect("Failed to mark thread as merged");

    assert_eq!(
        merged_thread.merged_into_id,
        Some(target_thread.id),
        "Source thread should reference target"
    );
    assert_eq!(
        merged_thread.post_count, 0,
        "Merged thread should have 0 posts"
    );

    // Verify from database
    let fetched_thread = threads::Entity::find_by_id(source_thread.id)
        .one(&db)
        .await
        .expect("Failed to fetch thread")
        .expect("Thread not found");

    assert_eq!(
        fetched_thread.merged_into_id,
        Some(target_thread.id),
        "merged_into_id should persist"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_find_merged_threads() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create test user");

    let forum = create_test_forum(&db, "Test Forum")
        .await
        .expect("Failed to create forum");

    let target_thread = create_test_thread(&db, forum.id, user.id, "Target Thread")
        .await
        .expect("Failed to create target thread");

    // Create multiple threads that will be merged into target
    let merged1 = threads::ActiveModel {
        forum_id: Set(forum.id),
        user_id: Set(Some(user.id)),
        title: Set("Merged 1".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        merged_into_id: Set(Some(target_thread.id)),
        post_count: Set(0),
        view_count: Set(0),
        is_locked: Set(false),
        is_pinned: Set(false),
        is_announcement: Set(false),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Failed to create merged thread 1");

    let merged2 = threads::ActiveModel {
        forum_id: Set(forum.id),
        user_id: Set(Some(user.id)),
        title: Set("Merged 2".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        merged_into_id: Set(Some(target_thread.id)),
        post_count: Set(0),
        view_count: Set(0),
        is_locked: Set(false),
        is_pinned: Set(false),
        is_announcement: Set(false),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Failed to create merged thread 2");

    // Find all threads merged into target
    let merged_threads = threads::Entity::find()
        .filter(threads::Column::MergedIntoId.eq(target_thread.id))
        .all(&db)
        .await
        .expect("Failed to find merged threads");

    assert_eq!(merged_threads.len(), 2, "Should find 2 merged threads");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_thread_move_between_different_forums() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create test user");

    // Create three forums
    let forum1 = create_test_forum(&db, "Forum 1")
        .await
        .expect("Failed to create forum 1");

    let forum2 = create_test_forum(&db, "Forum 2")
        .await
        .expect("Failed to create forum 2");

    let forum3 = create_test_forum(&db, "Forum 3")
        .await
        .expect("Failed to create forum 3");

    // Create thread in forum1
    let thread = create_test_thread(&db, forum1.id, user.id, "Traveling Thread")
        .await
        .expect("Failed to create thread");

    // Move to forum2
    let mut active_thread: threads::ActiveModel = thread.clone().into();
    active_thread.forum_id = Set(forum2.id);
    let thread = active_thread
        .update(&db)
        .await
        .expect("Failed to move to forum2");

    assert_eq!(thread.forum_id, forum2.id);

    // Move to forum3
    let mut active_thread: threads::ActiveModel = thread.into();
    active_thread.forum_id = Set(forum3.id);
    let thread = active_thread
        .update(&db)
        .await
        .expect("Failed to move to forum3");

    assert_eq!(thread.forum_id, forum3.id);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_exclude_merged_threads_from_listing() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user(&db, "testuser", "password123")
        .await
        .expect("Failed to create test user");

    let forum = create_test_forum(&db, "Test Forum")
        .await
        .expect("Failed to create forum");

    // Create regular thread
    let regular_thread = create_test_thread(&db, forum.id, user.id, "Regular Thread")
        .await
        .expect("Failed to create regular thread");

    // Create target thread
    let target_thread = create_test_thread(&db, forum.id, user.id, "Target Thread")
        .await
        .expect("Failed to create target thread");

    // Create merged thread
    let merged_thread = threads::ActiveModel {
        forum_id: Set(forum.id),
        user_id: Set(Some(user.id)),
        title: Set("Merged Thread".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        merged_into_id: Set(Some(target_thread.id)),
        post_count: Set(0),
        view_count: Set(0),
        is_locked: Set(false),
        is_pinned: Set(false),
        is_announcement: Set(false),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("Failed to create merged thread");

    // Query threads excluding merged ones
    let visible_threads = threads::Entity::find()
        .filter(threads::Column::ForumId.eq(forum.id))
        .filter(threads::Column::MergedIntoId.is_null())
        .all(&db)
        .await
        .expect("Failed to fetch threads");

    assert_eq!(
        visible_threads.len(),
        2,
        "Should only see 2 non-merged threads"
    );

    // Verify merged thread is excluded
    let thread_ids: Vec<i32> = visible_threads.iter().map(|t| t.id).collect();
    assert!(
        !thread_ids.contains(&merged_thread.id),
        "Merged thread should be excluded"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
