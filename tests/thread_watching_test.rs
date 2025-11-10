/// Integration tests for thread watching functionality
/// Tests watching/unwatching threads, notification delivery, and watched thread listing

mod common;
use serial_test::serial;

use common::*;
use ruforo::notifications;
use ruforo::orm::{forums, threads, watched_threads};
use sea_orm::{entity::*, ActiveValue::Set, DatabaseConnection, DbErr};

/// Create a test thread
async fn create_test_thread(
    db: &DatabaseConnection,
    forum_id: i32,
    user_id: i32,
    title: &str,
) -> Result<i32, DbErr> {
    let thread = threads::ActiveModel {
        forum_id: Set(forum_id),
        user_id: Set(Some(user_id)),
        title: Set(title.to_string()),
        subtitle: Set(None),
        view_count: Set(0),
        post_count: Set(1),
        reply_count: Set(0),
        first_post_id: Set(None),
        last_post_id: Set(None),
        last_post_at: Set(None),
        is_locked: Set(false),
        is_pinned: Set(false),
        is_announcement: Set(false),
        ..Default::default()
    };

    let thread_model = thread.insert(db).await?;
    Ok(thread_model.id)
}

/// Create a test forum
async fn create_test_forum(db: &DatabaseConnection, label: &str) -> Result<i32, DbErr> {
    let forum = forums::ActiveModel {
        label: Set(label.to_string()),
        description: Set(None),
        last_post_id: Set(None),
        last_thread_id: Set(None),
        ..Default::default()
    };

    let forum_model = forum.insert(db).await?;
    Ok(forum_model.id)
}

#[actix_rt::test]
#[serial]
async fn test_watch_thread() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test user and forum
    let user = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create user");

    let forum_id = create_test_forum(&db, "Test Forum")
        .await
        .expect("Failed to create forum");

    let thread_id = create_test_thread(&db, forum_id, user.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Watch the thread
    notifications::watch_thread(user.id, thread_id)
        .await
        .expect("Failed to watch thread");

    // Verify watch record was created
    let watch = watched_threads::Entity::find()
        .filter(watched_threads::Column::UserId.eq(user.id))
        .filter(watched_threads::Column::ThreadId.eq(thread_id))
        .one(&db)
        .await
        .expect("Failed to find watch record");

    assert!(watch.is_some());
    let watch_model = watch.unwrap();
    assert_eq!(watch_model.user_id, user.id);
    assert_eq!(watch_model.thread_id, thread_id);
    assert_eq!(watch_model.notify_on_reply, true);
}

#[actix_rt::test]
#[serial]
async fn test_unwatch_thread() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create user");

    let forum_id = create_test_forum(&db, "Test Forum")
        .await
        .expect("Failed to create forum");

    let thread_id = create_test_thread(&db, forum_id, user.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Watch the thread
    notifications::watch_thread(user.id, thread_id)
        .await
        .expect("Failed to watch thread");

    // Verify it's watched
    let is_watching = notifications::is_watching_thread(user.id, thread_id)
        .await
        .expect("Failed to check watch status");
    assert!(is_watching);

    // Unwatch the thread
    notifications::unwatch_thread(user.id, thread_id)
        .await
        .expect("Failed to unwatch thread");

    // Verify it's no longer watched
    let is_watching_after = notifications::is_watching_thread(user.id, thread_id)
        .await
        .expect("Failed to check watch status");
    assert!(!is_watching_after);
}

#[actix_rt::test]
#[serial]
async fn test_is_watching_thread() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "charlie", "charlie@example.com", true)
        .await
        .expect("Failed to create user");

    let forum_id = create_test_forum(&db, "Test Forum")
        .await
        .expect("Failed to create forum");

    let thread_id = create_test_thread(&db, forum_id, user.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Should not be watching initially
    let is_watching = notifications::is_watching_thread(user.id, thread_id)
        .await
        .expect("Failed to check watch status");
    assert!(!is_watching);

    // Watch the thread
    notifications::watch_thread(user.id, thread_id)
        .await
        .expect("Failed to watch thread");

    // Should be watching now
    let is_watching_after = notifications::is_watching_thread(user.id, thread_id)
        .await
        .expect("Failed to check watch status");
    assert!(is_watching_after);
}

