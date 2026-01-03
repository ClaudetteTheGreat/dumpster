use super::thread::get_url_for_pos;
use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{posts, ugc_deletions, ugc_revisions};
use crate::ugc::{create_ugc_revision, NewUgcPartial};
use crate::user::Profile as UserProfile;
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use chrono::prelude::Utc;
use sea_orm::{entity::*, query::*, sea_query::Expr};
use sea_orm::{DatabaseConnection, DbErr, FromQueryResult, QueryFilter};
use serde::Deserialize;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(delete_post)
        .service(destroy_post)
        .service(restore_post)
        .service(legal_hold_post)
        .service(remove_legal_hold_post)
        .service(edit_post)
        .service(update_post)
        .service(view_post_by_id)
        .service(view_post_in_thread)
        .service(view_post_history)
        .service(view_post_history_diff)
        .service(preview_bbcode);
}

#[derive(Deserialize)]
pub struct NewPostFormData {
    pub content: String,
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct DeletePostFormData {
    pub csrf_token: String,
    #[serde(default)]
    pub reason: Option<String>,
    /// Deletion type: "normal", "permanent", or omitted for normal
    #[serde(default)]
    pub deletion_type: Option<String>,
}

/// A fully joined struct representing the post model and its relational d&ata.
#[derive(Debug, FromQueryResult)]
pub struct PostForTemplate {
    pub id: i32,
    pub thread_id: i32,
    pub ugc_id: i32,
    pub user_id: Option<i32>,
    pub position: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    // join ugc
    pub ugc_revision_id: Option<i32>,
    pub content: Option<String>,
    pub ip_id: Option<i32>,
    // join ugc deletions
    pub deleted_by: Option<i32>,
    pub deleted_at: Option<chrono::NaiveDateTime>,
    pub deleted_reason: Option<String>,
    pub deletion_type: Option<String>,
}

impl PostForTemplate {}

#[derive(Template)]
#[template(path = "post_delete.html")]
pub struct PostDeleteTemplate<'a> {
    pub client: ClientCtx,
    pub post: &'a PostForTemplate,
}

#[derive(Template)]
#[template(path = "post_diff.html")]
pub struct PostDiffTemplate<'a> {
    pub client: ClientCtx,
    pub post: &'a PostForTemplate,
    pub diff: &'a Vec<dissimilar::Chunk<'a>>,
}

#[derive(Template)]
#[template(path = "post_history.html")]
pub struct PostHistoryTemplate<'a> {
    pub client: ClientCtx,
    pub post: &'a PostForTemplate,
    pub revisions: &'a Vec<(UgcRevisionLineItem, Option<UserProfile>)>,
}

#[derive(Template)]
#[template(path = "post_update.html")]
pub struct PostUpdateTemplate<'a> {
    pub client: ClientCtx,
    pub post: &'a PostForTemplate,
}

#[derive(FromQueryResult)]
pub struct UgcRevisionLineItem {
    pub id: i32,
    pub user_id: Option<i32>,
    pub ugc_id: i32,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Deserialize)]
pub struct UgcRevisionDiffFormData {
    pub new: i32,
    pub old: i32,
    pub csrf_token: String,
}

impl UgcRevisionLineItem {
    pub async fn get_for_ugc_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Vec<(Self, Option<UserProfile>)>, DbErr> {
        crate::user::find_also_user(
            ugc_revisions::Entity::find().filter(ugc_revisions::Column::UgcId.eq(id)),
            ugc_revisions::Column::UserId,
        )
        .into_model::<UgcRevisionLineItem, UserProfile>()
        .all(db)
        .await
    }
}

#[get("/posts/{post_id}/delete")]
pub async fn delete_post(client: ClientCtx, path: web::Path<i32>) -> Result<impl Responder, Error> {
    let db = get_db_pool();
    let (post, _user) = get_post_and_author_for_template(db, path.into_inner())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Post not found."))?;

    if !client.can_delete_post(&post) {
        return Err(error::ErrorForbidden(
            "You do not have permission to delete this post.",
        ));
    }

    Ok(PostDeleteTemplate {
        client,
        post: &post,
    }
    .to_response())
}

