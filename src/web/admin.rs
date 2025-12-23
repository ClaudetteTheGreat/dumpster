/// Administration and moderation tools
///
/// This module provides endpoints for moderators and administrators.
use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{forums, mod_log, threads, user_bans, user_names, users};
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use askama::Template;
use askama_actix::TemplateToResponse;
use chrono::{Duration, Utc};
use sea_orm::{entity::*, query::*, ActiveValue::Set, DatabaseConnection};
use serde::Deserialize;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(lock_thread)
        .service(unlock_thread)
        .service(pin_thread)
        .service(unpin_thread)
        .service(view_move_thread_form)
        .service(move_thread)
        .service(view_bans)
        .service(view_ban_form)
        .service(create_ban)
        .service(lift_ban);
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

// =============================================================================
// Thread Move
// =============================================================================

#[derive(Template)]
#[template(path = "admin/move_thread.html")]
struct MoveThreadTemplate {
    client: ClientCtx,
    thread: threads::Model,
    current_forum: forums::Model,
    forums: Vec<forums::Model>,
}

#[derive(Deserialize)]
struct MoveThreadForm {
    csrf_token: String,
    target_forum_id: i32,
    reason: Option<String>,
}

/// GET /admin/threads/{id}/move - Show move thread form
#[get("/admin/threads/{id}/move")]
pub async fn view_move_thread_form(
    client: ClientCtx,
    thread_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    client.require_login()?;
    client.require_permission("moderate.thread.move")?;

    let db = get_db_pool();
    let thread_id = thread_id.into_inner();

    // Get the thread
    let thread = threads::Entity::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find thread: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Thread not found"))?;

    // Get current forum
    let current_forum = forums::Entity::find_by_id(thread.forum_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find forum: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Forum not found"))?;

    // Get all forums for selection
    let all_forums = forums::Entity::find()
        .order_by_asc(forums::Column::Label)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch forums: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    Ok(MoveThreadTemplate {
        client,
        thread,
        current_forum,
        forums: all_forums,
    }
    .to_response())
}

/// POST /admin/threads/{id}/move - Move thread to another forum
#[post("/admin/threads/{id}/move")]
pub async fn move_thread(
    client: ClientCtx,
    cookies: actix_session::Session,
    thread_id: web::Path<i32>,
    form: web::Form<MoveThreadForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("moderate.thread.move")?;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let thread_id = thread_id.into_inner();

    // Get the thread
    let thread = threads::Entity::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find thread: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Thread not found"))?;

    let old_forum_id = thread.forum_id;
    let new_forum_id = form.target_forum_id;

    // Don't allow moving to same forum
    if old_forum_id == new_forum_id {
        return Err(error::ErrorBadRequest(
            "Thread is already in the selected forum",
        ));
    }

    // Verify target forum exists
    forums::Entity::find_by_id(new_forum_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find target forum: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Target forum not found"))?;

    // Update thread's forum_id
    let mut active_thread: threads::ActiveModel = thread.into();
    active_thread.forum_id = Set(new_forum_id);
    active_thread.update(db).await.map_err(|e| {
        log::error!("Failed to move thread: {}", e);
        error::ErrorInternalServerError("Failed to move thread")
    })?;

    // Log moderation action with metadata about the move
    let metadata = serde_json::json!({
        "from_forum_id": old_forum_id,
        "to_forum_id": new_forum_id
    });

    let log_entry = mod_log::ActiveModel {
        moderator_id: Set(Some(moderator_id)),
        action: Set("move_thread".to_string()),
        target_type: Set("thread".to_string()),
        target_id: Set(thread_id),
        reason: Set(form.reason.clone()),
        metadata: Set(Some(metadata)),
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

    log::info!(
        "Thread {} moved from forum {} to forum {} by moderator {}",
        thread_id,
        old_forum_id,
        new_forum_id,
        moderator_id
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/threads/{}/", thread_id)))
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

// =============================================================================
// Ban Management
// =============================================================================

/// Information about a ban for display
#[derive(Debug, Clone)]
pub struct BanDisplay {
    pub id: i32,
    pub user_id: i32,
    pub username: String,
    pub banned_by_id: Option<i32>,
    pub banned_by_name: Option<String>,
    pub reason: String,
    pub expires_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub is_permanent: bool,
    pub is_active: bool,
}

#[derive(Template)]
#[template(path = "admin/bans.html")]
struct BansTemplate {
    client: ClientCtx,
    bans: Vec<BanDisplay>,
}

#[derive(Template)]
#[template(path = "admin/ban_form.html")]
struct BanFormTemplate {
    client: ClientCtx,
    user_id: i32,
    username: String,
    error: Option<String>,
}

#[derive(Deserialize)]
struct BanForm {
    csrf_token: String,
    reason: String,
    duration: String, // "1h", "1d", "7d", "30d", "permanent", or custom days
    custom_days: Option<i32>,
}

/// GET /admin/bans - List all bans
#[get("/admin/bans")]
async fn view_bans(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("admin.user.ban")?;

    let db = get_db_pool();

    // Fetch all bans with user information
    let bans = user_bans::Entity::find()
        .order_by_desc(user_bans::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch bans: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    let now = Utc::now().naive_utc();
    let mut ban_displays = Vec::new();

    for ban in bans {
        // Get banned user's name
        let username = user_names::Entity::find()
            .filter(user_names::Column::UserId.eq(ban.user_id))
            .one(db)
            .await
            .map_err(|e| {
                log::error!("Failed to fetch username: {}", e);
                error::ErrorInternalServerError("Database error")
            })?
            .map(|un| un.name)
            .unwrap_or_else(|| format!("User #{}", ban.user_id));

        // Get moderator's name if exists
        let banned_by_name = if let Some(mod_id) = ban.banned_by {
            user_names::Entity::find()
                .filter(user_names::Column::UserId.eq(mod_id))
                .one(db)
                .await
                .ok()
                .flatten()
                .map(|un| un.name)
        } else {
            None
        };

        // Check if ban is currently active
        let is_active = ban.is_permanent || ban.expires_at.map(|e| e > now).unwrap_or(false);

        ban_displays.push(BanDisplay {
            id: ban.id,
            user_id: ban.user_id,
            username,
            banned_by_id: ban.banned_by,
            banned_by_name,
            reason: ban.reason,
            expires_at: ban.expires_at,
            created_at: ban.created_at,
            is_permanent: ban.is_permanent,
            is_active,
        });
    }

    Ok(BansTemplate {
        client,
        bans: ban_displays,
    }
    .to_response())
}

/// GET /admin/users/{id}/ban - Show ban form for a user
#[get("/admin/users/{id}/ban")]
async fn view_ban_form(
    client: ClientCtx,
    user_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    client.require_permission("admin.user.ban")?;

    let db = get_db_pool();
    let user_id = user_id.into_inner();

    // Get user's name
    let username = user_names::Entity::find()
        .filter(user_names::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch username: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .map(|un| un.name)
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Check user exists
    users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    Ok(BanFormTemplate {
        client,
        user_id,
        username,
        error: None,
    }
    .to_response())
}

/// POST /admin/users/{id}/ban - Create a ban for a user
#[post("/admin/users/{id}/ban")]
async fn create_ban(
    client: ClientCtx,
    cookies: actix_session::Session,
    user_id: web::Path<i32>,
    form: web::Form<BanForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("admin.user.ban")?;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let user_id = user_id.into_inner();

    // Validate reason is not empty
    if form.reason.trim().is_empty() {
        return Err(error::ErrorBadRequest("Ban reason is required"));
    }

    // Check user exists
    users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Prevent banning yourself
    if user_id == moderator_id {
        return Err(error::ErrorBadRequest("You cannot ban yourself"));
    }

    // Calculate expiration
    let (expires_at, is_permanent) = match form.duration.as_str() {
        "permanent" => (None, true),
        "1h" => (Some(Utc::now().naive_utc() + Duration::hours(1)), false),
        "1d" => (Some(Utc::now().naive_utc() + Duration::days(1)), false),
        "7d" => (Some(Utc::now().naive_utc() + Duration::days(7)), false),
        "30d" => (Some(Utc::now().naive_utc() + Duration::days(30)), false),
        "custom" => {
            let days = form.custom_days.unwrap_or(1).clamp(1, 365);
            (
                Some(Utc::now().naive_utc() + Duration::days(days as i64)),
                false,
            )
        }
        _ => return Err(error::ErrorBadRequest("Invalid ban duration")),
    };

    // Create the ban
    let ban = user_bans::ActiveModel {
        user_id: Set(user_id),
        banned_by: Set(Some(moderator_id)),
        reason: Set(form.reason.trim().to_string()),
        expires_at: Set(expires_at),
        is_permanent: Set(is_permanent),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };

    ban.insert(db).await.map_err(|e| {
        log::error!("Failed to create ban: {}", e);
        error::ErrorInternalServerError("Failed to create ban")
    })?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "ban_user",
        "user",
        user_id,
        Some(&form.reason),
    )
    .await?;

    log::info!(
        "User {} banned by moderator {} (permanent: {}, expires: {:?})",
        user_id,
        moderator_id,
        is_permanent,
        expires_at
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/bans"))
        .finish())
}

/// POST /admin/bans/{id}/lift - Lift a ban
#[post("/admin/bans/{id}/lift")]
async fn lift_ban(
    client: ClientCtx,
    cookies: actix_session::Session,
    ban_id: web::Path<i32>,
    form: web::Form<ModerationForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("admin.user.ban")?;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let ban_id = ban_id.into_inner();

    // Find the ban
    let ban = user_bans::Entity::find_by_id(ban_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch ban: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Ban not found"))?;

    let user_id = ban.user_id;

    // Delete the ban (lifting it)
    user_bans::Entity::delete_by_id(ban_id)
        .exec(db)
        .await
        .map_err(|e| {
            log::error!("Failed to lift ban: {}", e);
            error::ErrorInternalServerError("Failed to lift ban")
        })?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "unban_user",
        "user",
        user_id,
        form.reason.as_deref(),
    )
    .await?;

    log::info!(
        "Ban {} on user {} lifted by moderator {}",
        ban_id,
        user_id,
        moderator_id
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/bans"))
        .finish())
}
