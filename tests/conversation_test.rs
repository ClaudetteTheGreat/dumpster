/// Integration tests for private messaging (conversation) functionality
/// Tests conversation creation, message sending, read tracking, and participant management
mod common;
use serial_test::serial;

use common::{database::*, fixtures::*};
use dumpster::conversations;
use dumpster::orm::{conversation_participants, conversations as conversation_orm, private_messages};
use sea_orm::{entity::*, DbErr, QueryFilter};

#[actix_rt::test]
#[serial]
async fn test_create_conversation() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create test users
    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let user3 = create_test_user_with_email(&db, "charlie", "charlie@example.com", true)
        .await
        .expect("Failed to create charlie");

    // Create conversation with participants
    let conversation_id = conversations::create_conversation(
        user1.id,
        &[user2.id, user3.id],
        Some("Team Discussion"),
    )
    .await
    .expect("Failed to create conversation");

    // Verify conversation was created
    let conversation = conversation_orm::Entity::find_by_id(conversation_id)
        .one(&db)
        .await
        .expect("Failed to find conversation");

    assert!(conversation.is_some());
    let conv = conversation.unwrap();
    assert_eq!(conv.title, Some("Team Discussion".to_string()));

    // Verify all three users are participants
    let participants = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .all(&db)
        .await
        .expect("Failed to find participants");

    assert_eq!(participants.len(), 3);
    let participant_ids: Vec<i32> = participants.iter().map(|p| p.user_id).collect();
    assert!(participant_ids.contains(&user1.id));
    assert!(participant_ids.contains(&user2.id));
    assert!(participant_ids.contains(&user3.id));
}

#[actix_rt::test]
#[serial]
async fn test_create_conversation_duplicate_participants() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    // Create conversation - user1 is creator and also in participant list
    let conversation_id = conversations::create_conversation(
        user1.id,
        &[user1.id, user2.id], // user1 appears twice
        None,
    )
    .await
    .expect("Failed to create conversation");

    // Verify only 2 participants (not 3)
    let participants = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .all(&db)
        .await
        .expect("Failed to find participants");

    assert_eq!(participants.len(), 2);
}

#[actix_rt::test]
#[serial]
async fn test_send_message() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let conversation_id = conversations::create_conversation(user1.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation");

    // Send a message
    let message_id =
        conversations::send_message(conversation_id, user1.id, "Hello Bob! How are you?")
            .await
            .expect("Failed to send message");

    // Verify message was created
    let message = private_messages::Entity::find_by_id(message_id)
        .one(&db)
        .await
        .expect("Failed to find message")
        .expect("Message not found");

    assert_eq!(message.conversation_id, conversation_id);
    assert_eq!(message.user_id, Some(user1.id));

    // Verify conversation updated_at was updated
    let conversation = conversation_orm::Entity::find_by_id(conversation_id)
        .one(&db)
        .await
        .expect("Failed to find conversation")
        .expect("Conversation not found");

    assert!(conversation.updated_at >= message.created_at);
}

#[actix_rt::test]
#[serial]
async fn test_send_message_non_participant() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let user3 = create_test_user_with_email(&db, "charlie", "charlie@example.com", true)
        .await
        .expect("Failed to create charlie");

    let conversation_id = conversations::create_conversation(user1.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation");

    // Try to send message as non-participant (user3)
    let result = conversations::send_message(
        conversation_id,
        user3.id,
        "I shouldn't be able to send this",
    )
    .await;

    // Should fail
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, DbErr::Custom(_)));
}

#[actix_rt::test]
#[serial]
async fn test_mark_conversation_read() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let conversation_id = conversations::create_conversation(user1.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation");

    // Send a message
    conversations::send_message(conversation_id, user1.id, "Test message")
        .await
        .expect("Failed to send message");

    // Get participant record before marking as read
    let participant_before = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .filter(conversation_participants::Column::UserId.eq(user2.id))
        .one(&db)
        .await
        .expect("Failed to find participant")
        .expect("Participant not found");

    assert!(participant_before.last_read_at.is_none());

    // Mark conversation as read for user2
    conversations::mark_conversation_read(user2.id, conversation_id)
        .await
        .expect("Failed to mark as read");

    // Verify last_read_at was updated
    let participant_after = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .filter(conversation_participants::Column::UserId.eq(user2.id))
        .one(&db)
        .await
        .expect("Failed to find participant")
        .expect("Participant not found");

    assert!(participant_after.last_read_at.is_some());
}

