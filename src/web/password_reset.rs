/// Password reset functionality
///
/// This module handles password reset requests and confirmations.

use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{password_reset_tokens, users};
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use chrono::{Duration, Utc};
use rand::Rng;
use sea_orm::{entity::*, query::*, ActiveValue::Set, DatabaseConnection};
use serde::Deserialize;
use validator::Validate;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(request_reset_form)
        .service(request_reset)
        .service(confirm_reset_form)
        .service(confirm_reset);
}

/// Template for password reset request form
#[derive(Template)]
#[template(path = "password_reset_request.html")]
struct PasswordResetRequestTemplate {
    client: ClientCtx,
    error: Option<String>,
    success: Option<String>,
}

/// Template for password reset confirmation form
#[derive(Template)]
#[template(path = "password_reset_confirm.html")]
struct PasswordResetConfirmTemplate {
    client: ClientCtx,
    token: String,
    error: Option<String>,
}

/// Form data for password reset request
#[derive(Deserialize, Validate)]
struct PasswordResetRequestForm {
    #[validate(email)]
    email: String,
    csrf_token: String,
}

/// Form data for password reset confirmation
#[derive(Deserialize, Validate)]
struct PasswordResetConfirmForm {
    #[validate(length(min = 8, max = 1000))]
    password: String,
    #[validate(length(min = 8, max = 1000))]
    password_confirm: String,
    csrf_token: String,
}

