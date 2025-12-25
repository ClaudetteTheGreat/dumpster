//! Report submission and management endpoints

use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{posts, report_reasons, reports, threads, user_names, users};
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use askama::Template;
use askama_actix::TemplateToResponse;
use chrono::Utc;
use sea_orm::{
    entity::*, query::*, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(get_report_reasons)
        .service(submit_report)
        .service(view_reports)
        .service(view_report)
        .service(update_report_status);
}

/// Response for report reasons
#[derive(Serialize)]
struct ReportReasonResponse {
    id: i32,
    name: String,
    label: String,
    description: Option<String>,
}

/// Get available report reasons
#[get("/api/report-reasons")]
async fn get_report_reasons(client: ClientCtx) -> Result<HttpResponse, Error> {
    // Require authentication
    if !client.is_user() {
        return Err(error::ErrorUnauthorized("Must be logged in"));
    }

    let db = get_db_pool();

    let reasons = report_reasons::Entity::find()
        .filter(report_reasons::Column::IsActive.eq(true))
        .order_by_asc(report_reasons::Column::DisplayOrder)
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let response: Vec<ReportReasonResponse> = reasons
        .into_iter()
        .map(|r| ReportReasonResponse {
            id: r.id,
            name: r.name,
            label: r.label,
            description: r.description,
        })
        .collect();

    Ok(HttpResponse::Ok().json(response))
}

#[derive(Deserialize)]
struct ReportForm {
    csrf_token: String,
    content_type: String,
    content_id: i32,
    reason: String,
    details: Option<String>,
}

#[derive(Serialize)]
struct ReportResponse {
    success: bool,
    message: String,
    report_id: Option<i32>,
}