#[actix_rt::test]
#[serial]
async fn test_count_unread_conversations() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let user3 = create_test_user_with_email(&db, "charlie", "charlie@example.com", true)
        .await
        .expect("Failed to create charlie");

    // Create two conversations
    let conv1 = conversations::create_conversation(user1.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation 1");

    let conv2 = conversations::create_conversation(user3.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation 2");

    // Send messages in both
    conversations::send_message(conv1, user1.id, "Message 1")
        .await
        .expect("Failed to send message 1");

    conversations::send_message(conv2, user3.id, "Message 2")
        .await
        .expect("Failed to send message 2");

    // User2 should have 2 unread conversations
    let unread_count = conversations::count_unread_conversations(user2.id)
        .await
        .expect("Failed to count unread");

    assert_eq!(unread_count, 2);

    // Mark one conversation as read
    conversations::mark_conversation_read(user2.id, conv1)
        .await
        .expect("Failed to mark as read");

    // User2 should now have 1 unread conversation
    let unread_count_after = conversations::count_unread_conversations(user2.id)
        .await
        .expect("Failed to count unread");

    assert_eq!(unread_count_after, 1);
}

#[actix_rt::test]
#[serial]
async fn test_get_user_conversations() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let user3 = create_test_user_with_email(&db, "charlie", "charlie@example.com", true)
        .await
        .expect("Failed to create charlie");

    // Create conversations
    let conv1 = conversations::create_conversation(user1.id, &[user2.id], Some("Conv 1"))
        .await
        .expect("Failed to create conversation 1");

    let conv2 = conversations::create_conversation(user2.id, &[user3.id], Some("Conv 2"))
        .await
        .expect("Failed to create conversation 2");

    // Send messages
    conversations::send_message(conv1, user1.id, "Hello Bob!")
        .await
        .expect("Failed to send message 1");

    conversations::send_message(conv2, user2.id, "Hello Charlie!")
        .await
        .expect("Failed to send message 2");

    // Get conversations for user2 (should have both)
    let user2_conversations = conversations::get_user_conversations(user2.id, 10)
        .await
        .expect("Failed to get conversations");

    assert_eq!(user2_conversations.len(), 2);

    // Verify conversation details
    let conv1_preview = user2_conversations
        .iter()
        .find(|c| c.title == Some("Conv 1".to_string()));
    assert!(conv1_preview.is_some());
    let preview = conv1_preview.unwrap();
    assert_eq!(preview.participants, vec!["alice".to_string()]);
    assert_eq!(preview.last_message_content, Some("Hello Bob!".to_string()));
    assert!(preview.is_unread);
}

#[actix_rt::test]
#[serial]
async fn test_get_conversation_messages() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let conversation_id = conversations::create_conversation(user1.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation");

    // Send multiple messages
    conversations::send_message(conversation_id, user1.id, "Message 1")
        .await
        .expect("Failed to send message 1");

    conversations::send_message(conversation_id, user2.id, "Message 2")
        .await
        .expect("Failed to send message 2");

    conversations::send_message(conversation_id, user1.id, "Message 3")
        .await
        .expect("Failed to send message 3");

    // Get messages
    let messages = conversations::get_conversation_messages(conversation_id, 10, 0)
        .await
        .expect("Failed to get messages");

    assert_eq!(messages.len(), 3);

    // Verify order (should be ascending by created_at)
    assert_eq!(messages[0].content, "Message 1");
    assert_eq!(messages[0].author_name, "alice");
    assert_eq!(messages[0].user_id, Some(user1.id));

    assert_eq!(messages[1].content, "Message 2");
    assert_eq!(messages[1].author_name, "bob");
    assert_eq!(messages[1].user_id, Some(user2.id));

    assert_eq!(messages[2].content, "Message 3");
    assert_eq!(messages[2].author_name, "alice");
}