/// Generate a secure random token
fn generate_reset_token() -> String {
    use rand::distributions::Alphanumeric;

    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

/// GET /password-reset - Show password reset request form
#[get("/password-reset")]
pub async fn request_reset_form(client: ClientCtx) -> impl Responder {
    PasswordResetRequestTemplate {
        client,
        error: None,
        success: None,
    }
    .to_response()
}

/// POST /password-reset - Process password reset request
#[post("/password-reset")]
pub async fn request_reset(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<PasswordResetRequestForm>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Validate form
    form.validate().map_err(|e| {
        log::debug!("Password reset form validation failed: {}", e);
        error::ErrorBadRequest("Invalid email address")
    })?;

    let email = form.email.trim().to_lowercase();

    // Find user by email (don't reveal if user exists for security)
    let db = get_db_pool();

    match find_user_by_email(db, &email).await {
        Ok(Some((user, username))) => {
            // Generate reset token
            let token = generate_reset_token();
            let expires_at = Utc::now().naive_utc() + Duration::hours(1);

            // Save token to database
            let reset_token = password_reset_tokens::ActiveModel {
                token: Set(token.clone()),
                user_id: Set(user.id),
                created_at: Set(Utc::now().naive_utc()),
                expires_at: Set(expires_at),
                used: Set(false),
            };

            reset_token.insert(db).await.map_err(|e| {
                log::error!("Failed to save password reset token: {}", e);
                error::ErrorInternalServerError("Failed to process request")
            })?;

            // Send reset email
            let base_url = std::env::var("BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string());

            if let Err(e) = crate::email::templates::send_password_reset_email(
                &email,
                &username,
                &token,
                &base_url,
            )
            .await
            {
                log::error!("Failed to send password reset email: {}", e);
                // Don't fail the request - token is saved, user can try again
            }

            log::info!("Password reset requested for user: {}", username);
        }
        Ok(None) => {
            // User not found - don't reveal this for security
            log::debug!("Password reset requested for non-existent email: {}", email);
        }
        Err(e) => {
            log::error!("Database error during password reset: {}", e);
            return Err(error::ErrorInternalServerError("Failed to process request"));
        }
    }

    // Always show success message (don't reveal if email exists)
    Ok(PasswordResetRequestTemplate {
        client,
        error: None,
        success: Some(
            "If an account exists with that email, you will receive a password reset link shortly."
                .to_string(),
        ),
    }
    .to_response())
}

/// GET /password-reset/{token} - Show password reset confirmation form
#[get("/password-reset/{token}")]
pub async fn confirm_reset_form(
    client: ClientCtx,
    token: web::Path<String>,
) -> Result<impl Responder, Error> {
    let token_str = token.into_inner();

    // Validate token exists and is not expired
    let db = get_db_pool();

    match validate_reset_token(db, &token_str).await? {
        Some(_) => Ok(PasswordResetConfirmTemplate {
            client,
            token: token_str,
            error: None,
        }
        .to_response()),
        None => Ok(PasswordResetConfirmTemplate {
            client,
            token: token_str,
            error: Some("This password reset link is invalid or has expired.".to_string()),
        }
        .to_response()),
    }
}

/// POST /password-reset/{token} - Confirm password reset
#[post("/password-reset/{token}")]
pub async fn confirm_reset(
    client: ClientCtx,
    cookies: actix_session::Session,
    token: web::Path<String>,
    form: web::Form<PasswordResetConfirmForm>,
) -> Result<impl Responder, Error> {
    let token_str = token.into_inner();

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Validate form
    form.validate().map_err(|e| {
        log::debug!("Password reset confirm validation failed: {}", e);
        error::ErrorBadRequest("Invalid password")
    })?;

    // Check password confirmation
    if form.password != form.password_confirm {
        return Ok(PasswordResetConfirmTemplate {
            client,
            token: token_str,
            error: Some("Passwords do not match.".to_string()),
        }
        .to_response());
    }

    let db = get_db_pool();

    // Validate reset token
    let reset_token = match validate_reset_token(db, &token_str).await? {
        Some(token) => token,
        None => {
            return Ok(PasswordResetConfirmTemplate {
                client,
                token: token_str,
                error: Some("This password reset link is invalid or has expired.".to_string()),
            }
            .to_response());
        }
    };

    // Hash new password
    let password_hash = crate::session::hash_password(&form.password).map_err(|e| {
        log::error!("Failed to hash password: {}", e);
        error::ErrorInternalServerError("Failed to reset password")
    })?;

    // Update user password
    let user_id = reset_token.user_id;
    let mut user: users::ActiveModel = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find user: {}", e);
            error::ErrorInternalServerError("Failed to reset password")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?
        .into();

    user.password = Set(password_hash);
    user.update(db).await.map_err(|e| {
        log::error!("Failed to update user password: {}", e);
        error::ErrorInternalServerError("Failed to reset password")
    })?;

    // Mark token as used
    let mut token_model: password_reset_tokens::ActiveModel = reset_token.into();
    token_model.used = Set(true);
    if let Err(e) = token_model.update(db).await {
        log::error!("Failed to mark token as used: {}", e);
        // Don't fail - password is already updated
    }

    // Invalidate all user sessions for security
    // This forces re-authentication on all devices after password reset
    let sessions = crate::session::get_sess();
    if let Err(e) = crate::session::invalidate_user_sessions(sessions, user_id).await {
        log::error!("Failed to invalidate user sessions after password reset: {}", e);
        // Don't fail - password is already updated, this is just additional security
    }

    log::info!("Password reset successful for user_id: {}", user_id);

    // Redirect to login page with success message
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/login?reset=success"))
        .finish())
}

/// Helper: Find user by email, returns (user, username)
async fn find_user_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<(users::Model, String)>, sea_orm::DbErr> {
    use crate::orm::user_names;

    let result = users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .find_also_related(user_names::Entity)
        .one(db)
        .await?;

    match result {
        Some((user, Some(user_name))) => Ok(Some((user, user_name.name))),
        Some((user, None)) => {
            // User exists but has no username entry (shouldn't happen but handle gracefully)
            log::warn!("User {} has no username entry", user.id);
            Ok(None)
        }
        None => Ok(None),
    }
}

/// Helper: Validate reset token (not expired, not used)
async fn validate_reset_token(
    db: &DatabaseConnection,
    token: &str,
) -> Result<Option<password_reset_tokens::Model>, Error> {
    let reset_token = password_reset_tokens::Entity::find_by_id(token.to_string())
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find reset token: {}", e);
            error::ErrorInternalServerError("Failed to validate token")
        })?;

    match reset_token {
        Some(token) => {
            // Check if token is expired
            if token.expires_at < Utc::now().naive_utc() {
                return Ok(None);
            }

            // Check if token is already used
            if token.used {
                return Ok(None);
            }

            Ok(Some(token))
        }
        None => Ok(None),
    }
}
