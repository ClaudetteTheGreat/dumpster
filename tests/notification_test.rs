/// Integration tests for notification functionality
/// Tests notification creation, mention detection, reply notifications, and read/unread tracking
mod common;
use serial_test::serial;

use common::{database::*, fixtures::*};
use ruforo::notifications::{self, NotificationType};
use ruforo::orm::{notification_preferences, notifications as notification_orm};
use sea_orm::{entity::*, ActiveValue::Set, DatabaseConnection, DbErr};

/// Create test notification preferences for a user
async fn create_notification_preferences(
    db: &DatabaseConnection,
    user_id: i32,
    notification_type: &str,
    in_app: bool,
    email: bool,
) -> Result<notification_preferences::Model, DbErr> {
    let prefs = notification_preferences::ActiveModel {
        user_id: Set(user_id),
        notification_type: Set(notification_type.to_string()),
        in_app: Set(in_app),
        email: Set(email),
        frequency: Set("immediate".to_string()),
    };

    prefs.insert(db).await
}

#[actix_rt::test]
#[serial]
async fn test_create_notification() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test users
    let user1 = create_test_user_with_email(&db, "user1", "user1@example.com", true)
        .await
        .expect("Failed to create user1");

    let user2 = create_test_user_with_email(&db, "user2", "user2@example.com", true)
        .await
        .expect("Failed to create user2");

    // Create notification preferences
    create_notification_preferences(&db, user1.id, "reply", true, false)
        .await
        .expect("Failed to create preferences");

    // Create a notification
    let notification_id = notifications::create_notification(
        user1.id,
        NotificationType::Reply,
        "Test notification".to_string(),
        "This is a test message".to_string(),
        Some("/test".to_string()),
        Some(user2.id),
        Some("post".to_string()),
        Some(123),
    )
    .await
    .expect("Failed to create notification");

    assert!(notification_id > 0, "Notification should have valid ID");

    // Verify notification was created
    let notification = notification_orm::Entity::find_by_id(notification_id)
        .one(&db)
        .await
        .expect("Failed to fetch notification")
        .expect("Notification should exist");

    assert_eq!(notification.user_id, user1.id);
    assert_eq!(notification.title, "Test notification");
    assert_eq!(notification.message, "This is a test message");
    assert!(!notification.is_read);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_count_unread_notifications() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "testuser", "test@example.com", true)
        .await
        .expect("Failed to create user");

    create_notification_preferences(&db, user.id, "reply", true, false)
        .await
        .expect("Failed to create preferences");

    // Create multiple notifications
    for i in 0..3 {
        notifications::create_notification(
            user.id,
            NotificationType::Reply,
            format!("Notification {}", i),
            format!("Message {}", i),
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to create notification");
    }

    // Count unread notifications
    let count = notifications::count_unread_notifications(user.id)
        .await
        .expect("Failed to count notifications");

    assert_eq!(count, 3, "Should have 3 unread notifications");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_mark_notification_read() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "testuser", "test@example.com", true)
        .await
        .expect("Failed to create user");

    create_notification_preferences(&db, user.id, "reply", true, false)
        .await
        .expect("Failed to create preferences");

    // Create notification
    let notification_id = notifications::create_notification(
        user.id,
        NotificationType::Reply,
        "Test".to_string(),
        "Test message".to_string(),
        None,
        None,
        None,
        None,
    )
    .await
    .expect("Failed to create notification");

    // Verify it's unread
    let count_before = notifications::count_unread_notifications(user.id)
        .await
        .expect("Failed to count");
    assert_eq!(count_before, 1);

    // Mark as read
    notifications::mark_notification_read(notification_id, user.id)
        .await
        .expect("Failed to mark as read");

    // Verify it's now read
    let count_after = notifications::count_unread_notifications(user.id)
        .await
        .expect("Failed to count");
    assert_eq!(count_after, 0);

    // Verify the notification was updated
    let notification = notification_orm::Entity::find_by_id(notification_id)
        .one(&db)
        .await
        .expect("Failed to fetch notification")
        .expect("Notification should exist");

    assert!(
        notification.is_read,
        "Notification should be marked as read"
    );
    assert!(
        notification.read_at.is_some(),
        "Read timestamp should be set"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_mark_all_notifications_read() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "testuser", "test@example.com", true)
        .await
        .expect("Failed to create user");

    create_notification_preferences(&db, user.id, "reply", true, false)
        .await
        .expect("Failed to create preferences");

    // Create multiple notifications
    for i in 0..5 {
        notifications::create_notification(
            user.id,
            NotificationType::Reply,
            format!("Notification {}", i),
            format!("Message {}", i),
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to create notification");
    }

    // Verify all are unread
    let count_before = notifications::count_unread_notifications(user.id)
        .await
        .expect("Failed to count");
    assert_eq!(count_before, 5);

    // Mark all as read
    notifications::mark_all_read(user.id)
        .await
        .expect("Failed to mark all as read");

    // Verify all are now read
    let count_after = notifications::count_unread_notifications(user.id)
        .await
        .expect("Failed to count");
    assert_eq!(count_after, 0);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_mention_detection() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test users
    let author = create_test_user_with_email(&db, "author", "author@example.com", true)
        .await
        .expect("Failed to create author");

    let mentioned_user =
        create_test_user_with_email(&db, "mentioned", "mentioned@example.com", true)
            .await
            .expect("Failed to create mentioned user");

    // Create notification preferences
    create_notification_preferences(&db, mentioned_user.id, "mention", true, false)
        .await
        .expect("Failed to create preferences");

    // Create a thread
    let (_forum, thread) = create_test_forum_and_thread(&db, author.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Test mention detection
    let content = "Hey @mentioned, check this out! Also @nonexistent won't get notified.";

    ruforo::notifications::dispatcher::detect_and_notify_mentions(
        content, 1, // post_id
        thread.id, author.id,
    )
    .await
    .expect("Failed to detect mentions");

    // Verify notification was created for mentioned user
    let count = notifications::count_unread_notifications(mentioned_user.id)
        .await
        .expect("Failed to count notifications");

    assert_eq!(count, 1, "Mentioned user should have 1 notification");

    // Verify the notification content
    let notifs = notifications::get_user_notifications(mentioned_user.id, 10, false)
        .await
        .expect("Failed to get notifications");

    assert_eq!(notifs.len(), 1);
    assert_eq!(notifs[0].type_, "mention");
    assert!(notifs[0].title.contains("mentioned you"));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_thread_reply_notification() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create thread author
    let thread_author =
        create_test_user_with_email(&db, "threadauthor", "threadauthor@example.com", true)
            .await
            .expect("Failed to create thread author");

    // Create reply author
    let reply_author =
        create_test_user_with_email(&db, "replyauthor", "replyauthor@example.com", true)
            .await
            .expect("Failed to create reply author");

    // Create notification preferences
    create_notification_preferences(&db, thread_author.id, "reply", true, false)
        .await
        .expect("Failed to create preferences");

    // Create a thread
    let (_forum, thread) = create_test_forum_and_thread(&db, thread_author.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Simulate a reply notification
    ruforo::notifications::dispatcher::notify_thread_reply(thread.id, 2, reply_author.id)
        .await
        .expect("Failed to send reply notification");

    // Verify thread author got notification
    let count = notifications::count_unread_notifications(thread_author.id)
        .await
        .expect("Failed to count notifications");

    assert_eq!(count, 1, "Thread author should have 1 notification");

    // Verify notification content
    let notifs = notifications::get_user_notifications(thread_author.id, 10, false)
        .await
        .expect("Failed to get notifications");

    assert_eq!(notifs.len(), 1);
    assert_eq!(notifs[0].type_, "reply");
    assert!(notifs[0].title.contains("replied to your thread"));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_no_self_notification() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user
    let user = create_test_user_with_email(&db, "testuser", "test@example.com", true)
        .await
        .expect("Failed to create user");

    create_notification_preferences(&db, user.id, "mention", true, false)
        .await
        .expect("Failed to create preferences");

    create_notification_preferences(&db, user.id, "reply", true, false)
        .await
        .expect("Failed to create preferences");

    // Create a thread owned by the user
    let (_forum, thread) = create_test_forum_and_thread(&db, user.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Test self-mention (should not create notification)
    let content = "I'm mentioning myself @testuser";

    ruforo::notifications::dispatcher::detect_and_notify_mentions(content, 1, thread.id, user.id)
        .await
        .expect("Failed to detect mentions");

    // Test self-reply (should not create notification)
    ruforo::notifications::dispatcher::notify_thread_reply(thread.id, 2, user.id)
        .await
        .expect("Failed to send reply notification");

    // Verify no notifications were created
    let count = notifications::count_unread_notifications(user.id)
        .await
        .expect("Failed to count notifications");

    assert_eq!(
        count, 0,
        "User should not receive notifications from their own actions"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_notification_preferences_disabled() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "testuser", "test@example.com", true)
        .await
        .expect("Failed to create user");

    // Create preferences with in_app disabled
    create_notification_preferences(&db, user.id, "reply", false, false)
        .await
        .expect("Failed to create preferences");

    // Try to create notification (should return 0)
    let notification_id = notifications::create_notification(
        user.id,
        NotificationType::Reply,
        "Test".to_string(),
        "Message".to_string(),
        None,
        None,
        None,
        None,
    )
    .await
    .expect("Failed to create notification");

    assert_eq!(
        notification_id, 0,
        "Should return 0 when notifications disabled"
    );

    // Verify no notification was created
    let count = notifications::count_unread_notifications(user.id)
        .await
        .expect("Failed to count");

    assert_eq!(count, 0, "No notification should be created when disabled");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_get_user_notifications() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user = create_test_user_with_email(&db, "testuser", "test@example.com", true)
        .await
        .expect("Failed to create user");

    create_notification_preferences(&db, user.id, "reply", true, false)
        .await
        .expect("Failed to create preferences");

    // Create notifications
    for i in 0..3 {
        notifications::create_notification(
            user.id,
            NotificationType::Reply,
            format!("Notification {}", i),
            format!("Message {}", i),
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to create notification");
    }

    // Get notifications (unread only)
    let notifs = notifications::get_user_notifications(user.id, 10, false)
        .await
        .expect("Failed to get notifications");

    assert_eq!(notifs.len(), 3, "Should return 3 notifications");

    // Mark one as read
    notifications::mark_notification_read(notifs[0].id, user.id)
        .await
        .expect("Failed to mark as read");

    // Get unread only (should be 2)
    let unread = notifications::get_user_notifications(user.id, 10, false)
        .await
        .expect("Failed to get notifications");

    assert_eq!(unread.len(), 2, "Should return 2 unread notifications");

    // Get all (including read)
    let all = notifications::get_user_notifications(user.id, 10, true)
        .await
        .expect("Failed to get notifications");

    assert_eq!(all.len(), 3, "Should return all 3 notifications");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_quote_detection() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test users
    let author = create_test_user_with_email(&db, "quoter", "quoter@example.com", true)
        .await
        .expect("Failed to create author");

    let quoted_user = create_test_user_with_email(&db, "quoteduser", "quoted@example.com", true)
        .await
        .expect("Failed to create quoted user");

    // Create notification preferences
    create_notification_preferences(&db, quoted_user.id, "quote", true, false)
        .await
        .expect("Failed to create preferences");

    // Create a thread
    let (_forum, thread) = create_test_forum_and_thread(&db, author.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Test quote detection with BBCode
    let content = r#"[quote=quoteduser]This is the quoted text[/quote]
And here's my response."#;

    ruforo::notifications::dispatcher::detect_and_notify_quotes(
        content,
        1, // post_id
        thread.id,
        author.id,
    )
    .await
    .expect("Failed to detect quotes");

    // Verify notification was created for quoted user
    let count = notifications::count_unread_notifications(quoted_user.id)
        .await
        .expect("Failed to count notifications");

    assert_eq!(count, 1, "Quoted user should have 1 notification");

    // Verify the notification content
    let notifs = notifications::get_user_notifications(quoted_user.id, 10, false)
        .await
        .expect("Failed to get notifications");

    assert_eq!(notifs.len(), 1);
    assert_eq!(notifs[0].type_, "quote");
    assert!(notifs[0].title.contains("quoted you"));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_quote_detection_multiple_quotes() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test users
    let author = create_test_user_with_email(&db, "quoter2", "quoter2@example.com", true)
        .await
        .expect("Failed to create author");

    let quoted_user1 = create_test_user_with_email(&db, "quotedone", "quotedone@example.com", true)
        .await
        .expect("Failed to create quoted user 1");

    let quoted_user2 = create_test_user_with_email(&db, "quotedtwo", "quotedtwo@example.com", true)
        .await
        .expect("Failed to create quoted user 2");

    // Create notification preferences
    create_notification_preferences(&db, quoted_user1.id, "quote", true, false)
        .await
        .expect("Failed to create preferences 1");
    create_notification_preferences(&db, quoted_user2.id, "quote", true, false)
        .await
        .expect("Failed to create preferences 2");

    // Create a thread
    let (_forum, thread) = create_test_forum_and_thread(&db, author.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Test with multiple quotes
    let content = r#"[quote=quotedone]First quote[/quote]
[quote=quotedtwo]Second quote[/quote]
And my response."#;

    ruforo::notifications::dispatcher::detect_and_notify_quotes(
        content,
        1, // post_id
        thread.id,
        author.id,
    )
    .await
    .expect("Failed to detect quotes");

    // Both users should have notifications
    let count1 = notifications::count_unread_notifications(quoted_user1.id)
        .await
        .expect("Failed to count");
    let count2 = notifications::count_unread_notifications(quoted_user2.id)
        .await
        .expect("Failed to count");

    assert_eq!(count1, 1, "First quoted user should have 1 notification");
    assert_eq!(count2, 1, "Second quoted user should have 1 notification");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_quote_detection_no_self_notification() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create user
    let user = create_test_user_with_email(&db, "selfquoter", "selfquoter@example.com", true)
        .await
        .expect("Failed to create user");

    create_notification_preferences(&db, user.id, "quote", true, false)
        .await
        .expect("Failed to create preferences");

    // Create a thread
    let (_forum, thread) = create_test_forum_and_thread(&db, user.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Test self-quote (should not create notification)
    let content = r#"[quote=selfquoter]My own quote[/quote]
Quoting myself."#;

    ruforo::notifications::dispatcher::detect_and_notify_quotes(
        content,
        1, // post_id
        thread.id,
        user.id,
    )
    .await
    .expect("Failed to detect quotes");

    // Verify no notifications were created
    let count = notifications::count_unread_notifications(user.id)
        .await
        .expect("Failed to count notifications");

    assert_eq!(
        count, 0,
        "User should not receive notification for quoting themselves"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_quote_detection_duplicate_quotes_same_user() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test users
    let author = create_test_user_with_email(&db, "dupquoter", "dupquoter@example.com", true)
        .await
        .expect("Failed to create author");

    let quoted_user =
        create_test_user_with_email(&db, "dupquoted", "dupquoted@example.com", true)
            .await
            .expect("Failed to create quoted user");

    // Create notification preferences
    create_notification_preferences(&db, quoted_user.id, "quote", true, false)
        .await
        .expect("Failed to create preferences");

    // Create a thread
    let (_forum, thread) = create_test_forum_and_thread(&db, author.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Test with same user quoted multiple times
    let content = r#"[quote=dupquoted]First quote[/quote]
[quote=dupquoted]Second quote from same user[/quote]
My response."#;

    ruforo::notifications::dispatcher::detect_and_notify_quotes(
        content,
        1, // post_id
        thread.id,
        author.id,
    )
    .await
    .expect("Failed to detect quotes");

    // Should only have 1 notification (deduplicated)
    let count = notifications::count_unread_notifications(quoted_user.id)
        .await
        .expect("Failed to count notifications");

    assert_eq!(
        count, 1,
        "Quoted user should only have 1 notification even when quoted multiple times"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_quote_detection_case_insensitive() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test users
    let author = create_test_user_with_email(&db, "caseauthor", "caseauthor@example.com", true)
        .await
        .expect("Failed to create author");

    let quoted_user =
        create_test_user_with_email(&db, "caseuser", "caseuser@example.com", true)
            .await
            .expect("Failed to create quoted user");

    // Create notification preferences
    create_notification_preferences(&db, quoted_user.id, "quote", true, false)
        .await
        .expect("Failed to create preferences");

    // Create a thread
    let (_forum, thread) = create_test_forum_and_thread(&db, author.id, "Test Thread")
        .await
        .expect("Failed to create thread");

    // Test with uppercase QUOTE tag
    let content = r#"[QUOTE=caseuser]Quoted text[/QUOTE]
My response."#;

    ruforo::notifications::dispatcher::detect_and_notify_quotes(
        content,
        1, // post_id
        thread.id,
        author.id,
    )
    .await
    .expect("Failed to detect quotes");

    // Should still detect the quote
    let count = notifications::count_unread_notifications(quoted_user.id)
        .await
        .expect("Failed to count notifications");

    assert_eq!(
        count, 1,
        "Quote detection should be case-insensitive for the tag"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