#[post("/posts/{post_id}/delete")]
pub async fn destroy_post(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<DeletePostFormData>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let (post, _user) = get_post_and_author_for_template(db, path.into_inner())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Post not found."))?;

    // Determine deletion type from form
    let deletion_type = match form.deletion_type.as_deref() {
        Some("permanent") => {
            // Permanent deletion requires moderator permission
            if !client.can("moderate.post.delete_permanent") {
                return Err(error::ErrorForbidden(
                    "You do not have permission to permanently delete posts.",
                ));
            }
            ugc_deletions::DeletionType::Permanent
        }
        _ => {
            // Normal deletion - check regular permissions
            if !client.can_delete_post(&post) {
                return Err(error::ErrorForbidden(
                    "You do not have permission to delete this post.",
                ));
            }
            ugc_deletions::DeletionType::Normal
        }
    };

    // Check if post is under legal hold - cannot be deleted except by admin
    if post.deletion_type.as_deref() == Some("legal_hold") {
        return Err(error::ErrorForbidden(
            "This post is under legal hold and cannot be deleted.",
        ));
    }

    if post.deleted_at.is_some() {
        // Post already deleted - update the deletion record
        let mut update = ugc_deletions::Entity::update_many()
            .col_expr(
                ugc_deletions::Column::DeletedById,
                Expr::value(client.get_id()),
            )
            .col_expr(
                ugc_deletions::Column::DeletionType,
                Expr::value(deletion_type.clone()),
            );

        if let Some(ref reason) = form.reason {
            update = update.col_expr(ugc_deletions::Column::Reason, Expr::value(reason.clone()));
        }

        update
            .filter(ugc_deletions::Column::Id.eq(post.ugc_id))
            .exec(db)
            .await
            .map_err(error::ErrorInternalServerError)?;
    } else {
        ugc_deletions::Entity::insert(ugc_deletions::ActiveModel {
            id: Set(post.ugc_id),
            user_id: Set(post.user_id),
            deleted_at: Set(Utc::now().naive_utc()),
            reason: Set(form.reason.clone()),
            deletion_type: Set(deletion_type.clone()),
            deleted_by_id: Set(client.get_id()),
            legal_hold_at: Set(None),
            legal_hold_by: Set(None),
            legal_hold_reason: Set(None),
        })
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

        // Spawn a thread to handle post-deletion work.
        let post_user_id = post.user_id;
        actix_web::rt::spawn(async move {
            use super::thread::update_thread_after_reply_is_deleted;

            // Update subsequent posts's position.
            let _post_res = posts::Entity::update_many()
                .col_expr(posts::Column::Position, Expr::cust("position - 1"))
                .filter(
                    Condition::all()
                        .add(posts::Column::ThreadId.eq(post.thread_id))
                        .add(posts::Column::Position.gt(post.position)),
                )
                .exec(db)
                .await
                .map_err(|e| log::error!("destroy_post thread: {}", e));

            // Update post_count and last_post info.
            let _thread_res = update_thread_after_reply_is_deleted(post.thread_id)
                .await
                .map_err(|e| log::error!("destroy_post thread: {}", e));

            // Decrement user's post_count (denormalized for performance)
            if let Some(user_id) = post_user_id {
                use crate::orm::users;
                let _ = users::Entity::update_many()
                    .col_expr(
                        users::Column::PostCount,
                        Expr::cust("GREATEST(post_count - 1, 0)"), // Prevent negative
                    )
                    .filter(users::Column::Id.eq(user_id))
                    .exec(db)
                    .await
                    .map_err(|e| log::error!("destroy_post user post_count: {}", e));
            }
        });
    }

    // For permanent deletion, also clear the content
    if deletion_type == ugc_deletions::DeletionType::Permanent {
        ugc_revisions::Entity::update_many()
            .col_expr(
                ugc_revisions::Column::Content,
                Expr::value("[Content permanently removed]".to_string()),
            )
            .filter(ugc_revisions::Column::UgcId.eq(post.ugc_id))
            .exec(db)
            .await
            .map_err(error::ErrorInternalServerError)?;
    }

    Ok(HttpResponse::Found()
        .append_header(("Location", get_url_for_pos(post.thread_id, post.position)))
        .finish())
}