#[actix_rt::test]
#[serial]
async fn test_get_watched_threads() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "dave", "dave@example.com", true)
        .await
        .expect("Failed to create user");

    let forum_id = create_test_forum(&db, "Test Forum")
        .await
        .expect("Failed to create forum");

    let thread1_id = create_test_thread(&db, forum_id, user.id, "Thread 1")
        .await
        .expect("Failed to create thread 1");

    let thread2_id = create_test_thread(&db, forum_id, user.id, "Thread 2")
        .await
        .expect("Failed to create thread 2");

    let thread3_id = create_test_thread(&db, forum_id, user.id, "Thread 3")
        .await
        .expect("Failed to create thread 3");

    // Watch two threads
    notifications::watch_thread(user.id, thread1_id)
        .await
        .expect("Failed to watch thread 1");

    notifications::watch_thread(user.id, thread3_id)
        .await
        .expect("Failed to watch thread 3");

    // Get watched threads
    let watched = notifications::get_watched_threads(user.id)
        .await
        .expect("Failed to get watched threads");

    assert_eq!(watched.len(), 2);
    assert!(watched.contains(&thread1_id));
    assert!(watched.contains(&thread3_id));
    assert!(!watched.contains(&thread2_id));
}

#[actix_rt::test]
#[serial]
async fn test_count_watched_threads() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "eve", "eve@example.com", true)
        .await
        .expect("Failed to create user");

    let forum_id = create_test_forum(&db, "Test Forum")
        .await
        .expect("Failed to create forum");

    // Initially no watched threads
    let count_initial = notifications::count_watched_threads(user.id)
        .await
        .expect("Failed to count watched threads");
    assert_eq!(count_initial, 0);

    // Watch some threads
    for i in 1..=5 {
        let thread_id = create_test_thread(&db, forum_id, user.id, &format!("Thread {}", i))
            .await
            .expect("Failed to create thread");

        notifications::watch_thread(user.id, thread_id)
            .await
            .expect("Failed to watch thread");
    }

    // Should have 5 watched threads
    let count_after = notifications::count_watched_threads(user.id)
        .await
        .expect("Failed to count watched threads");
    assert_eq!(count_after, 5);
}

#[actix_rt::test]
#[serial]
async fn test_watch_thread_idempotent() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "frank", "frank@example.com", true)
        .await
        .expect("Failed to create user");

    let forum_id = create_test_forum(&db, "Test Forum")
        .await
        .expect("Failed to create forum");

    let thread_id = create_test_thread(&db, forum_id, user.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Watch the thread multiple times
    notifications::watch_thread(user.id, thread_id)
        .await
        .expect("Failed to watch thread");

    notifications::watch_thread(user.id, thread_id)
        .await
        .expect("Failed to watch thread again");

    notifications::watch_thread(user.id, thread_id)
        .await
        .expect("Failed to watch thread third time");

    // Should still only have one watch record
    let count = notifications::count_watched_threads(user.id)
        .await
        .expect("Failed to count watched threads");
    assert_eq!(count, 1);

    // Verify exactly one record in database
    let watches = watched_threads::Entity::find()
        .filter(watched_threads::Column::UserId.eq(user.id))
        .filter(watched_threads::Column::ThreadId.eq(thread_id))
        .all(&db)
        .await
        .expect("Failed to find watch records");

    assert_eq!(watches.len(), 1);
}

#[actix_rt::test]
#[serial]
async fn test_multiple_users_watching_same_thread() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "grace", "grace@example.com", true)
        .await
        .expect("Failed to create user1");

    let user2 = create_test_user_with_email(&db, "henry", "henry@example.com", true)
        .await
        .expect("Failed to create user2");

    let user3 = create_test_user_with_email(&db, "iris", "iris@example.com", true)
        .await
        .expect("Failed to create user3");

    let forum_id = create_test_forum(&db, "Test Forum")
        .await
        .expect("Failed to create forum");

    let thread_id = create_test_thread(&db, forum_id, user1.id, "Popular Thread")
        .await
        .expect("Failed to create thread");

    // All three users watch the same thread
    notifications::watch_thread(user1.id, thread_id)
        .await
        .expect("Failed to watch thread for user1");

    notifications::watch_thread(user2.id, thread_id)
        .await
        .expect("Failed to watch thread for user2");

    notifications::watch_thread(user3.id, thread_id)
        .await
        .expect("Failed to watch thread for user3");

    // Verify all users are watching
    assert!(notifications::is_watching_thread(user1.id, thread_id)
        .await
        .expect("Failed to check user1"));
    assert!(notifications::is_watching_thread(user2.id, thread_id)
        .await
        .expect("Failed to check user2"));
    assert!(notifications::is_watching_thread(user3.id, thread_id)
        .await
        .expect("Failed to check user3"));

    // User2 unwatches
    notifications::unwatch_thread(user2.id, thread_id)
        .await
        .expect("Failed to unwatch thread for user2");

    // Verify user2 is no longer watching, but others still are
    assert!(notifications::is_watching_thread(user1.id, thread_id)
        .await
        .expect("Failed to check user1"));
    assert!(!notifications::is_watching_thread(user2.id, thread_id)
        .await
        .expect("Failed to check user2"));
    assert!(notifications::is_watching_thread(user3.id, thread_id)
        .await
        .expect("Failed to check user3"));
}