#[actix_rt::test]
#[serial]
async fn test_get_conversation_messages_pagination() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let conversation_id = conversations::create_conversation(user1.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation");

    // Send 5 messages
    for i in 1..=5 {
        conversations::send_message(conversation_id, user1.id, &format!("Message {}", i))
            .await
            .expect("Failed to send message");
    }

    // Get first 2 messages
    let messages_page1 = conversations::get_conversation_messages(conversation_id, 2, 0)
        .await
        .expect("Failed to get messages page 1");

    assert_eq!(messages_page1.len(), 2);
    assert_eq!(messages_page1[0].content, "Message 1");
    assert_eq!(messages_page1[1].content, "Message 2");

    // Get next 2 messages
    let messages_page2 = conversations::get_conversation_messages(conversation_id, 2, 2)
        .await
        .expect("Failed to get messages page 2");

    assert_eq!(messages_page2.len(), 2);
    assert_eq!(messages_page2[0].content, "Message 3");
    assert_eq!(messages_page2[1].content, "Message 4");

    // Get last message
    let messages_page3 = conversations::get_conversation_messages(conversation_id, 2, 4)
        .await
        .expect("Failed to get messages page 3");

    assert_eq!(messages_page3.len(), 1);
    assert_eq!(messages_page3[0].content, "Message 5");
}

#[actix_rt::test]
#[serial]
async fn test_verify_participant() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let user3 = create_test_user_with_email(&db, "charlie", "charlie@example.com", true)
        .await
        .expect("Failed to create charlie");

    let conversation_id = conversations::create_conversation(user1.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation");

    // user1 and user2 should be participants
    let result1 = conversations::verify_participant(&db, user1.id, conversation_id).await;
    assert!(result1.is_ok());

    let result2 = conversations::verify_participant(&db, user2.id, conversation_id).await;
    assert!(result2.is_ok());

    // user3 should not be a participant
    let result3 = conversations::verify_participant(&db, user3.id, conversation_id).await;
    assert!(result3.is_err());
}

#[actix_rt::test]
#[serial]
async fn test_leave_conversation() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let user3 = create_test_user_with_email(&db, "charlie", "charlie@example.com", true)
        .await
        .expect("Failed to create charlie");

    let conversation_id = conversations::create_conversation(user1.id, &[user2.id, user3.id], None)
        .await
        .expect("Failed to create conversation");

    // Verify all three are participants
    let participants_before = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .all(&db)
        .await
        .expect("Failed to find participants");
    assert_eq!(participants_before.len(), 3);

    // User2 leaves the conversation
    conversations::leave_conversation(user2.id, conversation_id)
        .await
        .expect("Failed to leave conversation");

    // Verify user2 is no longer a participant
    let participants_after = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .all(&db)
        .await
        .expect("Failed to find participants");
    assert_eq!(participants_after.len(), 2);

    let participant_ids: Vec<i32> = participants_after.iter().map(|p| p.user_id).collect();
    assert!(participant_ids.contains(&user1.id));
    assert!(!participant_ids.contains(&user2.id));
    assert!(participant_ids.contains(&user3.id));

    // Conversation should still exist
    let conversation = conversation_orm::Entity::find_by_id(conversation_id)
        .one(&db)
        .await
        .expect("Failed to find conversation");
    assert!(conversation.is_some());
}

#[actix_rt::test]
#[serial]
async fn test_leave_conversation_deletes_when_empty() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let conversation_id = conversations::create_conversation(user1.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation");

    // Both users leave the conversation
    conversations::leave_conversation(user1.id, conversation_id)
        .await
        .expect("Failed to leave conversation");

    conversations::leave_conversation(user2.id, conversation_id)
        .await
        .expect("Failed to leave conversation");

    // Conversation should be deleted
    let conversation = conversation_orm::Entity::find_by_id(conversation_id)
        .one(&db)
        .await
        .expect("Failed to find conversation");
    assert!(conversation.is_none());
}

#[actix_rt::test]
#[serial]
async fn test_leave_conversation_non_participant() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let user3 = create_test_user_with_email(&db, "charlie", "charlie@example.com", true)
        .await
        .expect("Failed to create charlie");

    let conversation_id = conversations::create_conversation(user1.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation");

    // User3 (non-participant) tries to leave
    let result = conversations::leave_conversation(user3.id, conversation_id).await;
    assert!(result.is_err());
}

#[actix_rt::test]
#[serial]
async fn test_archive_conversation() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let conversation_id = conversations::create_conversation(user1.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation");

    // Verify conversation is not archived initially
    let participant_before = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .filter(conversation_participants::Column::UserId.eq(user1.id))
        .one(&db)
        .await
        .expect("Failed to find participant")
        .expect("Participant not found");
    assert!(!participant_before.is_archived);

    // Archive the conversation for user1
    conversations::archive_conversation(user1.id, conversation_id)
        .await
        .expect("Failed to archive conversation");

    // Verify it's now archived for user1
    let participant_after = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .filter(conversation_participants::Column::UserId.eq(user1.id))
        .one(&db)
        .await
        .expect("Failed to find participant")
        .expect("Participant not found");
    assert!(participant_after.is_archived);

    // Verify it's NOT archived for user2
    let user2_participant = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .filter(conversation_participants::Column::UserId.eq(user2.id))
        .one(&db)
        .await
        .expect("Failed to find participant")
        .expect("Participant not found");
    assert!(!user2_participant.is_archived);
}