/// Form data for restore/legal hold operations
#[derive(Deserialize)]
pub struct ModActionFormData {
    pub csrf_token: String,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Restore a soft-deleted post (moderators only)
#[post("/posts/{post_id}/restore")]
pub async fn restore_post(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<ModActionFormData>,
) -> Result<impl Responder, Error> {
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Require moderator permission
    if !client.can("moderate.post.restore") {
        return Err(error::ErrorForbidden(
            "You do not have permission to restore posts.",
        ));
    }

    let db = get_db_pool();
    let (post, _user) = get_post_and_author_for_template(db, path.into_inner())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Post not found."))?;

    // Check if post is deleted
    if post.deleted_at.is_none() {
        return Err(error::ErrorBadRequest("Post is not deleted."));
    }

    // Cannot restore permanently deleted posts
    if post.deletion_type.as_deref() == Some("permanent") {
        return Err(error::ErrorForbidden(
            "Permanently deleted posts cannot be restored.",
        ));
    }

    // Cannot restore posts under legal hold without admin permission
    if post.deletion_type.as_deref() == Some("legal_hold") {
        return Err(error::ErrorForbidden(
            "Posts under legal hold cannot be restored. Contact an administrator.",
        ));
    }

    // Delete the ugc_deletions record to restore the post
    ugc_deletions::Entity::delete_by_id(post.ugc_id)
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Update post positions and user post count
    let post_user_id = post.user_id;
    actix_web::rt::spawn(async move {
        // Increment positions of posts that came after this one
        let _post_res = posts::Entity::update_many()
            .col_expr(posts::Column::Position, Expr::cust("position + 1"))
            .filter(
                Condition::all()
                    .add(posts::Column::ThreadId.eq(post.thread_id))
                    .add(posts::Column::Position.gte(post.position)),
            )
            .exec(db)
            .await
            .map_err(|e| log::error!("restore_post thread: {}", e));

        // Increment user's post_count (denormalized for performance)
        if let Some(user_id) = post_user_id {
            use crate::orm::users;
            let _ = users::Entity::update_many()
                .col_expr(
                    users::Column::PostCount,
                    Expr::col(users::Column::PostCount).add(1),
                )
                .filter(users::Column::Id.eq(user_id))
                .exec(db)
                .await
                .map_err(|e| log::error!("restore_post user post_count: {}", e));
        }
    });

    Ok(HttpResponse::Found()
        .append_header(("Location", get_url_for_pos(post.thread_id, post.position)))
        .finish())
}

/// Place a legal hold on a post (admin only)
#[post("/posts/{post_id}/legal-hold")]
pub async fn legal_hold_post(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<ModActionFormData>,
) -> Result<impl Responder, Error> {
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Require admin permission
    if !client.can("admin.content.legal_hold") {
        return Err(error::ErrorForbidden(
            "You do not have permission to place legal holds.",
        ));
    }

    let db = get_db_pool();
    let (post, _user) = get_post_and_author_for_template(db, path.into_inner())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Post not found."))?;

    let now = Utc::now().naive_utc();

    if post.deleted_at.is_some() {
        // Post already deleted - update to legal hold
        ugc_deletions::Entity::update_many()
            .col_expr(
                ugc_deletions::Column::DeletionType,
                Expr::value(ugc_deletions::DeletionType::LegalHold),
            )
            .col_expr(ugc_deletions::Column::LegalHoldAt, Expr::value(now))
            .col_expr(
                ugc_deletions::Column::LegalHoldBy,
                Expr::value(client.get_id()),
            )
            .col_expr(
                ugc_deletions::Column::LegalHoldReason,
                Expr::value(form.reason.clone()),
            )
            .filter(ugc_deletions::Column::Id.eq(post.ugc_id))
            .exec(db)
            .await
            .map_err(error::ErrorInternalServerError)?;
    } else {
        // Create new deletion record with legal hold
        ugc_deletions::Entity::insert(ugc_deletions::ActiveModel {
            id: Set(post.ugc_id),
            user_id: Set(post.user_id),
            deleted_at: Set(now),
            reason: Set(Some("Legal hold".to_string())),
            deletion_type: Set(ugc_deletions::DeletionType::LegalHold),
            deleted_by_id: Set(client.get_id()),
            legal_hold_at: Set(Some(now)),
            legal_hold_by: Set(client.get_id()),
            legal_hold_reason: Set(form.reason.clone()),
        })
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;
    }

    Ok(HttpResponse::Found()
        .append_header(("Location", get_url_for_pos(post.thread_id, post.position)))
        .finish())
}

/// Remove a legal hold from a post (admin only)
#[post("/posts/{post_id}/remove-legal-hold")]
pub async fn remove_legal_hold_post(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<ModActionFormData>,
) -> Result<impl Responder, Error> {
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Require admin permission
    if !client.can("admin.content.remove_legal_hold") {
        return Err(error::ErrorForbidden(
            "You do not have permission to remove legal holds.",
        ));
    }

    let db = get_db_pool();
    let (post, _user) = get_post_and_author_for_template(db, path.into_inner())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Post not found."))?;

    // Check if post is under legal hold
    if post.deletion_type.as_deref() != Some("legal_hold") {
        return Err(error::ErrorBadRequest("Post is not under legal hold."));
    }

    // Change to normal deletion (still deleted, but can now be restored)
    ugc_deletions::Entity::update_many()
        .col_expr(
            ugc_deletions::Column::DeletionType,
            Expr::value(ugc_deletions::DeletionType::Normal),
        )
        .col_expr(
            ugc_deletions::Column::LegalHoldAt,
            Expr::value(Option::<chrono::NaiveDateTime>::None),
        )
        .col_expr(
            ugc_deletions::Column::LegalHoldBy,
            Expr::value(Option::<i32>::None),
        )
        .col_expr(
            ugc_deletions::Column::LegalHoldReason,
            Expr::value(Option::<String>::None),
        )
        .filter(ugc_deletions::Column::Id.eq(post.ugc_id))
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", get_url_for_pos(post.thread_id, post.position)))
        .finish())
}

