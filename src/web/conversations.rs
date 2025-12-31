//! Conversation (private messaging) routes

use crate::conversations;
use crate::middleware::ClientCtx;
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use serde::Deserialize;

mod filters {
    pub fn ugc(s: &str) -> ::askama::Result<String> {
        Ok(crate::bbcode::parse(s))
    }
}

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    // Order matters: specific routes before parameterized routes
    conf.service(view_inbox)
        .service(view_archived)
        .service(new_conversation_form)
        .service(create_conversation)
        .service(view_conversation) // Must be after /new and /archived
        .service(send_message_handler)
        .service(edit_message_handler)
        .service(leave_conversation_handler)
        .service(archive_conversation_handler)
        .service(unarchive_conversation_handler);
}

/// Template for inbox (conversation list)
#[derive(Template)]
#[template(path = "conversations/inbox.html")]
struct InboxTemplate {
    client: ClientCtx,
    conversations: Vec<conversations::ConversationPreview>,
    unread_count: i64,
}

/// Template for archived conversations
#[derive(Template)]
#[template(path = "conversations/archived.html")]
struct ArchivedTemplate {
    client: ClientCtx,
    conversations: Vec<conversations::ConversationPreview>,
}

/// Template for conversation view
#[derive(Template)]
#[template(path = "conversations/view.html")]
struct ConversationViewTemplate {
    client: ClientCtx,
    conversation_id: i32,
    messages: Vec<conversations::MessageDisplay>,
    participants: Vec<String>,
    title: Option<String>,
    is_archived: bool,
}

/// Template for new conversation form
#[derive(Template)]
#[template(path = "conversations/new.html")]
struct NewConversationTemplate {
    client: ClientCtx,
}

/// GET /conversations - View inbox with all conversations
#[get("/conversations")]
pub async fn view_inbox(client: ClientCtx) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    // Get user's conversations
    let conversations = conversations::get_user_conversations(user_id, 50)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Get unread count
    let unread_count = conversations::count_unread_conversations(user_id)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(InboxTemplate {
        client,
        conversations,
        unread_count,
    }
    .to_response())
}

/// GET /conversations/{id} - View a specific conversation
#[get("/conversations/{id}")]
pub async fn view_conversation(
    client: ClientCtx,
    conversation_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;
    let conv_id = *conversation_id;

    use crate::orm::conversation_participants;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = crate::db::get_db_pool();

    // Get participant record (verifies participation and gets archived status)
    let user_participant = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conv_id))
        .filter(conversation_participants::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorForbidden("You are not a participant in this conversation"))?;

    let is_archived = user_participant.is_archived;

    // Get messages
    let messages = conversations::get_conversation_messages(conv_id, 100, 0)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Get all participants
    let participants_data = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conv_id))
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let mut participant_names = Vec::new();
    for participant in participants_data {
        use crate::user::Profile;
        if let Ok(Some(profile)) = Profile::get_by_id(db, participant.user_id).await {
            participant_names.push(profile.name);
        }
    }

    // Get conversation title
    use crate::orm::conversations as conv_orm;
    let conversation = conv_orm::Entity::find_by_id(conv_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let title = conversation.and_then(|c| c.title);

    // Mark as read
    conversations::mark_conversation_read(user_id, conv_id)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(ConversationViewTemplate {
        client,
        conversation_id: conv_id,
        messages,
        participants: participant_names,
        title,
        is_archived,
    }
    .to_response())
}

/// GET /conversations/new - Show new conversation form
#[get("/conversations/new")]
pub async fn new_conversation_form(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_login()?;

    Ok(NewConversationTemplate { client }.to_response())
}

/// Form data for creating a new conversation
#[derive(Deserialize)]
pub struct NewConversationForm {
    recipient_usernames: String, // Comma-separated usernames
    title: Option<String>,
    message: String,
}

/// POST /conversations/new - Create a new conversation
#[post("/conversations/new")]
pub async fn create_conversation(
    client: ClientCtx,
    form: web::Form<NewConversationForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    // Parse recipient usernames
    let usernames: Vec<&str> = form
        .recipient_usernames
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if usernames.is_empty() {
        return Err(error::ErrorBadRequest("At least one recipient is required"));
    }

    // Look up user IDs
    use crate::orm::user_names;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = crate::db::get_db_pool();
    let mut recipient_ids = Vec::new();

    for username in usernames {
        let user = user_names::Entity::find()
            .filter(user_names::Column::Name.eq(username))
            .one(db)
            .await
            .map_err(error::ErrorInternalServerError)?;

        if let Some(user_name) = user {
            recipient_ids.push(user_name.user_id);
        } else {
            return Err(error::ErrorBadRequest(format!(
                "User '{}' not found",
                username
            )));
        }
    }

    // Create conversation
    let conversation_id =
        conversations::create_conversation(user_id, &recipient_ids, form.title.as_deref())
            .await
            .map_err(error::ErrorInternalServerError)?;

    // Send first message
    conversations::send_message(conversation_id, user_id, &form.message)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Send notifications to recipients
    for recipient_id in recipient_ids {
        if recipient_id != user_id {
            // Get sender name
            use crate::user::Profile;
            let sender_name = Profile::get_by_id(db, user_id)
                .await
                .ok()
                .flatten()
                .map(|p| p.name)
                .unwrap_or_else(|| "Someone".to_string());

            // Create notification
            let _ = crate::notifications::create_notification(
                recipient_id,
                crate::notifications::NotificationType::PrivateMessage,
                format!("New message from {}", sender_name),
                "You have received a new private message".to_string(),
                Some(format!("/conversations/{}", conversation_id)),
                Some(user_id),
                Some("conversation".to_string()),
                Some(conversation_id),
            )
            .await;
        }
    }

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/conversations/{}", conversation_id)))
        .finish())
}

