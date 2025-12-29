/// Administration and moderation tools
///
/// This module provides endpoints for moderators and administrators.
use crate::config::{Config, SettingValue};
use crate::db::get_db_pool;
use crate::group::GroupType;
use crate::middleware::ClientCtx;
use crate::orm::{
    badges, feature_flags, forum_permissions, forums, groups, ip_bans, mod_log, moderator_notes,
    permission_categories, permission_collections, permission_values, permissions, posts,
    reaction_types, reports, sessions, settings, threads, user_bans, user_groups, user_names,
    user_warnings, users, word_filters,
};
use crate::permission::flag::Flag;
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use askama::Template;
use askama_actix::TemplateToResponse;
use chrono::{Duration, Utc};
use sea_orm::{entity::*, query::*, ActiveValue::Set, DatabaseConnection};
use serde::Deserialize;
use std::sync::Arc;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(view_dashboard)
        .service(lock_thread)
        .service(unlock_thread)
        .service(pin_thread)
        .service(unpin_thread)
        .service(view_move_thread_form)
        .service(move_thread)
        .service(view_bans)
        .service(view_ban_form)
        .service(create_ban)
        .service(lift_ban)
        // Settings management
        .service(view_settings)
        .service(update_setting)
        .service(view_feature_flags)
        .service(toggle_feature_flag)
        // IP ban management
        .service(view_ip_bans)
        .service(view_ip_ban_form)
        .service(create_ip_ban)
        .service(lift_ip_ban)
        // Word filter management
        .service(view_word_filters)
        .service(view_word_filter_form)
        .service(create_word_filter)
        .service(view_edit_word_filter)
        .service(update_word_filter)
        .service(delete_word_filter)
        // User management
        .service(view_users)
        .service(view_edit_user)
        .service(update_user)
        // Moderator notes
        .service(view_user_notes)
        .service(create_user_note)
        .service(delete_user_note)
        // User warnings
        .service(view_user_warnings)
        .service(view_issue_warning_form)
        .service(issue_warning)
        .service(delete_warning)
        // Approval queue
        .service(view_approval_queue)
        .service(approve_user)
        .service(reject_user)
        // Mass moderation actions
        .service(mass_user_action)
        // Permission groups management
        .service(view_groups)
        .service(view_create_group_form)
        .service(create_group)
        .service(view_edit_group)
        .service(update_group)
        .service(delete_group)
        // Reaction types management
        .service(view_reaction_types)
        .service(view_edit_reaction_type)
        .service(update_reaction_type)
        .service(view_create_reaction_type_form)
        .service(create_reaction_type)
        // Badge management
        .service(view_badges)
        .service(view_create_badge_form)
        .service(create_badge)
        .service(view_edit_badge)
        .service(update_badge)
        .service(view_award_badge_form)
        .service(award_badge_to_user)
        .service(revoke_badge_from_user)
        // Forum permissions management
        .service(view_forum_permissions)
        .service(save_forum_permissions);
}

// ============================================================================
// Dashboard
// ============================================================================

/// Dashboard statistics
#[derive(Debug, Default)]
struct DashboardStats {
    total_users: i64,
    total_threads: i64,
    total_posts: i64,
    total_forums: i64,
    new_users_today: i64,
    new_threads_today: i64,
    new_posts_today: i64,
    active_bans: i64,
    active_ip_bans: i64,
    open_reports: i64,
    word_filters: i64,
    active_sessions: i64,
    db_size: String,
}

/// Recent user for dashboard display
struct RecentUser {
    id: i32,
    username: String,
    created_at: chrono::NaiveDateTime,
}

/// Recent moderation action for dashboard display
struct RecentModAction {
    action: String,
    target_type: String,
    target_id: i32,
    created_at: chrono::NaiveDateTime,
}

/// Open report for dashboard display
struct OpenReport {
    id: i32,
    content_type: String,
    reason: String,
    created_at: chrono::NaiveDateTime,
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct DashboardTemplate {
    client: ClientCtx,
    stats: DashboardStats,
    recent_users: Vec<RecentUser>,
    recent_mod_actions: Vec<RecentModAction>,
    open_reports: Vec<OpenReport>,
    server_time: String,
}

/// GET /admin - Admin dashboard
#[get("/admin")]
async fn view_dashboard(client: ClientCtx) -> Result<impl Responder, Error> {
    // Check admin permission - require login first
    let _user_id = client.require_login()?;

    // For now, allow any logged-in user to view dashboard
    // In production, you would check for admin permission here

    let db = get_db_pool();
    let now = Utc::now().naive_utc();
    let today_start = now.date().and_hms_opt(0, 0, 0).unwrap();

    // Gather statistics
    let total_users = users::Entity::find().count(db).await.unwrap_or(0) as i64;

    let total_threads = threads::Entity::find()
        .filter(threads::Column::DeletedAt.is_null())
        .filter(threads::Column::MergedIntoId.is_null())
        .count(db)
        .await
        .unwrap_or(0) as i64;

    let total_posts = posts::Entity::find().count(db).await.unwrap_or(0) as i64;

    let total_forums = forums::Entity::find().count(db).await.unwrap_or(0) as i64;

    let new_users_today = users::Entity::find()
        .filter(users::Column::CreatedAt.gte(today_start))
        .count(db)
        .await
        .unwrap_or(0) as i64;

    let new_threads_today = threads::Entity::find()
        .filter(threads::Column::CreatedAt.gte(today_start))
        .count(db)
        .await
        .unwrap_or(0) as i64;

    let new_posts_today = posts::Entity::find()
        .filter(posts::Column::CreatedAt.gte(today_start))
        .count(db)
        .await
        .unwrap_or(0) as i64;

    let active_bans = user_bans::Entity::find()
        .filter(
            user_bans::Column::ExpiresAt
                .is_null()
                .or(user_bans::Column::ExpiresAt.gt(now)),
        )
        .count(db)
        .await
        .unwrap_or(0) as i64;

    let active_ip_bans = ip_bans::Entity::find()
        .filter(
            ip_bans::Column::ExpiresAt
                .is_null()
                .or(ip_bans::Column::ExpiresAt.gt(now)),
        )
        .count(db)
        .await
        .unwrap_or(0) as i64;

    let open_reports_count = reports::Entity::find()
        .filter(reports::Column::Status.eq("open"))
        .count(db)
        .await
        .unwrap_or(0) as i64;

    let word_filter_count = word_filters::Entity::find()
        .filter(word_filters::Column::IsEnabled.eq(true))
        .count(db)
        .await
        .unwrap_or(0) as i64;

    let active_sessions = sessions::Entity::find().count(db).await.unwrap_or(0) as i64;

    // Database size would require raw query - simplified for now
    let db_size = "N/A".to_string();

    let stats = DashboardStats {
        total_users,
        total_threads,
        total_posts,
        total_forums,
        new_users_today,
        new_threads_today,
        new_posts_today,
        active_bans,
        active_ip_bans,
        open_reports: open_reports_count,
        word_filters: word_filter_count,
        active_sessions,
        db_size,
    };

    // Recent users (last 10) - join with user_names to get usernames
    let recent_user_models = users::Entity::find()
        .order_by_desc(users::Column::CreatedAt)
        .limit(10)
        .all(db)
        .await
        .unwrap_or_default();

    let mut recent_users: Vec<RecentUser> = Vec::new();
    for user in recent_user_models {
        let username = user_names::Entity::find()
            .filter(user_names::Column::UserId.eq(user.id))
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|un| un.name)
            .unwrap_or_else(|| format!("User #{}", user.id));

        recent_users.push(RecentUser {
            id: user.id,
            username,
            created_at: user.created_at,
        });
    }

    // Recent mod actions (last 10)
    let recent_mod_models = mod_log::Entity::find()
        .order_by_desc(mod_log::Column::CreatedAt)
        .limit(10)
        .all(db)
        .await
        .unwrap_or_default();

    let recent_mod_actions: Vec<RecentModAction> = recent_mod_models
        .into_iter()
        .map(|m| RecentModAction {
            action: m.action,
            target_type: m.target_type,
            target_id: m.target_id,
            created_at: m.created_at,
        })
        .collect();

    // Open reports (last 5)
    let open_report_models = reports::Entity::find()
        .filter(reports::Column::Status.eq("open"))
        .order_by_desc(reports::Column::CreatedAt)
        .limit(5)
        .all(db)
        .await
        .unwrap_or_default();

    let open_reports: Vec<OpenReport> = open_report_models
        .into_iter()
        .map(|r| OpenReport {
            id: r.id,
            content_type: r.content_type,
            reason: r.reason,
            created_at: r.created_at,
        })
        .collect();

    let server_time = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    Ok(DashboardTemplate {
        client,
        stats,
        recent_users,
        recent_mod_actions,
        open_reports,
        server_time,
    }
    .to_response())
}

// ============================================================================
// Thread Moderation
// ============================================================================

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

// =============================================================================
// Settings Management
// =============================================================================

#[derive(Template)]
#[template(path = "admin/settings.html")]
struct SettingsTemplate {
    client: ClientCtx,
    categories: Vec<(String, Vec<settings::Model>)>,
    #[allow(dead_code)]
    success_message: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/feature_flags.html")]
struct FeatureFlagsTemplate {
    client: ClientCtx,
    flags: Vec<feature_flags::Model>,
}

#[derive(Deserialize)]
struct UpdateSettingForm {
    csrf_token: String,
    key: String,
    value: String,
}

#[derive(Deserialize)]
struct ToggleFlagForm {
    csrf_token: String,
    key: String,
    enabled: Option<String>, // checkbox
}

/// GET /admin/settings - View and manage site settings
#[get("/admin/settings")]
async fn view_settings(
    client: ClientCtx,
    config: web::Data<Arc<Config>>,
) -> Result<impl Responder, Error> {
    client.require_permission("admin.settings")?;

    let db = get_db_pool();

    let categories = config.get_all_by_category(db).await.map_err(|e| {
        log::error!("Failed to fetch settings: {}", e);
        error::ErrorInternalServerError("Database error")
    })?;

    Ok(SettingsTemplate {
        client,
        categories,
        success_message: None,
    }
    .to_response())
}

/// POST /admin/settings - Update a setting
#[post("/admin/settings")]
async fn update_setting(
    client: ClientCtx,
    cookies: actix_session::Session,
    config: web::Data<Arc<Config>>,
    form: web::Form<UpdateSettingForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;
    client.require_permission("admin.settings")?;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();

    // Find the setting to get its type
    let setting = settings::Entity::find_by_id(form.key.clone())
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find setting: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Setting not found"))?;

    // Parse value according to type
    let value = SettingValue::parse(&form.value, &setting.value_type)
        .ok_or_else(|| error::ErrorBadRequest("Invalid value for setting type"))?;

    // Update the setting
    config
        .set_value(db, &form.key, value, Some(user_id))
        .await
        .map_err(|e| {
            log::error!("Failed to update setting: {}", e);
            error::ErrorInternalServerError("Failed to update setting")
        })?;

    log::info!("Setting '{}' updated by user {}", form.key, user_id);

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/settings?updated=1"))
        .finish())
}

/// GET /admin/feature-flags - View feature flags
#[get("/admin/feature-flags")]
async fn view_feature_flags(
    client: ClientCtx,
    config: web::Data<Arc<Config>>,
) -> Result<impl Responder, Error> {
    client.require_permission("admin.settings")?;

    let db = get_db_pool();

    let flags = config.get_all_feature_flags(db).await.map_err(|e| {
        log::error!("Failed to fetch feature flags: {}", e);
        error::ErrorInternalServerError("Database error")
    })?;

    Ok(FeatureFlagsTemplate { client, flags }.to_response())
}

/// POST /admin/feature-flags - Toggle a feature flag
#[post("/admin/feature-flags")]
async fn toggle_feature_flag(
    client: ClientCtx,
    cookies: actix_session::Session,
    config: web::Data<Arc<Config>>,
    form: web::Form<ToggleFlagForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;
    client.require_permission("admin.settings")?;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let enabled = form.enabled.is_some();

    // Update the feature flag
    config
        .set_feature_flag(db, &form.key, enabled)
        .await
        .map_err(|e| {
            log::error!("Failed to toggle feature flag: {}", e);
            error::ErrorInternalServerError("Failed to toggle feature flag")
        })?;

    log::info!(
        "Feature flag '{}' set to {} by user {}",
        form.key,
        enabled,
        user_id
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/feature-flags"))
        .finish())
}

// =============================================================================
// IP Ban Management
// =============================================================================

