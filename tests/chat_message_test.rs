/// Tests for chat message creation and retrieval
mod common;

use serial_test::serial;

#[actix_rt::test]
#[serial]
async fn test_create_chat_message() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::{create_test_chat_message, create_test_chat_room, create_test_user};
    use ruforo::orm::chat_messages;
    use sea_orm::EntityTrait;

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    // Create user and room
    let user = create_test_user(&db, "chat_msg_user", "password123")
        .await
        .unwrap();
    let room = create_test_chat_room(&db, "Test Chat Room").await.unwrap();

    // Create a message
    let message = create_test_chat_message(&db, room.id, user.id, "Hello, world!")
        .await
        .unwrap();

    // Verify the message exists and has correct data
    assert_eq!(message.chat_room_id, room.id);
    assert_eq!(message.user_id, Some(user.id));

    // Verify we can fetch it back
    let fetched = chat_messages::Entity::find_by_id(message.id)
        .one(&db)
        .await
        .unwrap();

    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.chat_room_id, room.id);
    assert_eq!(fetched.user_id, Some(user.id));

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_chat_message_has_valid_timestamps() {
    use chrono::{Duration, Utc};
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::{create_test_chat_message, create_test_chat_room, create_test_user};
    use ruforo::orm::chat_messages;
    use sea_orm::EntityTrait;

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    let user = create_test_user(&db, "timestamp_user", "password123")
        .await
        .unwrap();
    let room = create_test_chat_room(&db, "Timestamp Room").await.unwrap();

    let before = Utc::now().naive_utc();
    let message = create_test_chat_message(&db, room.id, user.id, "Testing timestamps")
        .await
        .unwrap();
    let after = Utc::now().naive_utc();

    // Fetch the message
    let fetched = chat_messages::Entity::find_by_id(message.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    // Verify timestamp is within expected range
    assert!(
        fetched.created_at >= before - Duration::seconds(1),
        "Timestamp should be after test start"
    );
    assert!(
        fetched.created_at <= after + Duration::seconds(1),
        "Timestamp should be before test end"
    );

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_chat_message_content_stored() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::{create_test_chat_message, create_test_chat_room, create_test_user};
    use ruforo::orm::{chat_messages, ugc_revisions};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    let user = create_test_user(&db, "content_user", "password123")
        .await
        .unwrap();
    let room = create_test_chat_room(&db, "Content Room").await.unwrap();

    let test_content = "This is a test message with [b]BBCode[/b]";
    let message = create_test_chat_message(&db, room.id, user.id, test_content)
        .await
        .unwrap();

    // Get the message and its UGC content
    let fetched = chat_messages::Entity::find_by_id(message.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    // Get the UGC revision to check content
    let revision = ugc_revisions::Entity::find()
        .filter(ruforo::orm::ugc_revisions::Column::UgcId.eq(fetched.ugc_id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(revision.content, test_content);

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_chat_message_user_ownership() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::{create_test_chat_message, create_test_chat_room, create_test_user};
    use ruforo::orm::chat_messages;
    use sea_orm::EntityTrait;

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    // Create two users
    let user1 = create_test_user(&db, "owner_user1", "password123")
        .await
        .unwrap();
    let user2 = create_test_user(&db, "other_user2", "password123")
        .await
        .unwrap();
    let room = create_test_chat_room(&db, "Ownership Room").await.unwrap();

    // User 1 creates a message
    let message = create_test_chat_message(&db, room.id, user1.id, "User 1's message")
        .await
        .unwrap();

    // Verify ownership
    let fetched = chat_messages::Entity::find_by_id(message.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        fetched.user_id,
        Some(user1.id),
        "Message should belong to user 1"
    );
    assert_ne!(
        fetched.user_id,
        Some(user2.id),
        "Message should not belong to user 2"
    );

    cleanup_test_data(&db).await.unwrap();
}
