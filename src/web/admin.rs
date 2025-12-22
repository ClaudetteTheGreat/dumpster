/// Administration and moderation tools
///
/// This module provides endpoints for moderators and administrators.
use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{mod_log, threads};
use actix_web::{error, post, web, Error, HttpResponse, Responder};
use sea_orm::{entity::*, ActiveValue::Set, DatabaseConnection};
use serde::Deserialize;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(lock_thread)
        .service(unlock_thread)
        .service(pin_thread)
        .service(unpin_thread);
}

#[derive(Deserialize)]
struct ModerationForm {
    csrf_token: String,
    reason: Option<String>,
}

/// POST /admin/threads/{id}/lock - Lock a thread
#[post("/admin/threads/{id}/lock")]
pub async fn lock_thread(
    client: ClientCtx,
    cookies: actix_session::Session,
    thread_id: web::Path<i32>,
    form: web::Form<ModerationForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Check moderation permission
    client.require_permission("moderate.thread.lock")?;

    let db = get_db_pool();
    let thread_id = thread_id.into_inner();

    // Lock the thread
    let thread = threads::Entity::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find thread: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Thread not found"))?;

    let mut active_thread: threads::ActiveModel = thread.into();
    active_thread.is_locked = Set(true);
    active_thread.update(db).await.map_err(|e| {
        log::error!("Failed to lock thread: {}", e);
        error::ErrorInternalServerError("Failed to lock thread")
    })?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "lock_thread",
        "thread",
        thread_id,
        form.reason.as_deref(),
    )
    .await?;

    log::info!("Thread {} locked by moderator {}", thread_id, moderator_id);

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/threads/{}", thread_id)))
        .finish())
}

/// POST /admin/threads/{id}/unlock - Unlock a thread
#[post("/admin/threads/{id}/unlock")]
pub async fn unlock_thread(
    client: ClientCtx,
    cookies: actix_session::Session,
    thread_id: web::Path<i32>,
    form: web::Form<ModerationForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Check moderation permission
    client.require_permission("moderate.thread.unlock")?;

    let db = get_db_pool();
    let thread_id = thread_id.into_inner();

    // Unlock the thread
    let thread = threads::Entity::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find thread: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Thread not found"))?;

    let mut active_thread: threads::ActiveModel = thread.into();
    active_thread.is_locked = Set(false);
    active_thread.update(db).await.map_err(|e| {
        log::error!("Failed to unlock thread: {}", e);
        error::ErrorInternalServerError("Failed to unlock thread")
    })?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "unlock_thread",
        "thread",
        thread_id,
        form.reason.as_deref(),
    )
    .await?;

    log::info!(
        "Thread {} unlocked by moderator {}",
        thread_id,
        moderator_id
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/threads/{}", thread_id)))
        .finish())
}

/// POST /admin/threads/{id}/pin - Pin a thread
#[post("/admin/threads/{id}/pin")]
pub async fn pin_thread(
    client: ClientCtx,
    cookies: actix_session::Session,
    thread_id: web::Path<i32>,
    form: web::Form<ModerationForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Check moderation permission
    client.require_permission("moderate.thread.pin")?;

    let db = get_db_pool();
    let thread_id = thread_id.into_inner();

    // Pin the thread
    let thread = threads::Entity::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find thread: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Thread not found"))?;

    let mut active_thread: threads::ActiveModel = thread.into();
    active_thread.is_pinned = Set(true);
    active_thread.update(db).await.map_err(|e| {
        log::error!("Failed to pin thread: {}", e);
        error::ErrorInternalServerError("Failed to pin thread")
    })?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "pin_thread",
        "thread",
        thread_id,
        form.reason.as_deref(),
    )
    .await?;

    log::info!("Thread {} pinned by moderator {}", thread_id, moderator_id);

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/threads/{}", thread_id)))
        .finish())
}

/// POST /admin/threads/{id}/unpin - Unpin a thread
#[post("/admin/threads/{id}/unpin")]
pub async fn unpin_thread(
    client: ClientCtx,
    cookies: actix_session::Session,
    thread_id: web::Path<i32>,
    form: web::Form<ModerationForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Check moderation permission
    client.require_permission("moderate.thread.unpin")?;

    let db = get_db_pool();
    let thread_id = thread_id.into_inner();

    // Unpin the thread
    let thread = threads::Entity::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find thread: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Thread not found"))?;

    let mut active_thread: threads::ActiveModel = thread.into();
    active_thread.is_pinned = Set(false);
    active_thread.update(db).await.map_err(|e| {
        log::error!("Failed to unpin thread: {}", e);
        error::ErrorInternalServerError("Failed to unpin thread")
    })?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "unpin_thread",
        "thread",
        thread_id,
        form.reason.as_deref(),
    )
    .await?;

    log::info!(
        "Thread {} unpinned by moderator {}",
        thread_id,
        moderator_id
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/threads/{}", thread_id)))
        .finish())
}

/// Helper function to log moderation actions
async fn log_moderation_action(
    db: &DatabaseConnection,
    moderator_id: i32,
    action: &str,
    target_type: &str,
    target_id: i32,
    reason: Option<&str>,
) -> Result<(), Error> {
    let log_entry = mod_log::ActiveModel {
        moderator_id: Set(Some(moderator_id)),
        action: Set(action.to_string()),
        target_type: Set(target_type.to_string()),
        target_id: Set(target_id),
        reason: Set(reason.map(|s| s.to_string())),
        metadata: Set(None),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    };

    mod_log::Entity::insert(log_entry)
        .exec(db)
        .await
        .map_err(|e| {
            log::error!("Failed to log moderation action: {}", e);
            error::ErrorInternalServerError("Failed to log action")
        })?;

    Ok(())
}
