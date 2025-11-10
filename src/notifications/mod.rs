//! Notification system for user engagement

pub mod dispatcher;
pub mod types;

use crate::db::get_db_pool;
use crate::orm::{notifications, notification_preferences, watched_threads};
use sea_orm::{entity::*, query::*, sea_query::Expr, DbErr, Set};

pub use types::NotificationType;

/// Notification preferences for a user
pub struct NotificationPreferences {
    pub in_app: bool,
    pub email: bool,
    pub frequency: String,
}

/// Create a notification for a user
pub async fn create_notification(
    user_id: i32,
    notification_type: NotificationType,
    title: String,
    message: String,
    url: Option<String>,
    source_user_id: Option<i32>,
    source_content_type: Option<String>,
    source_content_id: Option<i32>,
) -> Result<i32, DbErr> {
    let db = get_db_pool();

    // Check user preferences
    let prefs = get_user_preferences(user_id, &notification_type).await?;

    if !prefs.in_app {
        return Ok(0); // User has disabled this notification type
    }

    // Create notification
    let notification = notifications::ActiveModel {
        user_id: Set(user_id),
        type_: Set(notification_type.as_str().to_string()),
        title: Set(title.clone()),
        message: Set(message.clone()),
        url: Set(url.clone()),
        source_user_id: Set(source_user_id),
        source_content_type: Set(source_content_type),
        source_content_id: Set(source_content_id),
        is_read: Set(false),
        is_emailed: Set(false),
        ..Default::default()
    };

    let result = notification.insert(db).await?;

    // Send email if preferences allow
    if prefs.email && prefs.frequency == "immediate" {
        // Email sending will be handled by a background task or in the dispatcher
        log::info!(
            "Email notification queued for user {} (notification {})",
            user_id,
            result.id
        );
    }

    Ok(result.id)
}

/// Get user's notification preferences for a specific type
pub async fn get_user_preferences(
    user_id: i32,
    notification_type: &NotificationType,
) -> Result<NotificationPreferences, DbErr> {
    let db = get_db_pool();

    let pref = notification_preferences::Entity::find()
        .filter(notification_preferences::Column::UserId.eq(user_id))
        .filter(notification_preferences::Column::NotificationType.eq(notification_type.as_str()))
        .one(db)
        .await?;

    match pref {
        Some(p) => Ok(NotificationPreferences {
            in_app: p.in_app,
            email: p.email,
            frequency: p.frequency,
        }),
        None => {
            // Create default preferences if they don't exist
            let default_pref = notification_preferences::ActiveModel {
                user_id: Set(user_id),
                notification_type: Set(notification_type.as_str().to_string()),
                in_app: Set(true),
                email: Set(true),
                frequency: Set("immediate".to_string()),
            };
            default_pref.insert(db).await?;

            Ok(NotificationPreferences {
                in_app: true,
                email: true,
                frequency: "immediate".to_string(),
            })
        }
    }
}

/// Count unread notifications for a user
pub async fn count_unread_notifications(user_id: i32) -> Result<i64, DbErr> {
    let db = get_db_pool();

    let count = notifications::Entity::find()
        .filter(notifications::Column::UserId.eq(user_id))
        .filter(notifications::Column::IsRead.eq(false))
        .count(db)
        .await?;

    Ok(count as i64)
}

/// Mark a notification as read
pub async fn mark_notification_read(notification_id: i32, user_id: i32) -> Result<(), DbErr> {
    let db = get_db_pool();

    notifications::Entity::update_many()
        .col_expr(notifications::Column::IsRead, Expr::value(true))
        .col_expr(
            notifications::Column::ReadAt,
            Expr::value(chrono::Utc::now().naive_utc()),
        )
        .filter(notifications::Column::Id.eq(notification_id))
        .filter(notifications::Column::UserId.eq(user_id))
        .exec(db)
        .await?;

    Ok(())
}

/// Mark all notifications as read for a user
pub async fn mark_all_read(user_id: i32) -> Result<(), DbErr> {
    let db = get_db_pool();

    notifications::Entity::update_many()
        .col_expr(notifications::Column::IsRead, Expr::value(true))
        .col_expr(
            notifications::Column::ReadAt,
            Expr::value(chrono::Utc::now().naive_utc()),
        )
        .filter(notifications::Column::UserId.eq(user_id))
        .filter(notifications::Column::IsRead.eq(false))
        .exec(db)
        .await?;

    Ok(())
}

/// Fetch recent notifications for a user
pub async fn get_user_notifications(
    user_id: i32,
    limit: u64,
    show_read: bool,
) -> Result<Vec<notifications::Model>, DbErr> {
    let db = get_db_pool();

    let mut query = notifications::Entity::find()
        .filter(notifications::Column::UserId.eq(user_id))
        .order_by_desc(notifications::Column::CreatedAt)
        .limit(limit);

    if !show_read {
        query = query.filter(notifications::Column::IsRead.eq(false));
    }

    query.all(db).await
}

// Thread Watching Functions

/// Add a thread to user's watch list
pub async fn watch_thread(user_id: i32, thread_id: i32) -> Result<(), DbErr> {
    let db = get_db_pool();

    // Check if already watching
    let existing = watched_threads::Entity::find()
        .filter(watched_threads::Column::UserId.eq(user_id))
        .filter(watched_threads::Column::ThreadId.eq(thread_id))
        .one(db)
        .await?;

    if existing.is_some() {
        return Ok(()); // Already watching
    }

    // Create watch record
    let watch = watched_threads::ActiveModel {
        user_id: Set(user_id),
        thread_id: Set(thread_id),
        notify_on_reply: Set(true),
        ..Default::default()
    };

    watch.insert(db).await?;
    Ok(())
}

/// Remove a thread from user's watch list
pub async fn unwatch_thread(user_id: i32, thread_id: i32) -> Result<(), DbErr> {
    let db = get_db_pool();

    watched_threads::Entity::delete_many()
        .filter(watched_threads::Column::UserId.eq(user_id))
        .filter(watched_threads::Column::ThreadId.eq(thread_id))
        .exec(db)
        .await?;

    Ok(())
}

/// Check if a user is watching a thread
pub async fn is_watching_thread(user_id: i32, thread_id: i32) -> Result<bool, DbErr> {
    let db = get_db_pool();

    let watch = watched_threads::Entity::find()
        .filter(watched_threads::Column::UserId.eq(user_id))
        .filter(watched_threads::Column::ThreadId.eq(thread_id))
        .one(db)
        .await?;

    Ok(watch.is_some())
}

/// Get all threads a user is watching
pub async fn get_watched_threads(user_id: i32) -> Result<Vec<i32>, DbErr> {
    let db = get_db_pool();

    let watches = watched_threads::Entity::find()
        .filter(watched_threads::Column::UserId.eq(user_id))
        .all(db)
        .await?;

    Ok(watches.iter().map(|w| w.thread_id).collect())
}

/// Count how many threads a user is watching
pub async fn count_watched_threads(user_id: i32) -> Result<i64, DbErr> {
    let db = get_db_pool();

    let count = watched_threads::Entity::find()
        .filter(watched_threads::Column::UserId.eq(user_id))
        .count(db)
        .await?;

    Ok(count as i64)
}
