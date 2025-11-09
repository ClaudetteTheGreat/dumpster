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
        .service(mark_all_read);
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