#[get("/posts/{post_id}/edit")]
pub async fn edit_post(client: ClientCtx, path: web::Path<i32>) -> Result<impl Responder, Error> {
    let db = get_db_pool();
    let (post, _user) = get_post_and_author_for_template(db, path.into_inner())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Post not found."))?;

    if !client.can_update_post(&post) {
        return Err(error::ErrorForbidden(
            "You do not have permission to update this post.",
        ));
    }

    Ok(PostUpdateTemplate {
        client,
        post: &post,
    }
    .to_response())
}

#[post("/posts/{post_id}/edit")]
pub async fn update_post(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<NewPostFormData>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let (post, _user) = get_post_and_author_for_template(db, path.into_inner())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Post not found."))?;

    if !client.can_update_post(&post) {
        return Err(error::ErrorForbidden(
            "You do not have permission to update this post.",
        ));
    }

    create_ugc_revision(
        db,
        post.ugc_id,
        NewUgcPartial {
            ip_id: None,
            user_id: client.get_id(),
            content: &form.content,
        },
    )
    .await
    .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", get_url_for_pos(post.thread_id, post.position)))
        .finish())
}

#[get("/posts/{post_id}")]
pub async fn view_post_by_id(path: web::Path<i32>) -> Result<HttpResponse, Error> {
    view_post(path.into_inner()).await
}

// Permalink for a specific post.
#[get("/threads/{thread_id}/post-{post_id}")]
pub async fn view_post_in_thread(path: web::Path<(i32, i32)>) -> Result<HttpResponse, Error> {
    view_post(path.into_inner().1).await
}

/// Render post revisions as a line item table.
#[get("/posts/{post_id}/history")]
pub async fn view_post_history(
    client: ClientCtx,
    path: web::Path<i32>,
) -> Result<impl Responder, Error> {
    let db = get_db_pool();
    let (post, _user) = get_post_and_author_for_template(db, path.into_inner())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Post not found."))?;

    // Require authentication to view post edit history
    // This prevents exposing edit history to anonymous users
    client.require_login()?;

    let revisions = UgcRevisionLineItem::get_for_ugc_id(db, post.ugc_id)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(PostHistoryTemplate {
        client,
        post: &post,
        revisions: &revisions,
    }
    .to_response())
}
/// Render post edits with diffs highlighted.
#[post("/posts/{post_id}/history")]
pub async fn view_post_history_diff(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<UgcRevisionDiffFormData>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let (post, _user) = get_post_and_author_for_template(db, path.into_inner())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Post not found."))?;

    // Optimize: Fetch revisions but only use content field for diff
    let revision_models = ugc_revisions::Entity::find()
        .filter(ugc_revisions::Column::UgcId.eq(post.ugc_id))
        .filter(ugc_revisions::Column::Id.is_in([form.old, form.new]))
        .limit(2)
        .order_by_desc(ugc_revisions::Column::CreatedAt)
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Require authentication to view post diff history
    // This prevents exposing edit history to anonymous users
    client.require_login()?;

    if revision_models.len() < 2 {
        return Err(error::ErrorBadRequest(
            "Requested revisions either do not exist or are not attached to this resource as expected.",
        ));
    }

    let diff = dissimilar::diff(&revision_models[1].content, &revision_models[0].content);

    Ok(PostDiffTemplate {
        client,
        post: &post,
        diff: &diff,
    }
    .to_response())
}

