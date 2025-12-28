//! Notification dispatcher for detecting events and sending notifications

use crate::db::get_db_pool;
use crate::notifications::{create_notification, get_user_preferences, NotificationType};
use crate::orm::{threads, ugc, ugc_revisions, user_names, users, watched_threads};
use crate::user::Profile;
use crate::web::notifications_ws::{
    get_notification_server, BroadcastNotification, NotificationData,
};
use once_cell::sync::Lazy;
use regex::Regex;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

static MENTION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"@([a-zA-Z0-9_-]+)").unwrap());

/// Regex to match [quote=username] BBCode tags (case-insensitive)
static QUOTE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\[quote=([a-zA-Z0-9_-]+)\]").unwrap());

/// Get base URL for email links
fn get_base_url() -> String {
    std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// Broadcast a notification via WebSocket if the server is available
fn broadcast_realtime_notification(
    user_id: i32,
    notification_id: i32,
    notification_type: &str,
    title: &str,
    message: &str,
    url: Option<&str>,
) {
    if let Some(server) = get_notification_server() {
        let notification = NotificationData {
            id: notification_id,
            notification_type: notification_type.to_string(),
            title: title.to_string(),
            message: message.to_string(),
            url: url.map(|s| s.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        server.do_send(BroadcastNotification {
            user_id,
            notification,
        });
    }
}

/// Detect mentions in content and create notifications
pub async fn detect_and_notify_mentions(
    content: &str,
    post_id: i32,
    thread_id: i32,
    author_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = get_db_pool();

    // Extract mentioned usernames
    let mentioned_usernames: Vec<&str> = MENTION_REGEX
        .captures_iter(content)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str())
        .collect();

    if mentioned_usernames.is_empty() {
        return Ok(());
    }

    // Get author info once
    let author = Profile::get_by_id(db, author_id).await?;
    let author_name = author
        .map(|a| a.name)
        .unwrap_or_else(|| "Someone".to_string());

    // Get thread info once
    let thread = threads::Entity::find_by_id(thread_id).one(db).await?;
    let thread_title = thread
        .map(|t| t.title)
        .unwrap_or_else(|| "a thread".to_string());

    // Look up users and create notifications
    for username in mentioned_usernames {
        // Find user by username (using user_names table for current name)
        let user_name = user_names::Entity::find()
            .filter(user_names::Column::Name.eq(username))
            .one(db)
            .await?;

        if let Some(user_name_rec) = user_name {
            let mentioned_user_id = user_name_rec.user_id;

            // Don't notify yourself
            if mentioned_user_id == author_id {
                continue;
            }

            // Create in-app notification
            let title = format!("{} mentioned you", author_name);
            let message = format!("You were mentioned in: {}", thread_title);
            let url = format!("/threads/{}#post-{}", thread_id, post_id);

            let notification_id = create_notification(
                mentioned_user_id,
                NotificationType::Mention,
                title.clone(),
                message.clone(),
                Some(url.clone()),
                Some(author_id),
                Some("post".to_string()),
                Some(post_id),
            )
            .await?;

            // Broadcast real-time notification
            if notification_id > 0 {
                broadcast_realtime_notification(
                    mentioned_user_id,
                    notification_id,
                    "mention",
                    &title,
                    &message,
                    Some(&url),
                );
            }

            // Check if user wants email notifications for mentions
            let prefs = get_user_preferences(mentioned_user_id, &NotificationType::Mention).await?;
            if prefs.email && prefs.frequency == "immediate" {
                // Get user's email and check if verified
                if let Some(user) = users::Entity::find_by_id(mentioned_user_id).one(db).await? {
                    if user.email_verified {
                        if let Some(email) = &user.email {
                            // Get recipient username
                            let recipient_name = user_names::Entity::find()
                                .filter(user_names::Column::UserId.eq(mentioned_user_id))
                                .one(db)
                                .await?
                                .map(|un| un.name)
                                .unwrap_or_else(|| "User".to_string());

                            // Send mention email
                            if let Err(e) = crate::email::templates::send_mention_email(
                                email,
                                &recipient_name,
                                &author_name,
                                &thread_title,
                                thread_id,
                                post_id,
                                content,
                                &get_base_url(),
                            )
                            .await
                            {
                                log::error!(
                                    "Failed to send mention email to user {}: {}",
                                    mentioned_user_id,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Detect quotes in content and create notifications for quoted users
pub async fn detect_and_notify_quotes(
    content: &str,
    post_id: i32,
    thread_id: i32,
    author_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = get_db_pool();

    // Extract quoted usernames (deduplicate)
    let quoted_usernames: std::collections::HashSet<String> = QUOTE_REGEX
        .captures_iter(content)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str().to_lowercase())
        .collect();

    if quoted_usernames.is_empty() {
        return Ok(());
    }

    // Get author info once
    let author = Profile::get_by_id(db, author_id).await?;
    let author_name = author
        .map(|a| a.name)
        .unwrap_or_else(|| "Someone".to_string());

    // Get thread info once
    let thread = threads::Entity::find_by_id(thread_id).one(db).await?;
    let thread_title = thread
        .map(|t| t.title)
        .unwrap_or_else(|| "a thread".to_string());

    // Look up users and create notifications
    for username in quoted_usernames {
        // Find user by username (case-insensitive search)
        let user_name = user_names::Entity::find()
            .filter(sea_orm::Condition::all().add(user_names::Column::Name.eq(username.clone())))
            .one(db)
            .await?;

        if let Some(user_name_rec) = user_name {
            let quoted_user_id = user_name_rec.user_id;

            // Don't notify yourself
            if quoted_user_id == author_id {
                continue;
            }

            // Create in-app notification
            let title = format!("{} quoted you", author_name);
            let message = format!("Your post was quoted in: {}", thread_title);
            let url = format!("/threads/{}#post-{}", thread_id, post_id);

            let notification_id = create_notification(
                quoted_user_id,
                NotificationType::Quote,
                title.clone(),
                message.clone(),
                Some(url.clone()),
                Some(author_id),
                Some("post".to_string()),
                Some(post_id),
            )
            .await?;

            // Broadcast real-time notification
            if notification_id > 0 {
                broadcast_realtime_notification(
                    quoted_user_id,
                    notification_id,
                    "quote",
                    &title,
                    &message,
                    Some(&url),
                );
            }

            // Check if user wants email notifications for quotes
            let prefs = get_user_preferences(quoted_user_id, &NotificationType::Quote).await?;
            if prefs.email && prefs.frequency == "immediate" {
                // Get user's email and check if verified
                if let Some(user) = users::Entity::find_by_id(quoted_user_id).one(db).await? {
                    if user.email_verified {
                        if let Some(email) = &user.email {
                            // Get recipient username
                            let recipient_name = user_names::Entity::find()
                                .filter(user_names::Column::UserId.eq(quoted_user_id))
                                .one(db)
                                .await?
                                .map(|un| un.name)
                                .unwrap_or_else(|| "User".to_string());

                            // Send quote email
                            if let Err(e) = crate::email::templates::send_quote_email(
                                email,
                                &recipient_name,
                                &author_name,
                                &thread_title,
                                thread_id,
                                post_id,
                                content,
                                &get_base_url(),
                            )
                            .await
                            {
                                log::error!(
                                    "Failed to send quote email to user {}: {}",
                                    quoted_user_id,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Notify thread author and watchers of a new reply
pub async fn notify_thread_reply(
    thread_id: i32,
    post_id: i32,
    author_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = get_db_pool();

    // Get thread info
    let thread = threads::Entity::find_by_id(thread_id)
        .one(db)
        .await?
        .ok_or("Thread not found")?;

    // Get author username
    let author = Profile::get_by_id(db, author_id).await?;
    let author_name = author
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "Someone".to_string());

    // Get post content for emails
    let post_content = get_post_content(post_id).await.unwrap_or_default();

    // Notify thread author if they're not the one posting
    if let Some(thread_author_id) = thread.user_id {
        if thread_author_id != author_id {
            // Create in-app notification
            let title = format!("{} replied to your thread", author_name);
            let message = format!("New reply in: {}", thread.title);
            let url = format!("/threads/{}#post-{}", thread_id, post_id);

            let notification_id = create_notification(
                thread_author_id,
                NotificationType::Reply,
                title.clone(),
                message.clone(),
                Some(url.clone()),
                Some(author_id),
                Some("post".to_string()),
                Some(post_id),
            )
            .await?;

            // Broadcast real-time notification
            if notification_id > 0 {
                broadcast_realtime_notification(
                    thread_author_id,
                    notification_id,
                    "reply",
                    &title,
                    &message,
                    Some(&url),
                );
            }

            // Send email to thread author if they want it
            let prefs = get_user_preferences(thread_author_id, &NotificationType::Reply).await?;
            if prefs.email && prefs.frequency == "immediate" {
                if let Some(user) = users::Entity::find_by_id(thread_author_id).one(db).await? {
                    if user.email_verified {
                        if let Some(email) = &user.email {
                            // Get recipient username
                            let recipient_name = user_names::Entity::find()
                                .filter(user_names::Column::UserId.eq(thread_author_id))
                                .one(db)
                                .await?
                                .map(|un| un.name)
                                .unwrap_or_else(|| "User".to_string());

                            // Send author reply email
                            if let Err(e) = crate::email::templates::send_author_reply_email(
                                email,
                                &recipient_name,
                                &author_name,
                                &thread.title,
                                thread_id,
                                post_id,
                                &post_content,
                                &get_base_url(),
                            )
                            .await
                            {
                                log::error!(
                                    "Failed to send reply email to thread author {}: {}",
                                    thread_author_id,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Notify users watching the thread (in-app notifications)
    let watchers = watched_threads::Entity::find()
        .filter(watched_threads::Column::ThreadId.eq(thread_id))
        .filter(watched_threads::Column::NotifyOnReply.eq(true))
        .all(db)
        .await?;

    for watcher in watchers {
        // Skip the author and the thread owner (already notified above)
        if watcher.user_id == author_id {
            continue;
        }
        if Some(watcher.user_id) == thread.user_id {
            continue;
        }

        let title = format!("{} replied to a watched thread", author_name);
        let message = format!("New reply in: {}", thread.title);
        let url = format!("/threads/{}#post-{}", thread_id, post_id);

        let notification_id = create_notification(
            watcher.user_id,
            NotificationType::ThreadWatch,
            title.clone(),
            message.clone(),
            Some(url.clone()),
            Some(author_id),
            Some("post".to_string()),
            Some(post_id),
        )
        .await?;

        // Broadcast real-time notification
        if notification_id > 0 {
            broadcast_realtime_notification(
                watcher.user_id,
                notification_id,
                "thread_watch",
                &title,
                &message,
                Some(&url),
            );
        }
    }

    // Send email notifications to watchers with email_on_reply enabled
    send_thread_reply_emails(thread_id, post_id, author_id, &author_name, &thread.title).await?;

    Ok(())
}

/// Get post content from UGC table
async fn get_post_content(post_id: i32) -> Result<String, Box<dyn std::error::Error>> {
    use crate::orm::posts;

    let db = get_db_pool();

    let post = posts::Entity::find_by_id(post_id).one(db).await?;
    if let Some(post) = post {
        if let Some(ugc_model) = ugc::Entity::find_by_id(post.ugc_id).one(db).await? {
            if let Some(rev_id) = ugc_model.ugc_revision_id {
                if let Some(rev) = ugc_revisions::Entity::find_by_id(rev_id).one(db).await? {
                    return Ok(rev.content);
                }
            }
        }
    }
    Ok(String::new())
}

/// Send email notifications to thread watchers
async fn send_thread_reply_emails(
    thread_id: i32,
    post_id: i32,
    author_id: i32,
    author_name: &str,
    thread_title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::orm::posts;

    let db = get_db_pool();

    // Get watchers who want email notifications
    let email_watchers = watched_threads::Entity::find()
        .filter(watched_threads::Column::ThreadId.eq(thread_id))
        .filter(watched_threads::Column::EmailOnReply.eq(true))
        .all(db)
        .await?;

    if email_watchers.is_empty() {
        return Ok(());
    }

    // Get the post content for the email preview
    let post = posts::Entity::find_by_id(post_id).one(db).await?;
    let post_content = if let Some(post) = post {
        // Get UGC content
        if let Some(ugc_model) = ugc::Entity::find_by_id(post.ugc_id).one(db).await? {
            if let Some(rev_id) = ugc_model.ugc_revision_id {
                if let Some(rev) = ugc_revisions::Entity::find_by_id(rev_id).one(db).await? {
                    rev.content
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Get base URL for links
    let base_url =
        std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    for watcher in email_watchers {
        // Skip the author - don't email yourself
        if watcher.user_id == author_id {
            continue;
        }

        // Get user's email and username
        let user = users::Entity::find_by_id(watcher.user_id).one(db).await?;
        if let Some(user) = user {
            // Only send if user has a verified email
            if !user.email_verified {
                continue;
            }

            if let Some(email) = &user.email {
                // Get username
                let username = user_names::Entity::find()
                    .filter(user_names::Column::UserId.eq(watcher.user_id))
                    .one(db)
                    .await?
                    .map(|un| un.name)
                    .unwrap_or_else(|| "User".to_string());

                // Send email (don't block on errors)
                if let Err(e) = crate::email::templates::send_thread_reply_email(
                    email,
                    &username,
                    thread_title,
                    thread_id,
                    author_name,
                    &post_content,
                    &base_url,
                )
                .await
                {
                    log::error!(
                        "Failed to send thread reply email to user {}: {}",
                        watcher.user_id,
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

/// Create a notification for a moderation action
pub async fn notify_moderation_action(
    target_user_id: i32,
    action: &str,
    reason: Option<&str>,
    moderator_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = get_db_pool();

    let moderator = Profile::get_by_id(db, moderator_id).await?;
    let moderator_name = moderator
        .map(|m| m.name)
        .unwrap_or_else(|| "A moderator".to_string());

    let title = format!("Moderation action: {}", action);
    let message = if let Some(r) = reason {
        format!("Action taken by {}: {}", moderator_name, r)
    } else {
        format!("Action taken by {}", moderator_name)
    };

    let notification_id = create_notification(
        target_user_id,
        NotificationType::ModAction,
        title.clone(),
        message.clone(),
        None,
        Some(moderator_id),
        Some("mod_action".to_string()),
        None,
    )
    .await?;

    // Broadcast real-time notification
    if notification_id > 0 {
        broadcast_realtime_notification(
            target_user_id,
            notification_id,
            "mod_action",
            &title,
            &message,
            None,
        );
    }

    Ok(())
}
