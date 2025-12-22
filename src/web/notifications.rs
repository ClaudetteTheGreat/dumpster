/// Notification management routes
///
/// This module provides routes for viewing and managing notifications.
use crate::middleware::ClientCtx;
use crate::notifications;
use crate::orm::notifications as notification_orm;
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use serde::Deserialize;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(view_notifications)
        .service(mark_read)
        .service(mark_all_read)
        .service(watch_thread)
        .service(unwatch_thread)
        .service(view_watched_threads)
        .service(view_preferences)
        .service(update_preferences);
}

/// Template for notification list
#[derive(Template)]
#[template(path = "notifications.html")]
struct NotificationsTemplate {
    client: ClientCtx,
    notifications: Vec<NotificationDisplay>,
    unread_count: i64,
}

/// Notification display struct for templates
#[derive(Debug)]
#[allow(dead_code)]
struct NotificationDisplay {
    id: i32,
    title: String,
    message: String,
    url: Option<String>,
    is_read: bool,
    created_at: chrono::NaiveDateTime,
    notification_type: String,
}

impl From<notification_orm::Model> for NotificationDisplay {
    fn from(n: notification_orm::Model) -> Self {
        Self {
            id: n.id,
            title: n.title,
            message: n.message,
            url: n.url,
            is_read: n.is_read,
            created_at: n.created_at,
            notification_type: n.type_,
        }
    }
}

/// Query parameters for notification list
#[derive(Deserialize)]
struct NotificationQuery {
    show_read: Option<bool>,
}

/// GET /notifications - View notification list
#[get("/notifications")]
pub async fn view_notifications(
    client: ClientCtx,
    query: web::Query<NotificationQuery>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    let show_read = query.show_read.unwrap_or(false);

    // Fetch notifications
    let notifications = notifications::get_user_notifications(user_id, 50, show_read)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Get unread count
    let unread_count = notifications::count_unread_notifications(user_id)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let notification_displays: Vec<NotificationDisplay> =
        notifications.into_iter().map(Into::into).collect();

    Ok(NotificationsTemplate {
        client,
        notifications: notification_displays,
        unread_count,
    }
    .to_response())
}

/// POST /notifications/{id}/read - Mark a notification as read
#[post("/notifications/{id}/read")]
pub async fn mark_read(
    client: ClientCtx,
    notification_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    notifications::mark_notification_read(*notification_id, user_id)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true
    })))
}

/// POST /notifications/mark-all-read - Mark all notifications as read
#[post("/notifications/mark-all-read")]
pub async fn mark_all_read(client: ClientCtx) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    notifications::mark_all_read(user_id)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", "/notifications"))
        .finish())
}

// Thread Watching Routes

/// POST /threads/{id}/watch - Watch a thread
#[post("/threads/{id}/watch")]
pub async fn watch_thread(
    client: ClientCtx,
    thread_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    notifications::watch_thread(user_id, *thread_id)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Redirect back to the thread
    Ok(HttpResponse::Found()
        .append_header(("Location", format!("/threads/{}", *thread_id)))
        .finish())
}

/// POST /threads/{id}/unwatch - Unwatch a thread
#[post("/threads/{id}/unwatch")]
pub async fn unwatch_thread(
    client: ClientCtx,
    thread_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    notifications::unwatch_thread(user_id, *thread_id)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Redirect back to the thread
    Ok(HttpResponse::Found()
        .append_header(("Location", format!("/threads/{}", *thread_id)))
        .finish())
}

/// Template for watched threads page
#[derive(Template)]
#[template(path = "watched_threads.html")]
struct WatchedThreadsTemplate {
    client: ClientCtx,
    threads: Vec<WatchedThreadDisplay>,
}

/// Display struct for watched thread
#[derive(Debug)]
struct WatchedThreadDisplay {
    id: i32,
    title: String,
    forum_id: i32,
    forum_name: String,
    reply_count: i32,
    last_post_at: Option<chrono::NaiveDateTime>,
}

/// GET /watched-threads - View all watched threads
#[get("/watched-threads")]
pub async fn view_watched_threads(client: ClientCtx) -> Result<impl Responder, Error> {
    use crate::orm::{forums, threads};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let user_id = client.require_login()?;
    let db = crate::db::get_db_pool();

    // Get thread IDs that user is watching
    let watched_thread_ids = notifications::get_watched_threads(user_id)
        .await
        .map_err(error::ErrorInternalServerError)?;

    if watched_thread_ids.is_empty() {
        return Ok(WatchedThreadsTemplate {
            client,
            threads: vec![],
        }
        .to_response());
    }

    // Fetch thread details
    let threads_result = threads::Entity::find()
        .filter(threads::Column::Id.is_in(watched_thread_ids))
        .find_also_related(forums::Entity)
        .order_by_desc(threads::Column::LastPostAt)
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let thread_displays: Vec<WatchedThreadDisplay> = threads_result
        .into_iter()
        .filter_map(|(thread, forum)| {
            Some(WatchedThreadDisplay {
                id: thread.id,
                title: thread.title,
                forum_id: thread.forum_id,
                forum_name: forum?.label,
                reply_count: thread.post_count,
                last_post_at: thread.last_post_at,
            })
        })
        .collect();

    Ok(WatchedThreadsTemplate {
        client,
        threads: thread_displays,
    }
    .to_response())
}

// Notification Preference Routes

/// Template for notification preferences page
#[derive(Template)]
#[template(path = "notification_preferences.html")]
struct NotificationPreferencesTemplate {
    client: ClientCtx,
    preferences: Vec<notifications::NotificationPreferenceDisplay>,
}

/// Form data for updating preferences
#[derive(Deserialize)]
struct PreferenceUpdateForm {
    notification_type: String,
    in_app: Option<String>,
    email: Option<String>,
    frequency: String,
}

/// GET /notifications/preferences - View notification preferences
#[get("/notifications/preferences")]
pub async fn view_preferences(client: ClientCtx) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    let preferences = notifications::get_all_user_preferences(user_id)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(NotificationPreferencesTemplate {
        client,
        preferences,
    }
    .to_response())
}

/// POST /notifications/preferences - Update notification preferences
#[post("/notifications/preferences")]
pub async fn update_preferences(
    client: ClientCtx,
    form: web::Form<PreferenceUpdateForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    // Convert checkbox values (Some("on") or None) to boolean
    let in_app = form.in_app.is_some();
    let email = form.email.is_some();

    notifications::update_preference(
        user_id,
        &form.notification_type,
        in_app,
        email,
        &form.frequency,
    )
    .await
    .map_err(error::ErrorInternalServerError)?;

    // Redirect back to preferences page
    Ok(HttpResponse::Found()
        .append_header(("Location", "/notifications/preferences"))
        .finish())
}