/// Returns the result of a query selecting for a post by id with adjoined templating data.
/// TODO: It would be nice if this returned just the selector.
pub async fn get_post_and_author_for_template(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<(PostForTemplate, Option<UserProfile>)>, DbErr> {
    crate::user::find_also_user(
        posts::Entity::find_by_id(id)
            .left_join(ugc_revisions::Entity)
            .column_as(ugc_revisions::Column::Id, "ugc_revision_id")
            .column_as(ugc_revisions::Column::Content, "content")
            .column_as(ugc_revisions::Column::IpId, "ip_id")
            .column_as(ugc_revisions::Column::CreatedAt, "updated_at")
            .left_join(ugc_deletions::Entity)
            .column_as(ugc_deletions::Column::UserId, "deleted_by")
            .column_as(ugc_deletions::Column::DeletedAt, "deleted_at")
            .column_as(ugc_deletions::Column::Reason, "deleted_reason")
            // Cast enum to text for String field
            .column_as(
                Expr::cust("ugc_deletions.deletion_type::TEXT"),
                "deletion_type",
            ),
        posts::Column::UserId,
    )
    .into_model::<PostForTemplate, UserProfile>()
    .one(db)
    .await
}

pub async fn get_replies_and_author_for_template(
    db: &DatabaseConnection,
    id: i32,
    page: i32,
    posts_per_page: i32,
    show_pending: bool,
    current_user_id: Option<i32>,
) -> Result<Vec<(PostForTemplate, Option<UserProfile>)>, DbErr> {
    let mut query = crate::user::find_also_user(
        posts::Entity::find()
            .left_join(ugc_revisions::Entity)
            .column_as(ugc_revisions::Column::Id, "ugc_revision_id")
            .column_as(ugc_revisions::Column::Content, "content")
            .column_as(ugc_revisions::Column::IpId, "ip_id")
            .column_as(ugc_revisions::Column::CreatedAt, "updated_at")
            .left_join(ugc_deletions::Entity)
            .column_as(ugc_deletions::Column::UserId, "deleted_by")
            .column_as(ugc_deletions::Column::DeletedAt, "deleted_at")
            .column_as(ugc_deletions::Column::Reason, "deleted_reason")
            // Cast enum to text for String field
            .column_as(
                Expr::cust("ugc_deletions.deletion_type::TEXT"),
                "deletion_type",
            ),
        posts::Column::UserId,
    )
    .filter(posts::Column::ThreadId.eq(id))
    .filter(
        posts::Column::Position.between((page - 1) * posts_per_page + 1, page * posts_per_page),
    );

    // Filter out pending/rejected posts unless user is a moderator or the post author
    if !show_pending {
        // Only show approved posts, or user's own pending posts
        if let Some(user_id) = current_user_id {
            // Show approved posts OR user's own pending posts
            query =
                query.filter(
                    Condition::any()
                        .add(posts::Column::ModerationStatus.eq(posts::ModerationStatus::Approved))
                        .add(Condition::all().add(posts::Column::UserId.eq(user_id)).add(
                            posts::Column::ModerationStatus.eq(posts::ModerationStatus::Pending),
                        )),
                );
        } else {
            // Anonymous: only show approved posts
            query =
                query.filter(posts::Column::ModerationStatus.eq(posts::ModerationStatus::Approved));
        }
    }

    query
        .order_by_asc(posts::Column::Position)
        .order_by_asc(posts::Column::CreatedAt)
        .into_model::<PostForTemplate, UserProfile>()
        .all(db)
        .await
}

async fn view_post(id: i32) -> Result<HttpResponse, Error> {
    let post = posts::Entity::find_by_id(id)
        .one(get_db_pool())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Post not found."))?;

    Ok(HttpResponse::Found()
        .append_header(("Location", get_url_for_pos(post.thread_id, post.position)))
        .finish())
}

/// Preview form data
#[derive(Deserialize)]
pub struct PreviewFormData {
    pub content: String,
}

/// Preview BBCode as rendered HTML
/// POST /api/bbcode/preview
#[post("/api/bbcode/preview")]
pub async fn preview_bbcode(
    client: ClientCtx,
    form: web::Json<PreviewFormData>,
) -> Result<HttpResponse, Error> {
    // Require authentication to prevent abuse
    if !client.is_user() {
        return Err(error::ErrorUnauthorized("Must be logged in to preview"));
    }

    // Limit content size to prevent DoS
    let max_size = if client.can("moderate.post.edit") {
        100_000
    } else {
        50_000
    };

    if form.content.len() > max_size {
        return Err(error::ErrorBadRequest("Content too long"));
    }

    // Parse BBCode and return HTML
    let html = crate::bbcode::parse(&form.content);

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}