/// Information about an IP ban for display
#[derive(Debug, Clone)]
pub struct IpBanDisplay {
    pub id: i32,
    pub ip_address: String,
    pub banned_by_id: Option<i32>,
    pub banned_by_name: Option<String>,
    pub reason: String,
    pub expires_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub is_permanent: bool,
    pub is_range_ban: bool,
    pub is_active: bool,
}

#[derive(Template)]
#[template(path = "admin/ip_bans.html")]
struct IpBansTemplate {
    client: ClientCtx,
    bans: Vec<IpBanDisplay>,
}

#[derive(Template)]
#[template(path = "admin/ip_ban_form.html")]
struct IpBanFormTemplate {
    client: ClientCtx,
    error: Option<String>,
}

#[derive(Deserialize)]
struct IpBanForm {
    csrf_token: String,
    ip_address: String,
    reason: String,
    duration: String, // "1h", "1d", "7d", "30d", "90d", "permanent", or "custom"
    custom_days: Option<i32>,
    is_range_ban: Option<String>, // checkbox
}

/// GET /admin/ip-bans - List all IP bans
#[get("/admin/ip-bans")]
async fn view_ip_bans(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("admin.ip.ban")?;

    let db = get_db_pool();

    // Fetch all IP bans using raw SQL for proper INET type handling
    use sea_orm::{ConnectionTrait, Statement};

    let sql = r#"
        SELECT
            ib.id,
            ib.ip_address::TEXT as ip_address,
            ib.banned_by,
            ib.reason,
            ib.expires_at,
            ib.created_at,
            ib.is_permanent,
            ib.is_range_ban,
            un.name as banned_by_name
        FROM ip_bans ib
        LEFT JOIN user_names un ON un.user_id = ib.banned_by
        ORDER BY ib.created_at DESC
    "#;

    let rows = db
        .query_all(Statement::from_string(
            db.get_database_backend(),
            sql.to_string(),
        ))
        .await
        .map_err(|e| {
            log::error!("Failed to fetch IP bans: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    let now = Utc::now().naive_utc();
    let mut ban_displays = Vec::new();

    for row in rows {
        let id: i32 = row.try_get("", "id").map_err(|e| {
            log::error!("Failed to parse IP ban row: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;
        let ip_address: String = row.try_get("", "ip_address").unwrap_or_default();
        let banned_by: Option<i32> = row.try_get("", "banned_by").ok();
        let reason: String = row.try_get("", "reason").unwrap_or_default();
        let expires_at: Option<chrono::NaiveDateTime> = row.try_get("", "expires_at").ok();
        let created_at: chrono::NaiveDateTime = row
            .try_get("", "created_at")
            .unwrap_or_else(|_| Utc::now().naive_utc());
        let is_permanent: bool = row.try_get("", "is_permanent").unwrap_or(false);
        let is_range_ban: bool = row.try_get("", "is_range_ban").unwrap_or(false);
        let banned_by_name: Option<String> = row.try_get("", "banned_by_name").ok();

        // Check if ban is currently active
        let is_active = is_permanent || expires_at.map(|e| e > now).unwrap_or(false);

        ban_displays.push(IpBanDisplay {
            id,
            ip_address,
            banned_by_id: banned_by,
            banned_by_name,
            reason,
            expires_at,
            created_at,
            is_permanent,
            is_range_ban,
            is_active,
        });
    }

    Ok(IpBansTemplate {
        client,
        bans: ban_displays,
    }
    .to_response())
}

/// GET /admin/ip-bans/new - Show IP ban form
#[get("/admin/ip-bans/new")]
async fn view_ip_ban_form(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("admin.ip.ban")?;

    Ok(IpBanFormTemplate {
        client,
        error: None,
    }
    .to_response())
}

/// POST /admin/ip-bans - Create a new IP ban
#[post("/admin/ip-bans")]
async fn create_ip_ban(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<IpBanForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("admin.ip.ban")?;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();

    // Validate IP address format
    let ip_address = form.ip_address.trim();
    if ip_address.is_empty() {
        return Err(error::ErrorBadRequest("IP address is required"));
    }

    // Basic IP validation - PostgreSQL INET type will do final validation
    // Check for valid IPv4, IPv6, or CIDR notation
    let is_valid_ip = ip_address.parse::<std::net::IpAddr>().is_ok()
        || ip_address
            .split('/')
            .next()
            .map(|ip| ip.parse::<std::net::IpAddr>().is_ok())
            .unwrap_or(false);

    if !is_valid_ip {
        return Err(error::ErrorBadRequest(
            "Invalid IP address format. Use IPv4, IPv6, or CIDR notation (e.g., 192.168.1.1 or 192.168.1.0/24)",
        ));
    }

    // Validate reason is not empty
    if form.reason.trim().is_empty() {
        return Err(error::ErrorBadRequest("Ban reason is required"));
    }

    // Note: Duplicate IP check is handled by the unique constraint in the database.
    // The error handling in the insert will return an appropriate message if duplicate.

    // Calculate expiration
    let (expires_at, is_permanent) = match form.duration.as_str() {
        "permanent" => (None, true),
        "1h" => (Some(Utc::now().naive_utc() + Duration::hours(1)), false),
        "1d" => (Some(Utc::now().naive_utc() + Duration::days(1)), false),
        "7d" => (Some(Utc::now().naive_utc() + Duration::days(7)), false),
        "30d" => (Some(Utc::now().naive_utc() + Duration::days(30)), false),
        "90d" => (Some(Utc::now().naive_utc() + Duration::days(90)), false),
        "custom" => {
            let days = form.custom_days.unwrap_or(7).clamp(1, 365);
            (
                Some(Utc::now().naive_utc() + Duration::days(days as i64)),
                false,
            )
        }
        _ => return Err(error::ErrorBadRequest("Invalid ban duration")),
    };

    let is_range_ban = form.is_range_ban.is_some() || ip_address.contains('/');
    let now = Utc::now().naive_utc();
    let now_str = format!("{}", now.format("%Y-%m-%d %H:%M:%S"));

    // Create the IP ban using raw SQL for proper INET type handling
    let (expires_sql, expires_param) = if let Some(exp) = expires_at {
        (
            "$5::TIMESTAMP",
            format!("{}", exp.format("%Y-%m-%d %H:%M:%S")),
        )
    } else {
        ("NULL", String::new())
    };

    let insert_sql = format!(
        r#"
        INSERT INTO ip_bans (ip_address, banned_by, reason, expires_at, is_permanent, is_range_ban, created_at)
        VALUES ($1::INET, $2, $3, {}, $4, $6, $7::TIMESTAMP)
        "#,
        expires_sql
    );

    use sea_orm::{ConnectionTrait, Statement};
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        &insert_sql,
        vec![
            ip_address.into(),
            moderator_id.into(),
            form.reason.trim().into(),
            is_permanent.into(),
            expires_param.into(),
            is_range_ban.into(),
            now_str.into(),
        ],
    ))
    .await
    .map_err(|e| {
        log::error!("Failed to create IP ban: {}", e);
        // Check if it's a PostgreSQL INET type error
        if e.to_string().contains("inet") || e.to_string().contains("invalid input syntax") {
            error::ErrorBadRequest("Invalid IP address format")
        } else if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
            error::ErrorBadRequest("This IP address is already banned")
        } else {
            error::ErrorInternalServerError("Failed to create IP ban")
        }
    })?;

    // Log moderation action
    let metadata = serde_json::json!({
        "ip_address": ip_address,
        "is_range_ban": is_range_ban,
        "is_permanent": is_permanent,
        "expires_at": expires_at,
    });

    let log_entry = mod_log::ActiveModel {
        moderator_id: Set(Some(moderator_id)),
        action: Set("ban_ip".to_string()),
        target_type: Set("ip".to_string()),
        target_id: Set(0), // No target ID for IP bans
        reason: Set(Some(form.reason.trim().to_string())),
        metadata: Set(Some(metadata)),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    };

    mod_log::Entity::insert(log_entry)
        .exec(db)
        .await
        .map_err(|e| {
            log::error!("Failed to log IP ban action: {}", e);
            error::ErrorInternalServerError("Failed to log action")
        })?;

    log::info!(
        "IP {} banned by moderator {} (permanent: {}, range: {}, expires: {:?})",
        ip_address,
        moderator_id,
        is_permanent,
        is_range_ban,
        expires_at
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/ip-bans"))
        .finish())
}

/// POST /admin/ip-bans/{id}/lift - Lift an IP ban
#[post("/admin/ip-bans/{id}/lift")]
async fn lift_ip_ban(
    client: ClientCtx,
    cookies: actix_session::Session,
    ban_id: web::Path<i32>,
    form: web::Form<ModerationForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("admin.ip.ban")?;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let ban_id = ban_id.into_inner();

    // Find the ban using raw SQL for proper INET type handling
    use sea_orm::{ConnectionTrait, Statement};

    let sql = "SELECT ip_address::TEXT as ip_address FROM ip_bans WHERE id = $1";
    let row = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            sql,
            vec![ban_id.into()],
        ))
        .await
        .map_err(|e| {
            log::error!("Failed to fetch IP ban: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("IP ban not found"))?;

    let ip_address: String = row.try_get("", "ip_address").map_err(|e| {
        log::error!("Failed to parse IP ban row: {}", e);
        error::ErrorInternalServerError("Database error")
    })?;

    // Delete the ban (lifting it) - delete by ID works fine
    ip_bans::Entity::delete_by_id(ban_id)
        .exec(db)
        .await
        .map_err(|e| {
            log::error!("Failed to lift IP ban: {}", e);
            error::ErrorInternalServerError("Failed to lift IP ban")
        })?;

    // Log moderation action
    let metadata = serde_json::json!({
        "ip_address": ip_address,
    });

    let log_entry = mod_log::ActiveModel {
        moderator_id: Set(Some(moderator_id)),
        action: Set("unban_ip".to_string()),
        target_type: Set("ip".to_string()),
        target_id: Set(ban_id),
        reason: Set(form.reason.clone()),
        metadata: Set(Some(metadata)),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    };

    mod_log::Entity::insert(log_entry)
        .exec(db)
        .await
        .map_err(|e| {
            log::error!("Failed to log IP unban action: {}", e);
            error::ErrorInternalServerError("Failed to log action")
        })?;

    log::info!(
        "IP ban {} ({}) lifted by moderator {}",
        ban_id,
        ip_address,
        moderator_id
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/ip-bans"))
        .finish())
}

// =============================================================================
// Word Filter Management
// =============================================================================

#[derive(Template)]
#[template(path = "admin/word_filters.html")]
struct WordFiltersTemplate {
    client: ClientCtx,
    filters: Vec<word_filters::Model>,
}

#[derive(Template)]
#[template(path = "admin/word_filter_form.html")]
struct WordFilterFormTemplate {
    client: ClientCtx,
    filter: Option<word_filters::Model>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct WordFilterForm {
    csrf_token: String,
    pattern: String,
    replacement: Option<String>,
    action: String,
    is_regex: Option<String>,
    is_case_sensitive: Option<String>,
    is_whole_word: Option<String>,
    is_enabled: Option<String>,
    notes: Option<String>,
}

/// GET /admin/word-filters - View all word filters
#[get("/admin/word-filters")]
async fn view_word_filters(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("admin.word_filters.view")?;

    let db = get_db_pool();

    let filters = word_filters::Entity::find()
        .order_by_asc(word_filters::Column::Pattern)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch word filters: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    Ok(WordFiltersTemplate { client, filters }.to_response())
}

/// GET /admin/word-filters/new - Show word filter creation form
#[get("/admin/word-filters/new")]
async fn view_word_filter_form(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("admin.word_filters.manage")?;

    Ok(WordFilterFormTemplate {
        client,
        filter: None,
        error: None,
    }
    .to_response())
}

/// POST /admin/word-filters - Create a new word filter
#[post("/admin/word-filters")]
async fn create_word_filter(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<WordFilterForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;
    client.require_permission("admin.word_filters.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();

    // Validate pattern is not empty
    if form.pattern.trim().is_empty() {
        return Err(error::ErrorBadRequest("Pattern is required"));
    }

    // Validate action
    let action = match form.action.as_str() {
        "replace" => word_filters::FilterAction::Replace,
        "block" => word_filters::FilterAction::Block,
        "flag" => word_filters::FilterAction::Flag,
        _ => return Err(error::ErrorBadRequest("Invalid action")),
    };

    // For replace action, replacement is recommended
    let replacement = form.replacement.as_ref().map(|r| r.trim().to_string());

    // If regex, validate it compiles
    let is_regex = form.is_regex.is_some();
    if is_regex {
        if let Err(e) = regex::Regex::new(&form.pattern) {
            return Err(error::ErrorBadRequest(format!(
                "Invalid regex pattern: {}",
                e
            )));
        }
    }

    let filter = word_filters::ActiveModel {
        pattern: Set(form.pattern.trim().to_string()),
        replacement: Set(replacement),
        is_regex: Set(is_regex),
        is_case_sensitive: Set(form.is_case_sensitive.is_some()),
        is_whole_word: Set(form.is_whole_word.is_some()),
        action: Set(action),
        is_enabled: Set(form.is_enabled.is_some()),
        created_by: Set(Some(user_id)),
        created_at: Set(Utc::now().naive_utc()),
        notes: Set(form.notes.as_ref().map(|n| n.trim().to_string())),
        ..Default::default()
    };

    filter.insert(db).await.map_err(|e| {
        log::error!("Failed to create word filter: {}", e);
        error::ErrorInternalServerError("Failed to create word filter")
    })?;

    // Reload filters in cache
    crate::word_filter::reload_filters(db).await.ok();

    log::info!(
        "Word filter '{}' created by user {}",
        form.pattern.trim(),
        user_id
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/word-filters"))
        .finish())
}

/// GET /admin/word-filters/{id}/edit - Show word filter edit form
#[get("/admin/word-filters/{id}/edit")]
async fn view_edit_word_filter(
    client: ClientCtx,
    filter_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    client.require_permission("admin.word_filters.manage")?;

    let db = get_db_pool();
    let filter_id = filter_id.into_inner();

    let filter = word_filters::Entity::find_by_id(filter_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch word filter: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Word filter not found"))?;

    Ok(WordFilterFormTemplate {
        client,
        filter: Some(filter),
        error: None,
    }
    .to_response())
}

/// POST /admin/word-filters/{id} - Update a word filter
#[post("/admin/word-filters/{id}")]
async fn update_word_filter(
    client: ClientCtx,
    cookies: actix_session::Session,
    filter_id: web::Path<i32>,
    form: web::Form<WordFilterForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;
    client.require_permission("admin.word_filters.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let filter_id = filter_id.into_inner();

    // Validate pattern is not empty
    if form.pattern.trim().is_empty() {
        return Err(error::ErrorBadRequest("Pattern is required"));
    }

    // Find existing filter
    let filter = word_filters::Entity::find_by_id(filter_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch word filter: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Word filter not found"))?;

    // Validate action
    let action = match form.action.as_str() {
        "replace" => word_filters::FilterAction::Replace,
        "block" => word_filters::FilterAction::Block,
        "flag" => word_filters::FilterAction::Flag,
        _ => return Err(error::ErrorBadRequest("Invalid action")),
    };

    let replacement = form.replacement.as_ref().map(|r| r.trim().to_string());

    // If regex, validate it compiles
    let is_regex = form.is_regex.is_some();
    if is_regex {
        if let Err(e) = regex::Regex::new(&form.pattern) {
            return Err(error::ErrorBadRequest(format!(
                "Invalid regex pattern: {}",
                e
            )));
        }
    }

    let mut active_filter: word_filters::ActiveModel = filter.into();
    active_filter.pattern = Set(form.pattern.trim().to_string());
    active_filter.replacement = Set(replacement);
    active_filter.is_regex = Set(is_regex);
    active_filter.is_case_sensitive = Set(form.is_case_sensitive.is_some());
    active_filter.is_whole_word = Set(form.is_whole_word.is_some());
    active_filter.action = Set(action);
    active_filter.is_enabled = Set(form.is_enabled.is_some());
    active_filter.notes = Set(form.notes.as_ref().map(|n| n.trim().to_string()));

    active_filter.update(db).await.map_err(|e| {
        log::error!("Failed to update word filter: {}", e);
        error::ErrorInternalServerError("Failed to update word filter")
    })?;

    // Reload filters in cache
    crate::word_filter::reload_filters(db).await.ok();

    log::info!("Word filter {} updated by user {}", filter_id, user_id);

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/word-filters"))
        .finish())
}

/// POST /admin/word-filters/{id}/delete - Delete a word filter
#[post("/admin/word-filters/{id}/delete")]
async fn delete_word_filter(
    client: ClientCtx,
    cookies: actix_session::Session,
    filter_id: web::Path<i32>,
    form: web::Form<ModerationForm>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;
    client.require_permission("admin.word_filters.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let filter_id = filter_id.into_inner();

    // Find filter to get pattern for logging
    let filter = word_filters::Entity::find_by_id(filter_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch word filter: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Word filter not found"))?;

    let pattern = filter.pattern.clone();

    // Delete the filter
    word_filters::Entity::delete_by_id(filter_id)
        .exec(db)
        .await
        .map_err(|e| {
            log::error!("Failed to delete word filter: {}", e);
            error::ErrorInternalServerError("Failed to delete word filter")
        })?;

    // Reload filters in cache
    crate::word_filter::reload_filters(db).await.ok();

    log::info!(
        "Word filter '{}' (id: {}) deleted by user {}",
        pattern,
        filter_id,
        user_id
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/word-filters"))
        .finish())
}

// =============================================================================
// User Management
// =============================================================================

/// User display for admin list
#[derive(Debug)]
struct UserDisplay {
    id: i32,
    username: String,
    email: Option<String>,
    created_at: chrono::NaiveDateTime,
    email_verified: bool,
    is_banned: bool,
}

#[derive(Template)]
#[template(path = "admin/users.html")]
struct UsersTemplate {
    client: ClientCtx,
    users: Vec<UserDisplay>,
    page: i32,
    total_pages: i32,
    search_query: String,
    can_mass_moderate: bool,
}

/// Group with membership status for template
struct GroupWithMembership {
    id: i32,
    label: String,
    is_member: bool,
}

#[derive(Template)]
#[template(path = "admin/user_edit.html")]
struct UserEditTemplate {
    client: ClientCtx,
    user: users::Model,
    username: String,
    groups: Vec<GroupWithMembership>,
    error: Option<String>,
    success: Option<String>,
}

#[derive(Deserialize)]
struct UserListQuery {
    page: Option<i32>,
    q: Option<String>,
}

#[derive(Deserialize)]
struct UserEditForm {
    csrf_token: String,
    username: String,
    email: Option<String>,
    email_verified: Option<String>,
    custom_title: Option<String>,
    bio: Option<String>,
    location: Option<String>,
    website_url: Option<String>,
    signature: Option<String>,
    #[serde(default)]
    groups: Vec<i32>,
    new_password: Option<String>,
    reset_lockout: Option<String>,
}

/// GET /admin/users - List all users
#[get("/admin/users")]
async fn view_users(
    client: ClientCtx,
    query: web::Query<UserListQuery>,
) -> Result<impl Responder, Error> {
    client.require_permission("admin.user.manage")?;

    let db = get_db_pool();
    let page = query.page.unwrap_or(1).max(1);
    let per_page = 50;
    let offset = ((page - 1) * per_page) as u64;
    let search_query = query.q.clone().unwrap_or_default();

    // Build query
    let mut user_query = users::Entity::find();

    // If there's a search query, filter by username or email
    if !search_query.is_empty() {
        // We need to join with user_names for username search
        // For simplicity, we'll search by email only in the users table
        // and then filter by username after fetching
        user_query = user_query.filter(users::Column::Email.contains(&search_query));
    }

    // Get total count for pagination
    let total_count = user_query.clone().count(db).await.unwrap_or(0) as i32;

    let total_pages = (total_count + per_page - 1) / per_page;

    // Fetch users
    let user_models = user_query
        .order_by_desc(users::Column::CreatedAt)
        .offset(offset)
        .limit(per_page as u64)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch users: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    // Get current time for ban check
    let now = Utc::now().naive_utc();

    // Build user displays with additional info
    let mut user_displays = Vec::new();
    for user in user_models {
        // Get username
        let username = user_names::Entity::find()
            .filter(user_names::Column::UserId.eq(user.id))
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|un| un.name)
            .unwrap_or_else(|| format!("User #{}", user.id));

        // If searching and username doesn't match, skip
        if !search_query.is_empty()
            && !username
                .to_lowercase()
                .contains(&search_query.to_lowercase())
            && !user
                .email
                .as_ref()
                .map(|e| e.to_lowercase().contains(&search_query.to_lowercase()))
                .unwrap_or(false)
        {
            continue;
        }

        // Check if user is banned
        let is_banned = user_bans::Entity::find()
            .filter(user_bans::Column::UserId.eq(user.id))
            .filter(
                user_bans::Column::IsPermanent
                    .eq(true)
                    .or(user_bans::Column::ExpiresAt.gt(now)),
            )
            .one(db)
            .await
            .ok()
            .flatten()
            .is_some();

        user_displays.push(UserDisplay {
            id: user.id,
            username,
            email: user.email.clone(),
            created_at: user.created_at,
            email_verified: user.email_verified,
            is_banned,
        });
    }

    let can_mass_moderate = client.can("moderate.mass.users");

    Ok(UsersTemplate {
        client,
        users: user_displays,
        page,
        total_pages,
        search_query,
        can_mass_moderate,
    }
    .to_response())
}

/// GET /admin/users/{id}/edit - View user edit form
#[get("/admin/users/{id}/edit")]
async fn view_edit_user(
    client: ClientCtx,
    user_id: web::Path<i32>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<impl Responder, Error> {
    client.require_permission("admin.user.manage")?;

    let db = get_db_pool();
    let user_id = user_id.into_inner();

    // Find user
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Get username
    let username = user_names::Entity::find()
        .filter(user_names::Column::UserId.eq(user_id))
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|un| un.name)
        .unwrap_or_else(|| format!("User #{}", user_id));

    // Get all groups
    let all_groups = groups::Entity::find()
        .order_by_asc(groups::Column::Label)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch groups: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    // Get user's current groups
    let user_group_ids: Vec<i32> = user_groups::Entity::find()
        .filter(user_groups::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user groups: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .into_iter()
        .map(|ug| ug.group_id)
        .collect();

    // Build groups with membership status
    let groups: Vec<GroupWithMembership> = all_groups
        .into_iter()
        .map(|g| GroupWithMembership {
            id: g.id,
            label: g.label,
            is_member: user_group_ids.contains(&g.id),
        })
        .collect();

    // Check for success message
    let success = if query.contains_key("success") {
        Some("User updated successfully".to_string())
    } else {
        None
    };

    Ok(UserEditTemplate {
        client,
        user,
        username,
        groups,
        error: None,
        success,
    }
    .to_response())
}

/// POST /admin/users/{id}/edit - Update user details
#[post("/admin/users/{id}/edit")]
async fn update_user(
    client: ClientCtx,
    cookies: actix_session::Session,
    user_id: web::Path<i32>,
    form: web::Form<UserEditForm>,
) -> Result<impl Responder, Error> {
    let admin_id = client.require_login()?;
    client.require_permission("admin.user.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let user_id = user_id.into_inner();

    // Find user
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Validate username
    let new_username = form.username.trim();
    if new_username.is_empty() {
        return Err(error::ErrorBadRequest("Username is required"));
    }
    if new_username.len() > 255 {
        return Err(error::ErrorBadRequest("Username is too long"));
    }

    // Get current username
    let current_username = user_names::Entity::find()
        .filter(user_names::Column::UserId.eq(user_id))
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|un| un.name)
        .unwrap_or_default();

    // If username changed, update the username record
    if new_username != current_username {
        // Check if username is already taken by another user
        let existing = user_names::Entity::find()
            .filter(user_names::Column::Name.eq(new_username))
            .filter(user_names::Column::UserId.ne(user_id))
            .one(db)
            .await
            .map_err(|e| {
                log::error!("Failed to check username: {}", e);
                error::ErrorInternalServerError("Database error")
            })?;

        if existing.is_some() {
            return Err(error::ErrorBadRequest("Username is already taken"));
        }

        // Update existing username record
        let active_username = user_names::ActiveModel {
            user_id: Set(user_id),
            name: Set(new_username.to_string()),
        };
        active_username.update(db).await.map_err(|e| {
            log::error!("Failed to update username: {}", e);
            error::ErrorInternalServerError("Failed to update username")
        })?;

        log::info!(
            "Username changed for user {} from '{}' to '{}' by admin {}",
            user_id,
            current_username,
            new_username,
            admin_id
        );
    }

    // Update user record
    let mut active_user: users::ActiveModel = user.into();

    // Update email
    let email = form
        .email
        .as_ref()
        .map(|e| e.trim())
        .filter(|e| !e.is_empty())
        .map(|e| e.to_string());
    active_user.email = Set(email);

    // Update email verified status
    active_user.email_verified = Set(form.email_verified.is_some());

    // Update profile fields
    active_user.custom_title = Set(form
        .custom_title
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string()));

    active_user.bio = Set(form
        .bio
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string()));

    active_user.location = Set(form
        .location
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string()));

    active_user.website_url = Set(form
        .website_url
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string()));

    active_user.signature = Set(form
        .signature
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string()));

    // Reset lockout if requested
    if form.reset_lockout.is_some() {
        active_user.failed_login_attempts = Set(0);
        active_user.locked_until = Set(None);
        log::info!(
            "Account lockout reset for user {} by admin {}",
            user_id,
            admin_id
        );
    }

    // Update password if provided
    if let Some(new_password) = form.new_password.as_ref() {
        let new_password = new_password.trim();
        if !new_password.is_empty() {
            if new_password.len() < 8 {
                return Err(error::ErrorBadRequest(
                    "Password must be at least 8 characters",
                ));
            }

            // Hash the new password
            use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
            use rand::rngs::OsRng;

            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();
            let password_hash = argon2
                .hash_password(new_password.as_bytes(), &salt)
                .map_err(|e| {
                    log::error!("Failed to hash password: {}", e);
                    error::ErrorInternalServerError("Failed to hash password")
                })?
                .to_string();

            active_user.password = Set(password_hash);
            active_user.password_cipher = Set(users::Cipher::Argon2id);

            log::info!("Password reset for user {} by admin {}", user_id, admin_id);
        }
    }

    // Save user changes
    active_user.update(db).await.map_err(|e| {
        log::error!("Failed to update user: {}", e);
        error::ErrorInternalServerError("Failed to update user")
    })?;

    // Update user groups
    // First, delete all existing group memberships
    user_groups::Entity::delete_many()
        .filter(user_groups::Column::UserId.eq(user_id))
        .exec(db)
        .await
        .map_err(|e| {
            log::error!("Failed to delete user groups: {}", e);
            error::ErrorInternalServerError("Failed to update groups")
        })?;

    // Then, insert new group memberships
    for group_id in &form.groups {
        let membership = user_groups::ActiveModel {
            user_id: Set(user_id),
            group_id: Set(*group_id),
        };
        membership.insert(db).await.map_err(|e| {
            log::error!("Failed to add user to group: {}", e);
            error::ErrorInternalServerError("Failed to update groups")
        })?;
    }

    // Log the moderation action
    log_moderation_action(db, admin_id, "edit_user", "user", user_id, None).await?;

    log::info!("User {} updated by admin {}", user_id, admin_id);

    Ok(HttpResponse::SeeOther()
        .append_header((
            "Location",
            format!("/admin/users/{}/edit?success=1", user_id),
        ))
        .finish())
}

// =============================================================================
// Moderator Notes
// =============================================================================

/// Note display for templates
#[allow(dead_code)]
struct NoteDisplay {
    id: i32,
    author_id: Option<i32>,
    author_name: String,
    content: String,
    created_at: chrono::NaiveDateTime,
}

#[derive(Template)]
#[template(path = "admin/user_notes.html")]
struct UserNotesTemplate {
    client: ClientCtx,
    user_id: i32,
    username: String,
    notes: Vec<NoteDisplay>,
    can_manage: bool,
}

#[derive(Deserialize)]
struct NoteForm {
    csrf_token: String,
    content: String,
}

/// GET /admin/users/{id}/notes - View moderator notes for a user
#[get("/admin/users/{id}/notes")]
async fn view_user_notes(
    client: ClientCtx,
    user_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    client.require_permission("moderate.notes.view")?;

    let db = get_db_pool();
    let user_id = user_id.into_inner();

    // Get username
    let username = user_names::Entity::find()
        .filter(user_names::Column::UserId.eq(user_id))
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|un| un.name)
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Check if user can manage notes
    let can_manage = client.can("moderate.notes.manage");

    // Get notes
    let note_models = moderator_notes::Entity::find()
        .filter(moderator_notes::Column::UserId.eq(user_id))
        .order_by_desc(moderator_notes::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch notes: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    // Build note displays with author names
    let mut notes = Vec::new();
    for note in note_models {
        let author_name = if let Some(author_id) = note.author_id {
            user_names::Entity::find()
                .filter(user_names::Column::UserId.eq(author_id))
                .one(db)
                .await
                .ok()
                .flatten()
                .map(|un| un.name)
                .unwrap_or_else(|| format!("User #{}", author_id))
        } else {
            "Deleted User".to_string()
        };

        notes.push(NoteDisplay {
            id: note.id,
            author_id: note.author_id,
            author_name,
            content: note.content,
            created_at: note.created_at,
        });
    }

    Ok(UserNotesTemplate {
        client,
        user_id,
        username,
        notes,
        can_manage,
    }
    .to_response())
}

/// POST /admin/users/{id}/notes - Create a new moderator note
#[post("/admin/users/{id}/notes")]
async fn create_user_note(
    client: ClientCtx,
    cookies: actix_session::Session,
    user_id: web::Path<i32>,
    form: web::Form<NoteForm>,
) -> Result<impl Responder, Error> {
    let author_id = client.require_login()?;
    client.require_permission("moderate.notes.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let user_id = user_id.into_inner();

    // Validate content
    let content = form.content.trim();
    if content.is_empty() {
        return Err(error::ErrorBadRequest("Note content is required"));
    }
    if content.len() > 10000 {
        return Err(error::ErrorBadRequest("Note content is too long"));
    }

    // Verify user exists
    users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Create note
    let now = Utc::now().naive_utc();
    let note = moderator_notes::ActiveModel {
        user_id: Set(user_id),
        author_id: Set(Some(author_id)),
        content: Set(content.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    note.insert(db).await.map_err(|e| {
        log::error!("Failed to create note: {}", e);
        error::ErrorInternalServerError("Failed to create note")
    })?;

    log::info!(
        "Moderator note added for user {} by moderator {}",
        user_id,
        author_id
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/admin/users/{}/notes", user_id)))
        .finish())
}

/// POST /admin/notes/{id}/delete - Delete a moderator note
#[post("/admin/notes/{id}/delete")]
async fn delete_user_note(
    client: ClientCtx,
    cookies: actix_session::Session,
    note_id: web::Path<i32>,
    form: web::Form<ModerationForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("moderate.notes.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let note_id = note_id.into_inner();

    // Find the note to get user_id for redirect
    let note = moderator_notes::Entity::find_by_id(note_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch note: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Note not found"))?;

    let user_id = note.user_id;

    // Delete the note
    moderator_notes::Entity::delete_by_id(note_id)
        .exec(db)
        .await
        .map_err(|e| {
            log::error!("Failed to delete note: {}", e);
            error::ErrorInternalServerError("Failed to delete note")
        })?;

    log::info!(
        "Moderator note {} deleted by moderator {}",
        note_id,
        moderator_id
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/admin/users/{}/notes", user_id)))
        .finish())
}

// =============================================================================
// User Warnings
// =============================================================================

/// Warning display for templates
#[allow(dead_code)]
struct WarningDisplay {
    id: i32,
    issued_by_id: Option<i32>,
    issued_by_name: String,
    reason: String,
    points: i32,
    expires_at: Option<chrono::NaiveDateTime>,
    acknowledged_at: Option<chrono::NaiveDateTime>,
    created_at: chrono::NaiveDateTime,
    is_expired: bool,
}

#[derive(Template)]
#[template(path = "admin/user_warnings.html")]
struct UserWarningsTemplate {
    client: ClientCtx,
    user_id: i32,
    username: String,
    warning_points: i32,
    warnings: Vec<WarningDisplay>,
    can_issue: bool,
    can_delete: bool,
}

#[derive(Template)]
#[template(path = "admin/warning_form.html")]
struct WarningFormTemplate {
    client: ClientCtx,
    user_id: i32,
    username: String,
    error: Option<String>,
}

#[derive(Deserialize)]
struct WarningForm {
    csrf_token: String,
    reason: String,
    points: i32,
    expires_days: Option<i32>, // 0 or None = permanent
}

/// GET /admin/users/{id}/warnings - View warnings for a user
#[get("/admin/users/{id}/warnings")]
async fn view_user_warnings(
    client: ClientCtx,
    user_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    client.require_permission("moderate.warnings.view")?;

    let db = get_db_pool();
    let user_id = user_id.into_inner();
    let now = Utc::now().naive_utc();

    // Get user
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Get username
    let username = user_names::Entity::find()
        .filter(user_names::Column::UserId.eq(user_id))
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|un| un.name)
        .unwrap_or_else(|| format!("User #{}", user_id));

    // Check permissions
    let can_issue = client.can("moderate.warnings.issue");
    let can_delete = client.can("moderate.warnings.delete");

    // Get warnings
    let warning_models = user_warnings::Entity::find()
        .filter(user_warnings::Column::UserId.eq(user_id))
        .order_by_desc(user_warnings::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch warnings: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    // Build warning displays with issuer names
    let mut warnings = Vec::new();
    for warning in warning_models {
        let issued_by_name = if let Some(issuer_id) = warning.issued_by {
            user_names::Entity::find()
                .filter(user_names::Column::UserId.eq(issuer_id))
                .one(db)
                .await
                .ok()
                .flatten()
                .map(|un| un.name)
                .unwrap_or_else(|| format!("User #{}", issuer_id))
        } else {
            "Deleted User".to_string()
        };

        let is_expired = warning.expires_at.map(|exp| exp < now).unwrap_or(false);

        warnings.push(WarningDisplay {
            id: warning.id,
            issued_by_id: warning.issued_by,
            issued_by_name,
            reason: warning.reason,
            points: warning.points,
            expires_at: warning.expires_at,
            acknowledged_at: warning.acknowledged_at,
            created_at: warning.created_at,
            is_expired,
        });
    }

    Ok(UserWarningsTemplate {
        client,
        user_id,
        username,
        warning_points: user.warning_points,
        warnings,
        can_issue,
        can_delete,
    }
    .to_response())
}

/// GET /admin/users/{id}/warn - Show warning form
#[get("/admin/users/{id}/warn")]
async fn view_issue_warning_form(
    client: ClientCtx,
    user_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    client.require_permission("moderate.warnings.issue")?;

    let db = get_db_pool();
    let user_id = user_id.into_inner();

    // Verify user exists
    users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Get username
    let username = user_names::Entity::find()
        .filter(user_names::Column::UserId.eq(user_id))
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|un| un.name)
        .unwrap_or_else(|| format!("User #{}", user_id));

    Ok(WarningFormTemplate {
        client,
        user_id,
        username,
        error: None,
    }
    .to_response())
}

/// POST /admin/users/{id}/warn - Issue a warning
#[post("/admin/users/{id}/warn")]
async fn issue_warning(
    client: ClientCtx,
    cookies: actix_session::Session,
    config: web::Data<Arc<Config>>,
    user_id: web::Path<i32>,
    form: web::Form<WarningForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("moderate.warnings.issue")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let user_id = user_id.into_inner();
    let now = Utc::now().naive_utc();

    // Validate input
    let reason = form.reason.trim();
    if reason.is_empty() {
        return Err(error::ErrorBadRequest("Reason is required"));
    }
    if reason.len() > 5000 {
        return Err(error::ErrorBadRequest("Reason is too long"));
    }

    let points = form.points.clamp(1, 100);

    // Calculate expiration
    let expires_at = match form.expires_days {
        Some(days) if days > 0 => Some(now + Duration::days(days as i64)),
        _ => None, // Permanent warning
    };

    // Verify user exists
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Create warning
    let warning = user_warnings::ActiveModel {
        user_id: Set(user_id),
        issued_by: Set(Some(moderator_id)),
        reason: Set(reason.to_string()),
        points: Set(points),
        expires_at: Set(expires_at),
        created_at: Set(now),
        ..Default::default()
    };

    warning.insert(db).await.map_err(|e| {
        log::error!("Failed to create warning: {}", e);
        error::ErrorInternalServerError("Failed to create warning")
    })?;

    // Update user's warning points
    let new_points = user.warning_points + points;
    let mut active_user: users::ActiveModel = user.into();
    active_user.warning_points = Set(new_points);
    active_user.last_warning_at = Set(Some(now));
    active_user.update(db).await.map_err(|e| {
        log::error!("Failed to update user warning points: {}", e);
        error::ErrorInternalServerError("Failed to update user")
    })?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "issue_warning",
        "user",
        user_id,
        Some(reason),
    )
    .await?;

    log::info!(
        "Warning issued to user {} ({} points) by moderator {}. Total points: {}",
        user_id,
        points,
        moderator_id,
        new_points
    );

    // Check if user should be auto-banned
    let threshold = config.get_int("warning_threshold").unwrap_or(10) as i32;
    if new_points >= threshold {
        // Auto-ban the user
        let ban_days = config.get_int("warning_ban_duration_days").unwrap_or(7);
        let (expires_at, is_permanent) = if ban_days == 0 {
            (None, true)
        } else {
            (Some(now + Duration::days(ban_days)), false)
        };

        let ban = user_bans::ActiveModel {
            user_id: Set(user_id),
            banned_by: Set(Some(moderator_id)),
            reason: Set(format!(
                "Auto-ban: Warning points threshold ({}) reached",
                threshold
            )),
            expires_at: Set(expires_at),
            is_permanent: Set(is_permanent),
            created_at: Set(now),
            ..Default::default()
        };

        ban.insert(db).await.map_err(|e| {
            log::error!("Failed to create auto-ban: {}", e);
            error::ErrorInternalServerError("Failed to create ban")
        })?;

        log_moderation_action(
            db,
            moderator_id,
            "auto_ban_warning_threshold",
            "user",
            user_id,
            Some(&format!(
                "Warning points reached threshold: {} >= {}",
                new_points, threshold
            )),
        )
        .await?;

        log::info!(
            "User {} auto-banned due to warning threshold ({} >= {})",
            user_id,
            new_points,
            threshold
        );
    }

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/admin/users/{}/warnings", user_id)))
        .finish())
}

/// POST /admin/warnings/{id}/delete - Delete a warning
#[post("/admin/warnings/{id}/delete")]
async fn delete_warning(
    client: ClientCtx,
    cookies: actix_session::Session,
    warning_id: web::Path<i32>,
    form: web::Form<ModerationForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("moderate.warnings.delete")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let warning_id = warning_id.into_inner();

    // Find the warning
    let warning = user_warnings::Entity::find_by_id(warning_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch warning: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Warning not found"))?;

    let user_id = warning.user_id;
    let points = warning.points;

    // Get user to subtract points
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Delete the warning
    user_warnings::Entity::delete_by_id(warning_id)
        .exec(db)
        .await
        .map_err(|e| {
            log::error!("Failed to delete warning: {}", e);
            error::ErrorInternalServerError("Failed to delete warning")
        })?;

    // Subtract points from user
    let old_points = user.warning_points;
    let new_points = (old_points - points).max(0);
    let mut active_user: users::ActiveModel = user.into();
    active_user.warning_points = Set(new_points);
    active_user.update(db).await.map_err(|e| {
        log::error!("Failed to update user warning points: {}", e);
        error::ErrorInternalServerError("Failed to update user")
    })?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "delete_warning",
        "warning",
        warning_id,
        form.reason.as_deref(),
    )
    .await?;

    log::info!(
        "Warning {} deleted by moderator {}. User {} points: {} -> {}",
        warning_id,
        moderator_id,
        user_id,
        old_points,
        new_points
    );

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/admin/users/{}/warnings", user_id)))
        .finish())
}

// =============================================================================
// Approval Queue
// =============================================================================

/// Pending user display for templates
struct PendingUserDisplay {
    id: i32,
    username: String,
    email: Option<String>,
    created_at: chrono::NaiveDateTime,
}

#[derive(Template)]
#[template(path = "admin/approval_queue.html")]
struct ApprovalQueueTemplate {
    client: ClientCtx,
    pending_users: Vec<PendingUserDisplay>,
    can_manage: bool,
}

#[derive(Deserialize)]
struct RejectForm {
    csrf_token: String,
    reason: Option<String>,
}

/// GET /admin/approval-queue - View pending user registrations
#[get("/admin/approval-queue")]
async fn view_approval_queue(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("moderate.approval.view")?;

    let db = get_db_pool();
    let can_manage = client.can("moderate.approval.manage");

    // Get pending users
    let pending = users::Entity::find()
        .filter(users::Column::ApprovalStatus.eq(users::ApprovalStatus::Pending))
        .order_by_asc(users::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch pending users: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    // Build display list with usernames
    let mut pending_users = Vec::new();
    for user in pending {
        let username = user_names::Entity::find()
            .filter(user_names::Column::UserId.eq(user.id))
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|un| un.name)
            .unwrap_or_else(|| format!("User #{}", user.id));

        pending_users.push(PendingUserDisplay {
            id: user.id,
            username,
            email: user.email,
            created_at: user.created_at,
        });
    }

    Ok(ApprovalQueueTemplate {
        client,
        pending_users,
        can_manage,
    }
    .to_response())
}

/// POST /admin/users/{id}/approve - Approve a pending user
#[post("/admin/users/{id}/approve")]
async fn approve_user(
    client: ClientCtx,
    cookies: actix_session::Session,
    user_id: web::Path<i32>,
    form: web::Form<ModerationForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("moderate.approval.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let user_id = user_id.into_inner();
    let now = Utc::now().naive_utc();

    // Find the user
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Check if user is pending
    if user.approval_status != users::ApprovalStatus::Pending {
        return Err(error::ErrorBadRequest("User is not pending approval"));
    }

    // Approve the user
    let mut active_user: users::ActiveModel = user.into();
    active_user.approval_status = Set(users::ApprovalStatus::Approved);
    active_user.approved_at = Set(Some(now));
    active_user.approved_by = Set(Some(moderator_id));
    active_user.update(db).await.map_err(|e| {
        log::error!("Failed to approve user: {}", e);
        error::ErrorInternalServerError("Failed to approve user")
    })?;

    // Log moderation action
    log_moderation_action(db, moderator_id, "approve_user", "user", user_id, None).await?;

    log::info!("User {} approved by moderator {}", user_id, moderator_id);

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/approval-queue"))
        .finish())
}

/// POST /admin/users/{id}/reject - Reject a pending user
#[post("/admin/users/{id}/reject")]
async fn reject_user(
    client: ClientCtx,
    cookies: actix_session::Session,
    user_id: web::Path<i32>,
    form: web::Form<RejectForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("moderate.approval.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let user_id = user_id.into_inner();

    // Find the user
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Check if user is pending
    if user.approval_status != users::ApprovalStatus::Pending {
        return Err(error::ErrorBadRequest("User is not pending approval"));
    }

    // Reject the user
    let mut active_user: users::ActiveModel = user.into();
    active_user.approval_status = Set(users::ApprovalStatus::Rejected);
    active_user.rejection_reason = Set(form.reason.clone());
    active_user.update(db).await.map_err(|e| {
        log::error!("Failed to reject user: {}", e);
        error::ErrorInternalServerError("Failed to reject user")
    })?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "reject_user",
        "user",
        user_id,
        form.reason.as_deref(),
    )
    .await?;

    log::info!("User {} rejected by moderator {}", user_id, moderator_id);

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/approval-queue"))
        .finish())
}

// ============================================================================
// Mass Moderation Actions
// ============================================================================

/// Form for mass user actions
#[derive(Deserialize)]
struct MassUserActionForm {
    csrf_token: String,
    action: String,
    #[serde(default)]
    user_ids: Vec<i32>,
    reason: Option<String>,
    ban_duration_days: Option<i32>,
}

/// POST /admin/users/mass-action - Perform mass action on users
#[post("/admin/users/mass-action")]
async fn mass_user_action(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<MassUserActionForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("moderate.mass.users")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    if form.user_ids.is_empty() {
        return Err(error::ErrorBadRequest("No users selected"));
    }

    let db = get_db_pool();
    let now = Utc::now().naive_utc();

    match form.action.as_str() {
        "ban" => {
            // Mass ban users
            let duration_days = form.ban_duration_days.unwrap_or(7);
            let expires_at = if duration_days > 0 {
                Some(now + Duration::days(duration_days as i64))
            } else {
                None // Permanent
            };
            let is_permanent = expires_at.is_none();

            for user_id in &form.user_ids {
                // Skip self-ban
                if *user_id == moderator_id {
                    continue;
                }

                // Check if already banned
                let existing_ban = user_bans::Entity::find()
                    .filter(user_bans::Column::UserId.eq(*user_id))
                    .filter(
                        user_bans::Column::IsPermanent
                            .eq(true)
                            .or(user_bans::Column::ExpiresAt.gt(now)),
                    )
                    .one(db)
                    .await
                    .ok()
                    .flatten();

                if existing_ban.is_some() {
                    continue; // Already banned
                }

                // Create ban
                let ban = user_bans::ActiveModel {
                    user_id: Set(*user_id),
                    banned_by: Set(Some(moderator_id)),
                    reason: Set(form
                        .reason
                        .clone()
                        .unwrap_or_else(|| "Mass ban".to_string())),
                    is_permanent: Set(is_permanent),
                    expires_at: Set(expires_at),
                    created_at: Set(now),
                    ..Default::default()
                };
                let _ = ban.insert(db).await;

                // Log action
                let _ = log_moderation_action(
                    db,
                    moderator_id,
                    "mass_ban",
                    "user",
                    *user_id,
                    form.reason.as_deref(),
                )
                .await;
            }

            log::info!(
                "Mass ban of {} users by moderator {}",
                form.user_ids.len(),
                moderator_id
            );
        }
        "unban" => {
            // Mass unban users
            for user_id in &form.user_ids {
                // Find active bans
                let active_bans = user_bans::Entity::find()
                    .filter(user_bans::Column::UserId.eq(*user_id))
                    .filter(
                        user_bans::Column::IsPermanent
                            .eq(true)
                            .or(user_bans::Column::ExpiresAt.gt(now)),
                    )
                    .all(db)
                    .await
                    .unwrap_or_default();

                for ban in active_bans {
                    let mut active_ban: user_bans::ActiveModel = ban.into();
                    active_ban.expires_at = Set(Some(now));
                    active_ban.is_permanent = Set(false);
                    let _ = active_ban.update(db).await;
                }

                // Log action
                let _ =
                    log_moderation_action(db, moderator_id, "mass_unban", "user", *user_id, None)
                        .await;
            }

            log::info!(
                "Mass unban of {} users by moderator {}",
                form.user_ids.len(),
                moderator_id
            );
        }
        "verify_email" => {
            // Mass verify email
            for user_id in &form.user_ids {
                let user = users::Entity::find_by_id(*user_id)
                    .one(db)
                    .await
                    .ok()
                    .flatten();

                if let Some(user) = user {
                    if !user.email_verified {
                        let mut active_user: users::ActiveModel = user.into();
                        active_user.email_verified = Set(true);
                        let _ = active_user.update(db).await;

                        let _ = log_moderation_action(
                            db,
                            moderator_id,
                            "mass_verify_email",
                            "user",
                            *user_id,
                            None,
                        )
                        .await;
                    }
                }
            }

            log::info!(
                "Mass email verification of {} users by moderator {}",
                form.user_ids.len(),
                moderator_id
            );
        }
        "approve" => {
            // Mass approve pending users
            for user_id in &form.user_ids {
                let user = users::Entity::find_by_id(*user_id)
                    .one(db)
                    .await
                    .ok()
                    .flatten();

                if let Some(user) = user {
                    if user.approval_status == users::ApprovalStatus::Pending {
                        let mut active_user: users::ActiveModel = user.into();
                        active_user.approval_status = Set(users::ApprovalStatus::Approved);
                        active_user.approved_at = Set(Some(now));
                        active_user.approved_by = Set(Some(moderator_id));
                        let _ = active_user.update(db).await;

                        let _ = log_moderation_action(
                            db,
                            moderator_id,
                            "mass_approve",
                            "user",
                            *user_id,
                            None,
                        )
                        .await;
                    }
                }
            }

            log::info!(
                "Mass approval of {} users by moderator {}",
                form.user_ids.len(),
                moderator_id
            );
        }
        "delete" => {
            // Mass delete users - requires admin permission
            client.require_permission("admin.user.manage")?;

            for user_id in &form.user_ids {
                // Skip self-delete
                if *user_id == moderator_id {
                    continue;
                }

                let _ = users::Entity::delete_by_id(*user_id).exec(db).await;

                let _ = log_moderation_action(
                    db,
                    moderator_id,
                    "mass_delete",
                    "user",
                    *user_id,
                    form.reason.as_deref(),
                )
                .await;
            }

            log::info!(
                "Mass deletion of {} users by moderator {}",
                form.user_ids.len(),
                moderator_id
            );
        }
        _ => {
            return Err(error::ErrorBadRequest("Invalid action"));
        }
    }

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/users"))
        .finish())
}

// ============================================================================
// Permission Groups Management
// ============================================================================

/// Display data for a group in the list
struct GroupDisplay {
    id: i32,
    label: String,
    group_type: GroupType,
    is_system: bool,
    member_count: i64,
}

/// Template for listing groups
#[derive(Template)]
#[template(path = "admin/groups.html")]
struct GroupsTemplate {
    client: ClientCtx,
    groups: Vec<GroupDisplay>,
}

/// Permission display with current value for a group
struct PermissionDisplay {
    id: i32,
    label: String,
    value: String,
}

/// Category with permissions
#[allow(dead_code)]
struct CategoryDisplay {
    id: i32,
    label: String,
    permissions: Vec<PermissionDisplay>,
}

/// Template for creating a new group
#[derive(Template)]
#[template(path = "admin/group_form.html")]
struct GroupFormTemplate {
    client: ClientCtx,
    group: Option<groups::Model>,
    categories: Vec<CategoryDisplay>,
    is_edit: bool,
    is_system: bool,
}

/// Form for creating/updating a group
#[derive(Deserialize)]
struct GroupForm {
    csrf_token: String,
    label: String,
    #[serde(default)]
    permissions: std::collections::HashMap<String, String>,
}

/// GET /admin/groups - List all groups
#[get("/admin/groups")]
async fn view_groups(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("admin.permissions.manage")?;

    let db = get_db_pool();

    // Get all groups with member counts
    let all_groups = groups::Entity::find()
        .order_by_asc(groups::Column::Id)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch groups: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    let mut group_displays = Vec::new();
    for group in all_groups {
        // Count members in this group
        let member_count = user_groups::Entity::find()
            .filter(user_groups::Column::GroupId.eq(group.id))
            .count(db)
            .await
            .unwrap_or(0) as i64;

        let is_system = group.group_type != GroupType::Normal;

        group_displays.push(GroupDisplay {
            id: group.id,
            label: group.label,
            group_type: group.group_type,
            is_system,
            member_count,
        });
    }

    Ok(GroupsTemplate {
        client,
        groups: group_displays,
    }
    .to_response())
}

/// GET /admin/groups/new - Form to create a new group
#[get("/admin/groups/new")]
async fn view_create_group_form(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("admin.permissions.manage")?;

    let db = get_db_pool();

    // Get all permission categories with their permissions
    let categories = load_permission_categories(db).await?;

    Ok(GroupFormTemplate {
        client,
        group: None,
        categories,
        is_edit: false,
        is_system: false,
    }
    .to_response())
}

/// POST /admin/groups/new - Create a new group
#[post("/admin/groups/new")]
async fn create_group(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<GroupForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("admin.permissions.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();

    // Validate label
    let label = form.label.trim();
    if label.is_empty() {
        return Err(error::ErrorBadRequest("Group name cannot be empty"));
    }

    // Create the group
    let new_group = groups::ActiveModel {
        label: Set(label.to_string()),
        group_type: Set(GroupType::Normal),
        ..Default::default()
    };

    let group = new_group.insert(db).await.map_err(|e| {
        log::error!("Failed to create group: {}", e);
        error::ErrorInternalServerError("Failed to create group")
    })?;

    // Create a permission collection for this group
    let collection = permission_collections::ActiveModel {
        group_id: Set(Some(group.id)),
        user_id: Set(None),
        ..Default::default()
    };

    let collection = collection.insert(db).await.map_err(|e| {
        log::error!("Failed to create permission collection: {}", e);
        error::ErrorInternalServerError("Failed to create permission collection")
    })?;

    // Save permissions
    save_group_permissions(db, collection.id, &form.permissions).await?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "create_group",
        "group",
        group.id,
        Some(label),
    )
    .await?;

    log::info!("Group {} created by user {}", group.id, moderator_id);

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/admin/groups/{}/edit", group.id)))
        .finish())
}

/// GET /admin/groups/{id}/edit - Edit a group
#[get("/admin/groups/{id}/edit")]
async fn view_edit_group(
    client: ClientCtx,
    group_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    client.require_permission("admin.permissions.manage")?;

    let db = get_db_pool();
    let group_id = group_id.into_inner();

    // Find the group
    let group = groups::Entity::find_by_id(group_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch group: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Group not found"))?;

    let is_system = group.group_type != GroupType::Normal;

    // Get the permission collection for this group
    let collection = permission_collections::Entity::find()
        .filter(permission_collections::Column::GroupId.eq(group_id))
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch permission collection: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    // Load categories with current permission values
    let categories = load_permission_categories_with_values(db, collection.map(|c| c.id)).await?;

    Ok(GroupFormTemplate {
        client,
        group: Some(group),
        categories,
        is_edit: true,
        is_system,
    }
    .to_response())
}

/// POST /admin/groups/{id}/edit - Update a group
#[post("/admin/groups/{id}/edit")]
async fn update_group(
    client: ClientCtx,
    cookies: actix_session::Session,
    group_id: web::Path<i32>,
    form: web::Form<GroupForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("admin.permissions.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let group_id = group_id.into_inner();

    // Find the group
    let group = groups::Entity::find_by_id(group_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch group: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Group not found"))?;

    // Update group label (only for non-system groups)
    if group.group_type == GroupType::Normal {
        let label = form.label.trim();
        if !label.is_empty() {
            let mut active_group: groups::ActiveModel = group.into();
            active_group.label = Set(label.to_string());
            active_group.update(db).await.map_err(|e| {
                log::error!("Failed to update group: {}", e);
                error::ErrorInternalServerError("Failed to update group")
            })?;
        }
    }

    // Get or create permission collection
    let collection = permission_collections::Entity::find()
        .filter(permission_collections::Column::GroupId.eq(group_id))
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch permission collection: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    let collection_id = match collection {
        Some(c) => c.id,
        None => {
            // Create collection if it doesn't exist
            let new_collection = permission_collections::ActiveModel {
                group_id: Set(Some(group_id)),
                user_id: Set(None),
                ..Default::default()
            };
            let c = new_collection.insert(db).await.map_err(|e| {
                log::error!("Failed to create permission collection: {}", e);
                error::ErrorInternalServerError("Failed to create permission collection")
            })?;
            c.id
        }
    };

    // Save permissions
    save_group_permissions(db, collection_id, &form.permissions).await?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "update_group",
        "group",
        group_id,
        Some(&form.label),
    )
    .await?;

    log::info!("Group {} updated by user {}", group_id, moderator_id);

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/admin/groups/{}/edit", group_id)))
        .finish())
}

/// Form for deleting a group
#[derive(Deserialize)]
struct DeleteGroupForm {
    csrf_token: String,
}

/// POST /admin/groups/{id}/delete - Delete a group
#[post("/admin/groups/{id}/delete")]
async fn delete_group(
    client: ClientCtx,
    cookies: actix_session::Session,
    group_id: web::Path<i32>,
    form: web::Form<DeleteGroupForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("admin.permissions.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let group_id = group_id.into_inner();

    // Find the group
    let group = groups::Entity::find_by_id(group_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch group: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Group not found"))?;

    // Cannot delete system groups
    if group.group_type != GroupType::Normal {
        return Err(error::ErrorBadRequest("Cannot delete system groups"));
    }

    let group_label = group.label.clone();

    // Delete the group (cascades to user_groups and permission_collections)
    groups::Entity::delete_by_id(group_id)
        .exec(db)
        .await
        .map_err(|e| {
            log::error!("Failed to delete group: {}", e);
            error::ErrorInternalServerError("Failed to delete group")
        })?;

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "delete_group",
        "group",
        group_id,
        Some(&group_label),
    )
    .await?;

    log::info!("Group {} deleted by user {}", group_id, moderator_id);

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/admin/groups"))
        .finish())
}

/// Helper to load permission categories
async fn load_permission_categories(
    db: &DatabaseConnection,
) -> Result<Vec<CategoryDisplay>, Error> {
    load_permission_categories_with_values(db, None).await
}

/// Helper to load permission categories with current values for a collection
async fn load_permission_categories_with_values(
    db: &DatabaseConnection,
    collection_id: Option<i32>,
) -> Result<Vec<CategoryDisplay>, Error> {
    // Get all categories
    let categories = permission_categories::Entity::find()
        .order_by_asc(permission_categories::Column::Sort)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch permission categories: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    // Get all permissions
    let all_permissions = permissions::Entity::find()
        .order_by_asc(permissions::Column::Sort)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch permissions: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    // Get current values if collection_id provided
    let current_values: std::collections::HashMap<i32, String> = if let Some(cid) = collection_id {
        permission_values::Entity::find()
            .filter(permission_values::Column::CollectionId.eq(cid))
            .all(db)
            .await
            .map_err(|e| {
                log::error!("Failed to fetch permission values: {}", e);
                error::ErrorInternalServerError("Database error")
            })?
            .into_iter()
            .map(|pv| {
                let value_str = match pv.value {
                    Flag::YES => "yes",
                    Flag::NO => "no",
                    Flag::NEVER => "never",
                    Flag::DEFAULT => "default",
                };
                (pv.permission_id, value_str.to_string())
            })
            .collect()
    } else {
        std::collections::HashMap::new()
    };

    // Build category displays
    let mut category_displays = Vec::new();
    for cat in categories {
        let perms: Vec<PermissionDisplay> = all_permissions
            .iter()
            .filter(|p| p.category_id == cat.id)
            .map(|p| PermissionDisplay {
                id: p.id,
                label: p.label.clone(),
                value: current_values
                    .get(&p.id)
                    .cloned()
                    .unwrap_or_else(|| "default".to_string()),
            })
            .collect();

        if !perms.is_empty() {
            category_displays.push(CategoryDisplay {
                id: cat.id,
                label: cat.label,
                permissions: perms,
            });
        }
    }

    Ok(category_displays)
}

/// Helper to save group permissions
async fn save_group_permissions(
    db: &DatabaseConnection,
    collection_id: i32,
    permissions_map: &std::collections::HashMap<String, String>,
) -> Result<(), Error> {
    // Delete existing permission values for this collection
    permission_values::Entity::delete_many()
        .filter(permission_values::Column::CollectionId.eq(collection_id))
        .exec(db)
        .await
        .map_err(|e| {
            log::error!("Failed to delete old permission values: {}", e);
            error::ErrorInternalServerError("Failed to update permissions")
        })?;

    // Insert new permission values
    for (perm_id_str, value_str) in permissions_map {
        let perm_id: i32 = match perm_id_str.parse() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let flag = match value_str.as_str() {
            "yes" => Flag::YES,
            "no" => Flag::NO,
            "never" => Flag::NEVER,
            _ => continue, // Skip "default" values - don't store them
        };

        let pv = permission_values::ActiveModel {
            permission_id: Set(perm_id),
            collection_id: Set(collection_id),
            value: Set(flag),
        };

        let _ = pv.insert(db).await;
    }

    Ok(())
}

// ============================================================================
// Reaction Types Management
// ============================================================================

#[derive(Template)]
#[template(path = "admin/reaction_types.html")]
struct ReactionTypesTemplate {
    client: ClientCtx,
    reaction_types: Vec<reaction_types::Model>,
}

#[derive(Template)]
#[template(path = "admin/reaction_type_form.html")]
struct ReactionTypeFormTemplate {
    client: ClientCtx,
    reaction_type: Option<reaction_types::Model>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct ReactionTypeForm {
    csrf_token: String,
    name: String,
    emoji: String,
    display_order: i32,
    is_positive: Option<String>,
    is_active: Option<String>,
    reputation_value: i32,
}

/// GET /admin/reaction-types - List all reaction types
#[get("/admin/reaction-types")]
async fn view_reaction_types(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("admin.settings.manage")?;

    let db = get_db_pool();

    let reaction_types = reaction_types::Entity::find()
        .order_by_asc(reaction_types::Column::DisplayOrder)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch reaction types: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    Ok(ReactionTypesTemplate {
        client,
        reaction_types,
    }
    .to_response())
}

/// GET /admin/reaction-types/new - Show form to create new reaction type
#[get("/admin/reaction-types/new")]
async fn view_create_reaction_type_form(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("admin.settings.manage")?;

    Ok(ReactionTypeFormTemplate {
        client,
        reaction_type: None,
        error: None,
    }
    .to_response())
}

/// POST /admin/reaction-types - Create a new reaction type
#[post("/admin/reaction-types")]
async fn create_reaction_type(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<ReactionTypeForm>,
) -> Result<impl Responder, Error> {
    client.require_login()?;
    client.require_permission("admin.settings.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();

    // Validate input
    if form.name.trim().is_empty() {
        return Ok(ReactionTypeFormTemplate {
            client,
            reaction_type: None,
            error: Some("Name is required".to_string()),
        }
        .to_response());
    }

    if form.emoji.trim().is_empty() {
        return Ok(ReactionTypeFormTemplate {
            client,
            reaction_type: None,
            error: Some("Emoji is required".to_string()),
        }
        .to_response());
    }

    let new_reaction_type = reaction_types::ActiveModel {
        name: Set(form.name.trim().to_string()),
        emoji: Set(form.emoji.trim().to_string()),
        display_order: Set(form.display_order),
        is_positive: Set(form.is_positive.is_some()),
        is_active: Set(form.is_active.is_some()),
        reputation_value: Set(form.reputation_value),
        ..Default::default()
    };

    new_reaction_type.insert(db).await.map_err(|e| {
        log::error!("Failed to create reaction type: {}", e);
        error::ErrorInternalServerError("Failed to create reaction type")
    })?;

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", "/admin/reaction-types"))
        .finish())
}

/// GET /admin/reaction-types/{id}/edit - Show form to edit reaction type
#[get("/admin/reaction-types/{id}/edit")]
async fn view_edit_reaction_type(
    client: ClientCtx,
    path: web::Path<i32>,
) -> Result<impl Responder, Error> {
    client.require_permission("admin.settings.manage")?;

    let id = path.into_inner();
    let db = get_db_pool();

    let reaction_type = reaction_types::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch reaction type: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Reaction type not found"))?;

    Ok(ReactionTypeFormTemplate {
        client,
        reaction_type: Some(reaction_type),
        error: None,
    }
    .to_response())
}

/// POST /admin/reaction-types/{id} - Update a reaction type
#[post("/admin/reaction-types/{id}")]
async fn update_reaction_type(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<ReactionTypeForm>,
) -> Result<impl Responder, Error> {
    client.require_login()?;
    client.require_permission("admin.settings.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let id = path.into_inner();
    let db = get_db_pool();

    // Fetch existing reaction type
    let existing = reaction_types::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch reaction type: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Reaction type not found"))?;

    // Validate input
    if form.name.trim().is_empty() {
        return Ok(ReactionTypeFormTemplate {
            client,
            reaction_type: Some(existing),
            error: Some("Name is required".to_string()),
        }
        .to_response());
    }

    if form.emoji.trim().is_empty() {
        return Ok(ReactionTypeFormTemplate {
            client,
            reaction_type: Some(existing),
            error: Some("Emoji is required".to_string()),
        }
        .to_response());
    }

    let mut updated: reaction_types::ActiveModel = existing.into();
    updated.name = Set(form.name.trim().to_string());
    updated.emoji = Set(form.emoji.trim().to_string());
    updated.display_order = Set(form.display_order);
    updated.is_positive = Set(form.is_positive.is_some());
    updated.is_active = Set(form.is_active.is_some());
    updated.reputation_value = Set(form.reputation_value);

    updated.update(db).await.map_err(|e| {
        log::error!("Failed to update reaction type: {}", e);
        error::ErrorInternalServerError("Failed to update reaction type")
    })?;

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", "/admin/reaction-types"))
        .finish())
}

// ============================================================================
// Badge Management
// ============================================================================

#[derive(Template)]
#[template(path = "admin/badges.html")]
struct BadgesTemplate {
    client: ClientCtx,
    badges: Vec<badges::Model>,
}

#[derive(Template)]
#[template(path = "admin/badge_form.html")]
struct BadgeFormTemplate {
    client: ClientCtx,
    badge: Option<badges::Model>,
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/badge_award.html")]
struct BadgeAwardTemplate {
    client: ClientCtx,
    badge: badges::Model,
    current_holders: Vec<BadgeHolder>,
    error: Option<String>,
    success: Option<String>,
}

#[derive(Debug)]
struct BadgeHolder {
    user_id: i32,
    username: String,
    awarded_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
struct BadgeForm {
    csrf_token: String,
    name: String,
    slug: String,
    description: Option<String>,
    icon: String,
    color: Option<String>,
    condition_type: String,
    condition_value: Option<i32>,
    display_order: i32,
    is_active: Option<String>,
}

#[derive(Deserialize)]
struct AwardBadgeForm {
    csrf_token: String,
    username: String,
}

#[derive(Deserialize)]
struct RevokeBadgeForm {
    csrf_token: String,
    user_id: i32,
}

/// GET /admin/badges - List all badges
#[get("/admin/badges")]
async fn view_badges(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("admin.badges.manage")?;

    let db = get_db_pool();

    let all_badges = badges::Entity::find()
        .order_by_asc(badges::Column::DisplayOrder)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch badges: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    Ok(BadgesTemplate {
        client,
        badges: all_badges,
    }
    .to_response())
}

/// GET /admin/badges/new - Show form to create new badge
#[get("/admin/badges/new")]
async fn view_create_badge_form(client: ClientCtx) -> Result<impl Responder, Error> {
    client.require_permission("admin.badges.manage")?;

    Ok(BadgeFormTemplate {
        client,
        badge: None,
        error: None,
    }
    .to_response())
}

/// POST /admin/badges - Create a new badge
#[post("/admin/badges")]
async fn create_badge(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<BadgeForm>,
) -> Result<impl Responder, Error> {
    client.require_login()?;
    client.require_permission("admin.badges.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();

    // Validate input
    if form.name.trim().is_empty() {
        return Ok(BadgeFormTemplate {
            client,
            badge: None,
            error: Some("Name is required".to_string()),
        }
        .to_response());
    }

    if form.slug.trim().is_empty() {
        return Ok(BadgeFormTemplate {
            client,
            badge: None,
            error: Some("Slug is required".to_string()),
        }
        .to_response());
    }

    // Parse condition type
    let condition_type = match form.condition_type.as_str() {
        "manual" => badges::BadgeConditionType::Manual,
        "post_count" => badges::BadgeConditionType::PostCount,
        "thread_count" => badges::BadgeConditionType::ThreadCount,
        "time_member" => badges::BadgeConditionType::TimeMember,
        "reputation" => badges::BadgeConditionType::Reputation,
        _ => badges::BadgeConditionType::Manual,
    };

    let new_badge = badges::ActiveModel {
        name: Set(form.name.trim().to_string()),
        slug: Set(form.slug.trim().to_lowercase().replace(' ', "-")),
        description: Set(form.description.clone().filter(|s| !s.trim().is_empty())),
        icon: Set(form.icon.trim().to_string()),
        color: Set(form.color.clone().filter(|s| !s.trim().is_empty())),
        condition_type: Set(condition_type),
        condition_value: Set(form.condition_value),
        display_order: Set(form.display_order),
        is_active: Set(form.is_active.is_some()),
        ..Default::default()
    };

    new_badge.insert(db).await.map_err(|e| {
        log::error!("Failed to create badge: {}", e);
        error::ErrorInternalServerError("Failed to create badge")
    })?;

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", "/admin/badges"))
        .finish())
}

/// GET /admin/badges/{id}/edit - Show form to edit badge
#[get("/admin/badges/{id}/edit")]
async fn view_edit_badge(client: ClientCtx, path: web::Path<i32>) -> Result<impl Responder, Error> {
    client.require_permission("admin.badges.manage")?;

    let id = path.into_inner();
    let db = get_db_pool();

    let badge = badges::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch badge: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Badge not found"))?;

    Ok(BadgeFormTemplate {
        client,
        badge: Some(badge),
        error: None,
    }
    .to_response())
}

/// POST /admin/badges/{id} - Update a badge
#[post("/admin/badges/{id}")]
async fn update_badge(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<BadgeForm>,
) -> Result<impl Responder, Error> {
    client.require_login()?;
    client.require_permission("admin.badges.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let id = path.into_inner();
    let db = get_db_pool();

    // Fetch existing badge
    let existing = badges::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch badge: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Badge not found"))?;

    // Validate input
    if form.name.trim().is_empty() {
        return Ok(BadgeFormTemplate {
            client,
            badge: Some(existing),
            error: Some("Name is required".to_string()),
        }
        .to_response());
    }

    // Parse condition type
    let condition_type = match form.condition_type.as_str() {
        "manual" => badges::BadgeConditionType::Manual,
        "post_count" => badges::BadgeConditionType::PostCount,
        "thread_count" => badges::BadgeConditionType::ThreadCount,
        "time_member" => badges::BadgeConditionType::TimeMember,
        "reputation" => badges::BadgeConditionType::Reputation,
        _ => badges::BadgeConditionType::Manual,
    };

    let mut updated: badges::ActiveModel = existing.into();
    updated.name = Set(form.name.trim().to_string());
    updated.slug = Set(form.slug.trim().to_lowercase().replace(' ', "-"));
    updated.description = Set(form.description.clone().filter(|s| !s.trim().is_empty()));
    updated.icon = Set(form.icon.trim().to_string());
    updated.color = Set(form.color.clone().filter(|s| !s.trim().is_empty()));
    updated.condition_type = Set(condition_type);
    updated.condition_value = Set(form.condition_value);
    updated.display_order = Set(form.display_order);
    updated.is_active = Set(form.is_active.is_some());

    updated.update(db).await.map_err(|e| {
        log::error!("Failed to update badge: {}", e);
        error::ErrorInternalServerError("Failed to update badge")
    })?;

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", "/admin/badges"))
        .finish())
}

/// GET /admin/badges/{id}/award - Show form to award badge to users
#[get("/admin/badges/{id}/award")]
async fn view_award_badge_form(
    client: ClientCtx,
    path: web::Path<i32>,
) -> Result<impl Responder, Error> {
    client.require_permission("admin.badges.manage")?;

    let id = path.into_inner();
    let db = get_db_pool();

    let badge = badges::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch badge: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Badge not found"))?;

    // Get current badge holders
    let holders = get_badge_holders(db, id).await.map_err(|e| {
        log::error!("Failed to fetch badge holders: {}", e);
        error::ErrorInternalServerError("Database error")
    })?;

    Ok(BadgeAwardTemplate {
        client,
        badge,
        current_holders: holders,
        error: None,
        success: None,
    }
    .to_response())
}

async fn get_badge_holders(
    db: &DatabaseConnection,
    badge_id: i32,
) -> Result<Vec<BadgeHolder>, sea_orm::DbErr> {
    use sea_orm::FromQueryResult;

    #[derive(Debug, FromQueryResult)]
    struct HolderRow {
        user_id: i32,
        username: String,
        awarded_at: chrono::DateTime<chrono::Utc>,
    }

    let rows = HolderRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        r#"
        SELECT ub.user_id, un.name as username, ub.awarded_at
        FROM user_badges ub
        JOIN user_names un ON un.user_id = ub.user_id
        WHERE ub.badge_id = $1
        ORDER BY ub.awarded_at DESC
        "#,
        vec![badge_id.into()],
    ))
    .all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| BadgeHolder {
            user_id: r.user_id,
            username: r.username,
            awarded_at: r.awarded_at,
        })
        .collect())
}

/// POST /admin/badges/{id}/award - Award badge to a user
#[post("/admin/badges/{id}/award")]
async fn award_badge_to_user(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<AwardBadgeForm>,
) -> Result<impl Responder, Error> {
    client.require_login()?;
    client.require_permission("admin.badges.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let badge_id = path.into_inner();
    let db = get_db_pool();

    // Fetch badge
    let badge = badges::Entity::find_by_id(badge_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch badge: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Badge not found"))?;

    // Look up user by username
    let user_id = crate::user::get_user_id_from_name(db, &form.username).await;

    let holders = get_badge_holders(db, badge_id).await.map_err(|e| {
        log::error!("Failed to fetch badge holders: {}", e);
        error::ErrorInternalServerError("Database error")
    })?;

    let user_id = match user_id {
        Some(id) => id,
        None => {
            return Ok(BadgeAwardTemplate {
                client,
                badge,
                current_holders: holders,
                error: Some(format!("User '{}' not found", form.username)),
                success: None,
            }
            .to_response());
        }
    };

    // Award the badge
    let awarded_by = client.get_id();
    match crate::badges::award_badge(db, user_id, badge_id, awarded_by).await {
        Ok(true) => {
            // Refresh holders list
            let holders = get_badge_holders(db, badge_id).await.map_err(|e| {
                log::error!("Failed to fetch badge holders: {}", e);
                error::ErrorInternalServerError("Database error")
            })?;

            Ok(BadgeAwardTemplate {
                client,
                badge,
                current_holders: holders,
                error: None,
                success: Some(format!("Badge awarded to {}", form.username)),
            }
            .to_response())
        }
        Ok(false) => Ok(BadgeAwardTemplate {
            client,
            badge,
            current_holders: holders,
            error: Some(format!("User '{}' already has this badge", form.username)),
            success: None,
        }
        .to_response()),
        Err(e) => {
            log::error!("Failed to award badge: {}", e);
            Ok(BadgeAwardTemplate {
                client,
                badge,
                current_holders: holders,
                error: Some("Failed to award badge".to_string()),
                success: None,
            }
            .to_response())
        }
    }
}

/// POST /admin/badges/{id}/revoke - Revoke badge from a user
#[post("/admin/badges/{id}/revoke")]
async fn revoke_badge_from_user(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<RevokeBadgeForm>,
) -> Result<impl Responder, Error> {
    client.require_login()?;
    client.require_permission("admin.badges.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let badge_id = path.into_inner();
    let db = get_db_pool();

    // Revoke the badge
    crate::badges::revoke_badge(db, form.user_id, badge_id)
        .await
        .map_err(|e| {
            log::error!("Failed to revoke badge: {}", e);
            error::ErrorInternalServerError("Failed to revoke badge")
        })?;

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", format!("/admin/badges/{}/award", badge_id)))
        .finish())
}

// ============================================================================
// Forum Permissions Management
// ============================================================================

/// Group info for column headers
struct ForumPermGroupInfo {
    label: String,
}

/// Permission value for a specific group
struct ForumPermGroupValue {
    group_id: i32,
    value: String,
}

/// Permission row with values per group
struct ForumPermissionRow {
    id: i32,
    label: String,
    /// Values in same order as groups
    values: Vec<ForumPermGroupValue>,
}

/// Category with permissions for forum permission matrix
struct ForumPermCategoryDisplay {
    label: String,
    permissions: Vec<ForumPermissionRow>,
}

#[derive(Template)]
#[template(path = "admin/forum_permissions.html")]
struct ForumPermissionsTemplate {
    client: ClientCtx,
    forum: forums::Model,
    groups: Vec<ForumPermGroupInfo>,
    categories: Vec<ForumPermCategoryDisplay>,
}

/// Form for updating forum permissions
#[derive(Deserialize)]
struct ForumPermissionsForm {
    csrf_token: String,
    /// Map of "perm_{permission_id}_{group_id}" -> value
    #[serde(flatten)]
    permissions: std::collections::HashMap<String, String>,
}

/// GET /admin/forums/{id}/permissions - View/edit forum permissions
#[get("/admin/forums/{id}/permissions")]
async fn view_forum_permissions(
    client: ClientCtx,
    forum_id: web::Path<i32>,
) -> Result<impl Responder, Error> {
    client.require_permission("admin.permissions.manage")?;

    let db = get_db_pool();
    let forum_id = forum_id.into_inner();

    // Find the forum
    let forum = forums::Entity::find_by_id(forum_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch forum: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Forum not found"))?;

    // Get all groups
    let all_groups = groups::Entity::find()
        .order_by_asc(groups::Column::Id)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch groups: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    let groups_info: Vec<ForumPermGroupInfo> = all_groups
        .iter()
        .map(|g| ForumPermGroupInfo {
            label: g.label.clone(),
        })
        .collect();

    // Get all categories
    let categories = permission_categories::Entity::find()
        .order_by_asc(permission_categories::Column::Sort)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch permission categories: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    // Get all permissions
    let all_permissions = permissions::Entity::find()
        .order_by_asc(permissions::Column::Sort)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch permissions: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    // Get forum permission collections for this forum
    let forum_perms = forum_permissions::Entity::find()
        .filter(forum_permissions::Column::ForumId.eq(forum_id))
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch forum permissions: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    // Build a map of collection_id -> group_id for this forum's collections
    let collection_ids: Vec<i32> = forum_perms.iter().map(|fp| fp.collection_id).collect();

    let collections = if !collection_ids.is_empty() {
        permission_collections::Entity::find()
            .filter(permission_collections::Column::Id.is_in(collection_ids.clone()))
            .all(db)
            .await
            .map_err(|e| {
                log::error!("Failed to fetch permission collections: {}", e);
                error::ErrorInternalServerError("Database error")
            })?
    } else {
        Vec::new()
    };

    // Map: group_id -> collection_id
    let group_to_collection: std::collections::HashMap<i32, i32> = collections
        .into_iter()
        .filter_map(|c| c.group_id.map(|gid| (gid, c.id)))
        .collect();

    // Map: collection_id -> group_id (inverse)
    let collection_to_group: std::collections::HashMap<i32, i32> = group_to_collection
        .iter()
        .map(|(&gid, &cid)| (cid, gid))
        .collect();

    // Get permission values for these collections
    let perm_values = if !collection_ids.is_empty() {
        permission_values::Entity::find()
            .filter(permission_values::Column::CollectionId.is_in(collection_ids))
            .all(db)
            .await
            .map_err(|e| {
                log::error!("Failed to fetch permission values: {}", e);
                error::ErrorInternalServerError("Database error")
            })?
    } else {
        Vec::new()
    };

    // Build map: (group_id, permission_id) -> value_string
    let mut value_map: std::collections::HashMap<(i32, i32), String> =
        std::collections::HashMap::new();
    for pv in perm_values {
        if let Some(&group_id) = collection_to_group.get(&pv.collection_id) {
            let value_str = match pv.value {
                Flag::YES => "yes",
                Flag::NO => "no",
                Flag::NEVER => "never",
                Flag::DEFAULT => "default",
            };
            value_map.insert((group_id, pv.permission_id), value_str.to_string());
        }
    }

    // Build category displays
    let mut category_displays = Vec::new();
    for cat in categories {
        let perms: Vec<ForumPermissionRow> = all_permissions
            .iter()
            .filter(|p| p.category_id == cat.id)
            .map(|p| {
                let values: Vec<ForumPermGroupValue> = all_groups
                    .iter()
                    .map(|group| {
                        let value = value_map
                            .get(&(group.id, p.id))
                            .cloned()
                            .unwrap_or_else(|| "default".to_string());
                        ForumPermGroupValue {
                            group_id: group.id,
                            value,
                        }
                    })
                    .collect();
                ForumPermissionRow {
                    id: p.id,
                    label: p.label.clone(),
                    values,
                }
            })
            .collect();

        if !perms.is_empty() {
            category_displays.push(ForumPermCategoryDisplay {
                label: cat.label,
                permissions: perms,
            });
        }
    }

    Ok(ForumPermissionsTemplate {
        client,
        forum,
        groups: groups_info,
        categories: category_displays,
    }
    .to_response())
}

/// POST /admin/forums/{id}/permissions - Save forum permissions
#[post("/admin/forums/{id}/permissions")]
async fn save_forum_permissions(
    client: ClientCtx,
    cookies: actix_session::Session,
    forum_id: web::Path<i32>,
    form: web::Form<ForumPermissionsForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("admin.permissions.manage")?;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let forum_id = forum_id.into_inner();

    // Verify forum exists
    let forum = forums::Entity::find_by_id(forum_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch forum: {}", e);
            error::ErrorInternalServerError("Database error")
        })?
        .ok_or_else(|| error::ErrorNotFound("Forum not found"))?;

    // Get all groups
    let all_groups = groups::Entity::find().all(db).await.map_err(|e| {
        log::error!("Failed to fetch groups: {}", e);
        error::ErrorInternalServerError("Database error")
    })?;

    // Parse form data: perm_{permission_id}_{group_id} -> value
    // Build map: group_id -> HashMap<permission_id, value>
    let mut group_permissions: std::collections::HashMap<
        i32,
        std::collections::HashMap<i32, String>,
    > = std::collections::HashMap::new();

    for (key, value) in &form.permissions {
        if !key.starts_with("perm_") {
            continue;
        }
        let parts: Vec<&str> = key.split('_').collect();
        if parts.len() != 3 {
            continue;
        }
        let perm_id: i32 = match parts[1].parse() {
            Ok(id) => id,
            Err(_) => continue,
        };
        let group_id: i32 = match parts[2].parse() {
            Ok(id) => id,
            Err(_) => continue,
        };
        group_permissions
            .entry(group_id)
            .or_insert_with(std::collections::HashMap::new)
            .insert(perm_id, value.clone());
    }

    // Get existing forum permission links
    let existing_forum_perms = forum_permissions::Entity::find()
        .filter(forum_permissions::Column::ForumId.eq(forum_id))
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch forum permissions: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    let existing_collection_ids: Vec<i32> =
        existing_forum_perms.iter().map(|fp| fp.collection_id).collect();

    // Get existing collections for these IDs
    let existing_collections = if !existing_collection_ids.is_empty() {
        permission_collections::Entity::find()
            .filter(permission_collections::Column::Id.is_in(existing_collection_ids))
            .all(db)
            .await
            .map_err(|e| {
                log::error!("Failed to fetch permission collections: {}", e);
                error::ErrorInternalServerError("Database error")
            })?
    } else {
        Vec::new()
    };

    // Map: group_id -> collection_id
    let mut group_to_collection: std::collections::HashMap<i32, i32> = existing_collections
        .into_iter()
        .filter_map(|c| c.group_id.map(|gid| (gid, c.id)))
        .collect();

    // For each group, update or create permission collection
    for group in &all_groups {
        let group_perms = match group_permissions.get(&group.id) {
            Some(perms) => perms,
            None => continue, // No permissions for this group
        };

        // Check if all values are "default" - if so, skip/delete
        let has_non_default = group_perms.values().any(|v| v != "default");

        if !has_non_default {
            // All default - delete collection if exists
            if let Some(collection_id) = group_to_collection.remove(&group.id) {
                // Delete permission values
                permission_values::Entity::delete_many()
                    .filter(permission_values::Column::CollectionId.eq(collection_id))
                    .exec(db)
                    .await
                    .ok();

                // Delete forum_permission link
                forum_permissions::Entity::delete_many()
                    .filter(forum_permissions::Column::ForumId.eq(forum_id))
                    .filter(forum_permissions::Column::CollectionId.eq(collection_id))
                    .exec(db)
                    .await
                    .ok();

                // Delete collection
                permission_collections::Entity::delete_by_id(collection_id)
                    .exec(db)
                    .await
                    .ok();
            }
            continue;
        }

        // Get or create collection for this group
        let collection_id = if let Some(&cid) = group_to_collection.get(&group.id) {
            cid
        } else {
            // Create new collection
            let new_collection = permission_collections::ActiveModel {
                group_id: Set(Some(group.id)),
                user_id: Set(None),
                ..Default::default()
            };
            let c = new_collection.insert(db).await.map_err(|e| {
                log::error!("Failed to create permission collection: {}", e);
                error::ErrorInternalServerError("Failed to create permission collection")
            })?;

            // Link to forum
            let fp = forum_permissions::ActiveModel {
                forum_id: Set(forum_id),
                collection_id: Set(c.id),
            };
            fp.insert(db).await.map_err(|e| {
                log::error!("Failed to link collection to forum: {}", e);
                error::ErrorInternalServerError("Failed to link collection to forum")
            })?;

            c.id
        };

        // Delete existing permission values for this collection
        permission_values::Entity::delete_many()
            .filter(permission_values::Column::CollectionId.eq(collection_id))
            .exec(db)
            .await
            .map_err(|e| {
                log::error!("Failed to delete old permission values: {}", e);
                error::ErrorInternalServerError("Failed to update permissions")
            })?;

        // Insert new permission values
        for (perm_id, value_str) in group_perms {
            let flag = match value_str.as_str() {
                "yes" => Flag::YES,
                "no" => Flag::NO,
                "never" => Flag::NEVER,
                _ => continue, // Skip "default" values
            };

            let pv = permission_values::ActiveModel {
                permission_id: Set(*perm_id),
                collection_id: Set(collection_id),
                value: Set(flag),
            };

            let _ = pv.insert(db).await;
        }
    }

    // Log moderation action
    log_moderation_action(
        db,
        moderator_id,
        "update_forum_permissions",
        "forum",
        forum_id,
        Some(&forum.label),
    )
    .await?;

    log::info!(
        "Forum {} permissions updated by user {}",
        forum_id,
        moderator_id
    );

    // Reload forum permissions cache so changes take effect immediately
    if let Err(e) = crate::permission::reload_forum_permissions().await {
        log::error!("Failed to reload forum permissions cache: {}", e);
        // Continue anyway - changes are saved, just need server restart
    }

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/admin/forums/{}/permissions", forum_id)))
        .finish())
}