/// Submit a report
#[post("/reports")]
async fn submit_report(
    client: ClientCtx,
    session: actix_session::Session,
    form: web::Form<ReportForm>,
) -> Result<HttpResponse, Error> {
    let reporter_id = client
        .get_id()
        .ok_or_else(|| error::ErrorUnauthorized("Must be logged in"))?;

    // Validate CSRF
    crate::middleware::csrf::validate_csrf_token(&session, &form.csrf_token)?;

    let db = get_db_pool();

    // Validate content type
    let valid_types = ["post", "thread", "user", "message"];
    if !valid_types.contains(&form.content_type.as_str()) {
        return Ok(HttpResponse::BadRequest().json(ReportResponse {
            success: false,
            message: "Invalid content type".to_string(),
            report_id: None,
        }));
    }

    // Validate reason exists
    let reason = report_reasons::Entity::find()
        .filter(report_reasons::Column::Name.eq(form.reason.clone()))
        .filter(report_reasons::Column::IsActive.eq(true))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    if reason.is_none() {
        return Ok(HttpResponse::BadRequest().json(ReportResponse {
            success: false,
            message: "Invalid report reason".to_string(),
            report_id: None,
        }));
    }

    // Check if user already has a pending report for this content
    let existing = reports::Entity::find()
        .filter(reports::Column::ReporterId.eq(reporter_id))
        .filter(reports::Column::ContentType.eq(form.content_type.clone()))
        .filter(reports::Column::ContentId.eq(form.content_id))
        .filter(reports::Column::Status.is_in(["open", "reviewed"]))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    if existing.is_some() {
        return Ok(HttpResponse::Conflict().json(ReportResponse {
            success: false,
            message: "You have already reported this content".to_string(),
            report_id: None,
        }));
    }

    // Validate that content exists
    let content_exists = match form.content_type.as_str() {
        "post" => posts::Entity::find_by_id(form.content_id)
            .one(db)
            .await
            .map_err(error::ErrorInternalServerError)?
            .is_some(),
        "thread" => threads::Entity::find_by_id(form.content_id)
            .one(db)
            .await
            .map_err(error::ErrorInternalServerError)?
            .is_some(),
        "user" => users::Entity::find_by_id(form.content_id)
            .one(db)
            .await
            .map_err(error::ErrorInternalServerError)?
            .is_some(),
        _ => false,
    };

    if !content_exists {
        return Ok(HttpResponse::NotFound().json(ReportResponse {
            success: false,
            message: "Content not found".to_string(),
            report_id: None,
        }));
    }

    // Require details for "other" reason
    if form.reason == "other" && form.details.as_ref().is_none_or(|d| d.trim().is_empty()) {
        return Ok(HttpResponse::BadRequest().json(ReportResponse {
            success: false,
            message: "Please provide details for 'Other' reports".to_string(),
            report_id: None,
        }));
    }

    // Create the report
    let now = Utc::now().naive_utc();
    let new_report = reports::ActiveModel {
        reporter_id: Set(reporter_id),
        content_type: Set(form.content_type.clone()),
        content_id: Set(form.content_id),
        reason: Set(form.reason.clone()),
        details: Set(form.details.clone()),
        status: Set("open".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let result = new_report
        .insert(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(ReportResponse {
        success: true,
        message: "Report submitted successfully. Thank you for helping keep the community safe."
            .to_string(),
        report_id: Some(result.id),
    }))
}

// ============ Admin/Moderator Views ============

#[derive(Template)]
#[template(path = "admin/reports.html")]
struct ReportsListTemplate {
    client: ClientCtx,
    reports: Vec<ReportView>,
    filter_status: String,
}

struct ReportView {
    id: i32,
    reporter_name: String,
    content_type: String,
    content_id: i32,
    reason: String,
    reason_label: String,
    details: Option<String>,
    status: String,
    created_at: chrono::NaiveDateTime,
    content_preview: String,
}

/// View all reports (moderators only)
#[get("/admin/reports")]
async fn view_reports(
    client: ClientCtx,
    query: web::Query<ReportsQuery>,
) -> Result<impl Responder, Error> {
    client.require_login()?;
    client.require_permission("moderate.reports.view")?;

    let db = get_db_pool();
    let status_filter = query.status.clone().unwrap_or_else(|| "open".to_string());

    // Get reports with filter
    let mut query_builder = reports::Entity::find().order_by_desc(reports::Column::CreatedAt);

    if status_filter != "all" {
        query_builder = query_builder.filter(reports::Column::Status.eq(status_filter.clone()));
    }

    let report_models = query_builder
        .limit(100)
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Build report views with related data
    let mut report_views = Vec::new();
    for report in report_models {
        // Get reporter name
        let reporter_name = user_names::Entity::find()
            .filter(user_names::Column::UserId.eq(report.reporter_id))
            .one(db)
            .await
            .map_err(error::ErrorInternalServerError)?
            .map(|u| u.name)
            .unwrap_or_else(|| "Unknown".to_string());

        // Get reason label
        let reason_label = report_reasons::Entity::find()
            .filter(report_reasons::Column::Name.eq(report.reason.clone()))
            .one(db)
            .await
            .map_err(error::ErrorInternalServerError)?
            .map(|r| r.label)
            .unwrap_or_else(|| report.reason.clone());

        // Get content preview
        let content_preview = match report.content_type.as_str() {
            "post" => {
                if let Some(post) = posts::Entity::find_by_id(report.content_id)
                    .one(db)
                    .await
                    .map_err(error::ErrorInternalServerError)?
                {
                    format!("Post #{} in thread #{}", post.id, post.thread_id)
                } else {
                    "Post deleted".to_string()
                }
            }
            "thread" => {
                if let Some(thread) = threads::Entity::find_by_id(report.content_id)
                    .one(db)
                    .await
                    .map_err(error::ErrorInternalServerError)?
                {
                    format!("Thread: {}", thread.title)
                } else {
                    "Thread deleted".to_string()
                }
            }
            "user" => {
                if let Some(name) = user_names::Entity::find()
                    .filter(user_names::Column::UserId.eq(report.content_id))
                    .one(db)
                    .await
                    .map_err(error::ErrorInternalServerError)?
                {
                    format!("User: {}", name.name)
                } else {
                    "User deleted".to_string()
                }
            }
            _ => format!("{} #{}", report.content_type, report.content_id),
        };

        report_views.push(ReportView {
            id: report.id,
            reporter_name,
            content_type: report.content_type,
            content_id: report.content_id,
            reason: report.reason,
            reason_label,
            details: report.details,
            status: report.status,
            created_at: report.created_at,
            content_preview,
        });
    }

    Ok(ReportsListTemplate {
        client,
        reports: report_views,
        filter_status: status_filter,
    }
    .to_response())
}

#[derive(Deserialize)]
struct ReportsQuery {
    status: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/report_detail.html")]
struct ReportDetailTemplate {
    client: ClientCtx,
    report: ReportDetailView,
}

struct ReportDetailView {
    id: i32,
    reporter_name: String,
    reporter_id: i32,
    content_type: String,
    content_id: i32,
    content_url: String,
    content_preview: String,
    reason: String,
    reason_label: String,
    details: Option<String>,
    status: String,
    moderator_name: Option<String>,
    moderator_notes: Option<String>,
    resolved_at: Option<chrono::NaiveDateTime>,
    created_at: chrono::NaiveDateTime,
}

/// View single report details
#[get("/admin/reports/{id}")]
async fn view_report(client: ClientCtx, path: web::Path<i32>) -> Result<impl Responder, Error> {
    client.require_login()?;
    client.require_permission("moderate.reports.view")?;

    let db = get_db_pool();
    let report_id = path.into_inner();

    let report = reports::Entity::find_by_id(report_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Report not found"))?;

    // Get reporter name
    let reporter_name = user_names::Entity::find()
        .filter(user_names::Column::UserId.eq(report.reporter_id))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .map(|u| u.name)
        .unwrap_or_else(|| "Unknown".to_string());

    // Get moderator name if assigned
    let moderator_name = if let Some(mod_id) = report.moderator_id {
        user_names::Entity::find()
            .filter(user_names::Column::UserId.eq(mod_id))
            .one(db)
            .await
            .map_err(error::ErrorInternalServerError)?
            .map(|u| u.name)
    } else {
        None
    };

    // Get reason label
    let reason_label = report_reasons::Entity::find()
        .filter(report_reasons::Column::Name.eq(report.reason.clone()))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .map(|r| r.label)
        .unwrap_or_else(|| report.reason.clone());

    // Get content URL and preview
    let (content_url, content_preview) = match report.content_type.as_str() {
        "post" => {
            if let Some(post) = posts::Entity::find_by_id(report.content_id)
                .one(db)
                .await
                .map_err(error::ErrorInternalServerError)?
            {
                (
                    format!("/threads/{}#post-{}", post.thread_id, post.id),
                    format!("Post #{} in thread #{}", post.id, post.thread_id),
                )
            } else {
                ("#".to_string(), "Post deleted".to_string())
            }
        }
        "thread" => {
            if let Some(thread) = threads::Entity::find_by_id(report.content_id)
                .one(db)
                .await
                .map_err(error::ErrorInternalServerError)?
            {
                (
                    format!("/threads/{}/", thread.id),
                    format!("Thread: {}", thread.title),
                )
            } else {
                ("#".to_string(), "Thread deleted".to_string())
            }
        }
        "user" => (
            format!("/members/{}/", report.content_id),
            format!("User #{}", report.content_id),
        ),
        _ => (
            "#".to_string(),
            format!("{} #{}", report.content_type, report.content_id),
        ),
    };

    Ok(ReportDetailTemplate {
        client,
        report: ReportDetailView {
            id: report.id,
            reporter_name,
            reporter_id: report.reporter_id,
            content_type: report.content_type,
            content_id: report.content_id,
            content_url,
            content_preview,
            reason: report.reason,
            reason_label,
            details: report.details,
            status: report.status,
            moderator_name,
            moderator_notes: report.moderator_notes,
            resolved_at: report.resolved_at,
            created_at: report.created_at,
        },
    }
    .to_response())
}

#[derive(Deserialize)]
struct UpdateReportForm {
    csrf_token: String,
    status: String,
    moderator_notes: Option<String>,
}

/// Update report status (moderators only)
#[post("/admin/reports/{id}/update")]
async fn update_report_status(
    client: ClientCtx,
    session: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<UpdateReportForm>,
) -> Result<impl Responder, Error> {
    let moderator_id = client.require_login()?;
    client.require_permission("moderate.reports.manage")?;

    // Validate CSRF
    crate::middleware::csrf::validate_csrf_token(&session, &form.csrf_token)?;

    let db = get_db_pool();
    let report_id = path.into_inner();

    // Validate status
    let valid_statuses = ["open", "reviewed", "resolved", "dismissed"];
    if !valid_statuses.contains(&form.status.as_str()) {
        return Err(error::ErrorBadRequest("Invalid status"));
    }

    let report = reports::Entity::find_by_id(report_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Report not found"))?;

    let now = Utc::now().naive_utc();
    let resolved_at = if form.status == "resolved" || form.status == "dismissed" {
        Some(now)
    } else {
        None
    };

    let mut active_report: reports::ActiveModel = report.into();
    active_report.status = Set(form.status.clone());
    active_report.moderator_id = Set(Some(moderator_id));
    active_report.moderator_notes = Set(form.moderator_notes.clone());
    active_report.resolved_at = Set(resolved_at);
    active_report.updated_at = Set(now);

    active_report
        .update(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", format!("/admin/reports/{}", report_id)))
        .finish())
}
