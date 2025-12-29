//! Conversation management for private messaging

use crate::db::get_db_pool;
use crate::orm::{conversation_participants, conversations, private_messages, ugc, ugc_revisions};
use crate::ugc::{create_ugc, NewUgcPartial};
use sea_orm::{entity::*, query::*, sea_query::Expr, ActiveValue::Set, DatabaseConnection, DbErr};

/// Create a new conversation with participants
pub async fn create_conversation(
    creator_id: i32,
    participant_ids: &[i32],
    title: Option<&str>,
) -> Result<i32, DbErr> {
    let db = get_db_pool();
    let txn = db.begin().await?;

    // Create conversation
    let conversation = conversations::ActiveModel {
        title: Set(title.map(|s| s.to_string())),
        ..Default::default()
    };
    let conversation_model = conversation.insert(&txn).await?;

    // Add creator as participant
    let creator_participant = conversation_participants::ActiveModel {
        conversation_id: Set(conversation_model.id),
        user_id: Set(creator_id),
        ..Default::default()
    };
    creator_participant.insert(&txn).await?;

    // Add other participants
    for &participant_id in participant_ids {
        if participant_id != creator_id {
            let participant = conversation_participants::ActiveModel {
                conversation_id: Set(conversation_model.id),
                user_id: Set(participant_id),
                ..Default::default()
            };
            participant.insert(&txn).await?;
        }
    }

    txn.commit().await?;

    Ok(conversation_model.id)
}

/// Send a message in a conversation
pub async fn send_message(
    conversation_id: i32,
    sender_id: i32,
    content: &str,
) -> Result<i32, DbErr> {
    let db = get_db_pool();
    let txn = db.begin().await?;

    // Verify sender is a participant
    verify_participant(&txn, sender_id, conversation_id).await?;

    // Create UGC for the message
    let ugc_revision = create_ugc(
        &txn,
        NewUgcPartial {
            ip_id: None,
            user_id: Some(sender_id),
            content,
        },
    )
    .await
    .map_err(|e| DbErr::Custom(format!("Failed to create UGC: {}", e)))?;

    // Create private message
    let message = private_messages::ActiveModel {
        conversation_id: Set(conversation_id),
        ugc_id: Set(ugc_revision.ugc_id),
        user_id: Set(Some(sender_id)),
        created_at: Set(ugc_revision.created_at),
        ..Default::default()
    };
    let message_model = message.insert(&txn).await?;

    // Update conversation updated_at timestamp
    conversations::Entity::update_many()
        .col_expr(
            conversations::Column::UpdatedAt,
            Expr::value(ugc_revision.created_at),
        )
        .filter(conversations::Column::Id.eq(conversation_id))
        .exec(&txn)
        .await?;

    txn.commit().await?;

    Ok(message_model.id)
}

/// Verify that a user is a participant in a conversation
pub async fn verify_participant<C>(db: &C, user_id: i32, conversation_id: i32) -> Result<(), DbErr>
where
    C: sea_orm::ConnectionTrait,
{
    let participant = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .filter(conversation_participants::Column::UserId.eq(user_id))
        .one(db)
        .await?;

    if participant.is_none() {
        return Err(DbErr::Custom(
            "User is not a participant in this conversation".to_string(),
        ));
    }

    Ok(())
}

/// Mark a conversation as read for a user
pub async fn mark_conversation_read(user_id: i32, conversation_id: i32) -> Result<(), DbErr> {
    let db = get_db_pool();

    conversation_participants::Entity::update_many()
        .col_expr(
            conversation_participants::Column::LastReadAt,
            Expr::value(chrono::Utc::now().naive_utc()),
        )
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .filter(conversation_participants::Column::UserId.eq(user_id))
        .exec(db)
        .await?;

    Ok(())
}