#[actix_rt::test]
#[serial]
async fn test_unarchive_conversation() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    let conversation_id = conversations::create_conversation(user1.id, &[user2.id], None)
        .await
        .expect("Failed to create conversation");

    // Archive then unarchive
    conversations::archive_conversation(user1.id, conversation_id)
        .await
        .expect("Failed to archive conversation");

    conversations::unarchive_conversation(user1.id, conversation_id)
        .await
        .expect("Failed to unarchive conversation");

    // Verify it's no longer archived
    let participant = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .filter(conversation_participants::Column::UserId.eq(user1.id))
        .one(&db)
        .await
        .expect("Failed to find participant")
        .expect("Participant not found");
    assert!(!participant.is_archived);
}

#[actix_rt::test]
#[serial]
async fn test_get_archived_conversations() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    // Create two conversations
    let conv1 = conversations::create_conversation(user1.id, &[user2.id], Some("Conv 1"))
        .await
        .expect("Failed to create conversation 1");

    let _conv2 = conversations::create_conversation(user1.id, &[user2.id], Some("Conv 2"))
        .await
        .expect("Failed to create conversation 2");

    // Archive only conv1
    conversations::archive_conversation(user1.id, conv1)
        .await
        .expect("Failed to archive conversation");

    // Get user's non-archived conversations
    let active_convs = conversations::get_user_conversations(user1.id, 10)
        .await
        .expect("Failed to get conversations");
    assert_eq!(active_convs.len(), 1);
    assert_eq!(active_convs[0].title, Some("Conv 2".to_string()));

    // Get user's archived conversations
    let archived_convs = conversations::get_archived_conversations(user1.id, 10)
        .await
        .expect("Failed to get archived conversations");
    assert_eq!(archived_convs.len(), 1);
    assert_eq!(archived_convs[0].title, Some("Conv 1".to_string()));
}

/// Test that sending a message does not mark the conversation as unread for the sender
/// but does mark it as unread for other participants
#[actix_rt::test]
#[serial]
async fn test_send_message_does_not_mark_sender_unread() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let user1 = create_test_user_with_email(&db, "alice", "alice@example.com", true)
        .await
        .expect("Failed to create alice");

    let user2 = create_test_user_with_email(&db, "bob", "bob@example.com", true)
        .await
        .expect("Failed to create bob");

    // Create conversation
    let conversation_id = conversations::create_conversation(user1.id, &[user2.id], Some("Test"))
        .await
        .expect("Failed to create conversation");

    // Mark as read for both users initially
    conversations::mark_conversation_read(user1.id, conversation_id)
        .await
        .expect("Failed to mark read for user1");
    conversations::mark_conversation_read(user2.id, conversation_id)
        .await
        .expect("Failed to mark read for user2");

    // Verify both users have 0 unread conversations
    let user1_unread_before = conversations::count_unread_conversations(user1.id)
        .await
        .expect("Failed to count unread");
    let user2_unread_before = conversations::count_unread_conversations(user2.id)
        .await
        .expect("Failed to count unread");
    assert_eq!(user1_unread_before, 0, "User1 should have 0 unread before");
    assert_eq!(user2_unread_before, 0, "User2 should have 0 unread before");

    // User1 sends a message
    conversations::send_message(conversation_id, user1.id, "Hello from Alice!")
        .await
        .expect("Failed to send message");

    // User1 (sender) should still have 0 unread
    let user1_unread_after = conversations::count_unread_conversations(user1.id)
        .await
        .expect("Failed to count unread");
    assert_eq!(
        user1_unread_after, 0,
        "Sender should NOT have unread conversation after sending"
    );

    // User2 (recipient) should have 1 unread
    let user2_unread_after = conversations::count_unread_conversations(user2.id)
        .await
        .expect("Failed to count unread");
    assert_eq!(
        user2_unread_after, 1,
        "Recipient should have 1 unread conversation after message received"
    );
}
