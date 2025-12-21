//! Notification dispatcher for detecting events and sending notifications

use crate::db::get_db_pool;
use crate::notifications::{create_notification, NotificationType};
use crate::orm::{threads, user_names, watched_threads};
use crate::user::Profile;
use once_cell::sync::Lazy;
use regex::Regex;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

static MENTION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"@([a-zA-Z0-9_-]+)").unwrap());

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

    // Look up users and create notifications
    for username in mentioned_usernames {
        // Find user by username (using user_names table for current name)
        let user_name = user_names::Entity::find()
            .filter(user_names::Column::Name.eq(username))
            .one(db)
            .await?;

        if let Some(user_name_rec) = user_name {
            let user_id = user_name_rec.user_id;

            // Don't notify yourself
            if user_id == author_id {
                continue;
            }

            // Get author username
            let author = Profile::get_by_id(db, author_id).await?;
            let author_name = author
                .map(|a| a.name)
                .unwrap_or_else(|| "Someone".to_string());

            // Get thread info
            let thread = threads::Entity::find_by_id(thread_id).one(db).await?;
            let thread_title = thread
                .map(|t| t.title)
                .unwrap_or_else(|| "a thread".to_string());

            create_notification(
                user_id,
                NotificationType::Mention,
                format!("{} mentioned you", author_name),
                format!("You were mentioned in: {}", thread_title),
                Some(format!("/threads/{}#post-{}", thread_id, post_id)),
                Some(author_id),
                Some("post".to_string()),
                Some(post_id),
            )
            .await?;
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
        .map(|a| a.name)
        .unwrap_or_else(|| "Someone".to_string());

    // Notify thread author if they're not the one posting
    if let Some(thread_author_id) = thread.user_id {
        if thread_author_id != author_id {
            create_notification(
                thread_author_id,
                NotificationType::Reply,
                format!("{} replied to your thread", author_name),
                format!("New reply in: {}", thread.title),
                Some(format!("/threads/{}#post-{}", thread_id, post_id)),
                Some(author_id),
                Some("post".to_string()),
                Some(post_id),
            )
            .await?;
        }
    }

    // Notify users watching the thread
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

        create_notification(
            watcher.user_id,
            NotificationType::ThreadWatch,
            format!("{} replied to a watched thread", author_name),
            format!("New reply in: {}", thread.title),
            Some(format!("/threads/{}#post-{}", thread_id, post_id)),
            Some(author_id),
            Some("post".to_string()),
            Some(post_id),
        )
        .await?;
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

    create_notification(
        target_user_id,
        NotificationType::ModAction,
        title,
        message,
        None,
        Some(moderator_id),
        Some("mod_action".to_string()),
        None,
    )
    .await?;

    Ok(())
}