/// Form data for sending a message
#[derive(Deserialize)]
pub struct SendMessageForm {
    message: String,
}

/// POST /conversations/{id}/send - Send a message in a conversation
#[post("/conversations/{id}/send")]
pub async fn send_message_handler(
    client: ClientCtx,
    conversation_id: web::Path<i32>,
    form: web::Form<SendMessageForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;
    let conv_id = *conversation_id;

    if form.message.trim().is_empty() {
        return Err(error::ErrorBadRequest("Message cannot be empty"));
    }

    // Send message
    conversations::send_message(conv_id, user_id, &form.message)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Get participants to notify
    use crate::orm::conversation_participants;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = crate::db::get_db_pool();
    let participants = conversation_participants::Entity::find()
        .filter(conversation_participants::Column::ConversationId.eq(conv_id))
        .filter(conversation_participants::Column::UserId.ne(user_id))
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Get sender name
    use crate::user::Profile;
    let sender_name = Profile::get_by_id(db, user_id)
        .await
        .ok()
        .flatten()
        .map(|p| p.name)
        .unwrap_or_else(|| "Someone".to_string());

    // Send notifications
    for participant in participants {
        let _ = crate::notifications::create_notification(
            participant.user_id,
            crate::notifications::NotificationType::PrivateMessage,
            format!("New message from {}", sender_name),
            "You have a new message in a conversation".to_string(),
            Some(format!("/conversations/{}", conv_id)),
            Some(user_id),
            Some("message".to_string()),
            None,
        )
        .await;
    }

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/conversations/{}", conv_id)))
        .finish())
}

/// Form data for editing a message
#[derive(Deserialize)]
pub struct EditMessageForm {
    content: String,
}

/// POST /conversations/messages/{id}/edit - Edit a message
#[post("/conversations/messages/{id}/edit")]
pub async fn edit_message_handler(
    client: ClientCtx,
    message_id: web::Path<i32>,
    form: web::Form<EditMessageForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;
    let msg_id = *message_id;

    if form.content.trim().is_empty() {
        return Err(error::ErrorBadRequest("Message cannot be empty"));
    }

    // Get the message to find the conversation ID for redirect
    let message = conversations::get_message(msg_id)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Message not found"))?;

    let conv_id = message.conversation_id;

    // Update the message
    conversations::update_message(msg_id, user_id, &form.content)
        .await
        .map_err(|e| {
            log::error!("Failed to edit message: {}", e);
            error::ErrorForbidden(e.to_string())
        })?;

    log::info!("User {} edited message {}", user_id, msg_id);

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/conversations/{}", conv_id)))
        .finish())
}

/// Form data for leaving a conversation
#[derive(Deserialize)]
pub struct LeaveConversationForm {
    csrf_token: String,
}

/// POST /conversations/{id}/leave - Leave a conversation
#[post("/conversations/{id}/leave")]
pub async fn leave_conversation_handler(
    client: ClientCtx,
    session: actix_session::Session,
    conversation_id: web::Path<i32>,
    form: web::Form<LeaveConversationForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;
    let conv_id = *conversation_id;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&session, &form.csrf_token)?;

    // Leave the conversation
    conversations::leave_conversation(user_id, conv_id)
        .await
        .map_err(|e| {
            log::error!("Failed to leave conversation: {}", e);
            error::ErrorInternalServerError("Failed to leave conversation")
        })?;

    log::info!("User {} left conversation {}", user_id, conv_id);

    // Redirect to inbox
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/conversations"))
        .finish())
}

/// Form data for archiving/unarchiving a conversation
#[derive(Deserialize)]
pub struct ArchiveConversationForm {
    csrf_token: String,
}

/// GET /conversations/archived - View archived conversations
#[get("/conversations/archived")]
pub async fn view_archived(client: ClientCtx) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    // Get user's archived conversations
    let archived = conversations::get_archived_conversations(user_id, 50)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(ArchivedTemplate {
        client,
        conversations: archived,
    }
    .to_response())
}

/// POST /conversations/{id}/archive - Archive a conversation
#[post("/conversations/{id}/archive")]
pub async fn archive_conversation_handler(
    client: ClientCtx,
    session: actix_session::Session,
    conversation_id: web::Path<i32>,
    form: web::Form<ArchiveConversationForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;
    let conv_id = *conversation_id;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&session, &form.csrf_token)?;

    // Archive the conversation
    conversations::archive_conversation(user_id, conv_id)
        .await
        .map_err(|e| {
            log::error!("Failed to archive conversation: {}", e);
            error::ErrorInternalServerError("Failed to archive conversation")
        })?;

    log::info!("User {} archived conversation {}", user_id, conv_id);

    // Redirect to inbox
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/conversations"))
        .finish())
}

/// POST /conversations/{id}/unarchive - Unarchive a conversation
#[post("/conversations/{id}/unarchive")]
pub async fn unarchive_conversation_handler(
    client: ClientCtx,
    session: actix_session::Session,
    conversation_id: web::Path<i32>,
    form: web::Form<ArchiveConversationForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;
    let conv_id = *conversation_id;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&session, &form.csrf_token)?;

    // Unarchive the conversation
    conversations::unarchive_conversation(user_id, conv_id)
        .await
        .map_err(|e| {
            log::error!("Failed to unarchive conversation: {}", e);
            error::ErrorInternalServerError("Failed to unarchive conversation")
        })?;

    log::info!("User {} unarchived conversation {}", user_id, conv_id);

    // Redirect to the conversation
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/conversations/{}", conv_id)))
        .finish())
}