/// Count unread conversations for a user
pub async fn count_unread_conversations(user_id: i32) -> Result<i64, DbErr> {
    let db = get_db_pool();

    // Get all non-archived conversation participants with their conversations
    let participants = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::UserId.eq(user_id))
        .filter(conversation_participants::Column::IsArchived.eq(false))
        .find_also_related(conversations::Entity)
        .all(db)
        .await?;

    // Count conversations where updated_at is after last_read_at
    let mut count = 0i64;
    for (participant, conversation) in participants {
        if let Some(conv) = conversation {
            let is_unread = if let Some(last_read) = participant.last_read_at {
                conv.updated_at > last_read
            } else {
                true // Never read
            };
            if is_unread {
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Get list of conversations for a user with preview data
pub async fn get_user_conversations(
    user_id: i32,
    limit: u64,
) -> Result<Vec<ConversationPreview>, DbErr> {
    let db = get_db_pool();

    // Get conversations where user is participant
    let participants = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::UserId.eq(user_id))
        .filter(conversation_participants::Column::IsArchived.eq(false))
        .find_also_related(conversations::Entity)
        .order_by_desc(conversations::Column::UpdatedAt)
        .limit(limit)
        .all(db)
        .await?;

    let mut previews = Vec::new();

    for (participant, conversation) in participants {
        if let Some(conv) = conversation {
            // Get other participants
            let other_participants =
                get_conversation_participants(db, conv.id, Some(user_id)).await?;

            // Get last message
            let last_message = get_last_message(db, conv.id).await?;

            // Check if unread
            let is_unread = if let Some(last_read) = participant.last_read_at {
                conv.updated_at > last_read
            } else {
                true
            };

            // Extract content and timestamp from last_message
            let (last_content, last_timestamp) = match last_message {
                Some((content, timestamp)) => (Some(content), Some(timestamp)),
                None => (None, None),
            };

            previews.push(ConversationPreview {
                id: conv.id,
                title: conv.title,
                participants: other_participants,
                last_message_content: last_content,
                last_message_at: last_timestamp,
                is_unread,
            });
        }
    }

    Ok(previews)
}

/// Conversation preview data for inbox listing
#[derive(Debug, Clone)]
pub struct ConversationPreview {
    pub id: i32,
    pub title: Option<String>,
    pub participants: Vec<String>,
    pub last_message_content: Option<String>,
    pub last_message_at: Option<chrono::NaiveDateTime>,
    pub is_unread: bool,
}

/// Get participant names for a conversation (excluding optional user_id)
async fn get_conversation_participants(
    db: &DatabaseConnection,
    conversation_id: i32,
    exclude_user_id: Option<i32>,
) -> Result<Vec<String>, DbErr> {
    use crate::user::Profile;

    // Get participant user IDs
    let mut query = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id));

    if let Some(exclude_id) = exclude_user_id {
        query = query.filter(conversation_participants::Column::UserId.ne(exclude_id));
    }

    let participants = query.all(db).await?;

    // Fetch names for each participant
    let mut names = Vec::new();
    for participant in participants {
        if let Some(profile) = Profile::get_by_id(db, participant.user_id).await? {
            names.push(profile.name);
        }
    }

    Ok(names)
}

/// Get the last message in a conversation
async fn get_last_message(
    db: &DatabaseConnection,
    conversation_id: i32,
) -> Result<Option<(String, chrono::NaiveDateTime)>, DbErr> {
    // Get the last message
    let message = private_messages::Entity::find()
        .filter(private_messages::Column::ConversationId.eq(conversation_id))
        .order_by_desc(private_messages::Column::CreatedAt)
        .one(db)
        .await?;

    if let Some(msg) = message {
        // Get the UGC content
        let ugc_model = ugc::Entity::find_by_id(msg.ugc_id).one(db).await?;
        if let Some(ugc) = ugc_model {
            if let Some(revision_id) = ugc.ugc_revision_id {
                let revision = ugc_revisions::Entity::find_by_id(revision_id)
                    .one(db)
                    .await?;
                if let Some(rev) = revision {
                    return Ok(Some((rev.content, rev.created_at)));
                }
            }
        }
    }

    Ok(None)
}

/// Get messages for a conversation
pub async fn get_conversation_messages(
    conversation_id: i32,
    limit: u64,
    offset: u64,
) -> Result<Vec<MessageDisplay>, DbErr> {
    use crate::user::Profile;

    let db = get_db_pool();

    // Get all messages for the conversation
    let messages = private_messages::Entity::find()
        .filter(private_messages::Column::ConversationId.eq(conversation_id))
        .order_by_asc(private_messages::Column::CreatedAt)
        .limit(limit)
        .offset(offset)
        .all(db)
        .await?;

    let mut displays = Vec::new();

    for msg in messages {
        // Get UGC content
        let ugc_model = ugc::Entity::find_by_id(msg.ugc_id).one(db).await?;
        if let Some(ugc) = ugc_model {
            if let Some(revision_id) = ugc.ugc_revision_id {
                let revision = ugc_revisions::Entity::find_by_id(revision_id)
                    .one(db)
                    .await?;
                if let Some(rev) = revision {
                    // Get author name
                    let author_name = if let Some(author_id) = msg.user_id {
                        Profile::get_by_id(db, author_id)
                            .await?
                            .map(|p| p.name)
                            .unwrap_or_else(|| "Unknown".to_string())
                    } else {
                        "Unknown".to_string()
                    };

                    displays.push(MessageDisplay {
                        id: msg.id,
                        user_id: msg.user_id,
                        author_name,
                        content: rev.content,
                        created_at: msg.created_at,
                    });
                }
            }
        }
    }

    Ok(displays)
}

/// Message display data for templates
#[derive(Debug, Clone)]
pub struct MessageDisplay {
    pub id: i32,
    pub user_id: Option<i32>,
    pub author_name: String,
    pub content: String,
    pub created_at: chrono::NaiveDateTime,
}

/// Leave a conversation (remove user as participant)
/// If no participants remain, the conversation is deleted
pub async fn leave_conversation(user_id: i32, conversation_id: i32) -> Result<(), DbErr> {
    let db = get_db_pool();
    let txn = db.begin().await?;

    // Verify user is a participant
    verify_participant(&txn, user_id, conversation_id).await?;

    // Delete the participant record
    conversation_participants::Entity::delete_many()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .filter(conversation_participants::Column::UserId.eq(user_id))
        .exec(&txn)
        .await?;

    // Check if any participants remain
    let remaining = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conversation_id))
        .count(&txn)
        .await?;

    // If no participants remain, delete the conversation (cascade will delete messages)
    if remaining == 0 {
        conversations::Entity::delete_by_id(conversation_id)
            .exec(&txn)
            .await?;
    }

    txn.commit().await?;

    Ok(())
}
