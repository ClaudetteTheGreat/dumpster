/// Email verification functionality
///
/// This module handles email verification for new user registrations.
use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{email_verification_tokens, users};
use actix_web::{error, get, post, web, Error, HttpRequest, Responder};
use askama_actix::{Template, TemplateToResponse};
use chrono::{Duration, Utc};
use rand::Rng;
use sea_orm::{entity::*, query::*, ActiveValue::Set, DatabaseConnection};
use serde::Deserialize;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(verify_email)
        .service(resend_verification_form)
        .service(resend_verification);
}

/// Template for email verification page
#[derive(Template)]
#[template(path = "email_verification.html")]
#[allow(dead_code)]
struct EmailVerificationTemplate {
    client: ClientCtx,
    success: bool,
    error: Option<String>,
    message: Option<String>,
}

/// Template for resend verification email page
#[derive(Template)]
#[template(path = "email_verification_resend.html")]
struct ResendVerificationTemplate {
    client: ClientCtx,
    error: Option<String>,
    success: Option<String>,
}

/// Form data for resending verification email
#[derive(Deserialize)]
struct ResendVerificationForm {
    email: String,
    csrf_token: String,
}

/// Generate a secure random token
fn generate_verification_token() -> String {
    use rand::distributions::Alphanumeric;

    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

/// GET /verify-email/{token} - Verify email address
#[get("/verify-email/{token}")]
pub async fn verify_email(
    client: ClientCtx,
    token: web::Path<String>,
) -> Result<impl Responder, Error> {
    let token_str = token.into_inner();
    let db = get_db_pool();

    // Find and validate token
    let verification_token = match validate_verification_token(db, &token_str).await? {
        Some(token) => token,
        None => {
            return Ok(EmailVerificationTemplate {
                client,
                success: false,
                error: Some("This verification link is invalid or has expired.".to_string()),
                message: None,
            }
            .to_response());
        }
    };

    // Update user email and mark as verified
    let user_id = verification_token.user_id;
    let email = verification_token.email.clone();

    let mut user: users::ActiveModel = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find user: {}", e);
            error::ErrorInternalServerError("Failed to verify email")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?
        .into();

    user.email = Set(Some(email));
    user.email_verified = Set(true);
    user.update(db).await.map_err(|e| {
        log::error!("Failed to update user: {}", e);
        error::ErrorInternalServerError("Failed to verify email")
    })?;

    // Mark token as used
    let mut token_model: email_verification_tokens::ActiveModel = verification_token.into();
    token_model.used = Set(true);
    if let Err(e) = token_model.update(db).await {
        log::error!("Failed to mark token as used: {}", e);
        // Don't fail - email is already verified
    }

    log::info!("Email verified for user_id: {}", user_id);

    Ok(EmailVerificationTemplate {
        client,
        success: true,
        error: None,
        message: Some("Your email has been verified! You can now log in.".to_string()),
    }
    .to_response())
}

/// GET /verify-email/resend - Show resend verification email form
#[get("/verify-email/resend")]
pub async fn resend_verification_form(client: ClientCtx) -> impl Responder {
    ResendVerificationTemplate {
        client,
        error: None,
        success: None,
    }
    .to_response()
}

/// POST /verify-email/resend - Resend verification email
#[post("/verify-email/resend")]
pub async fn resend_verification(
    req: HttpRequest,
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<ResendVerificationForm>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Get client IP for rate limiting
    let ip = crate::ip::extract_client_ip(&req)
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Rate limiting - prevent abuse
    if let Err(e) = crate::rate_limit::check_email_verification_rate_limit(&ip) {
        log::warn!("Email verification rate limit exceeded for IP: {}", ip);
        return Err(error::ErrorTooManyRequests(format!(
            "Too many verification requests. Please try again in {} seconds.",
            e.retry_after_seconds
        )));
    }

    let email = form.email.trim().to_lowercase();
    let db = get_db_pool();

    // Find user by email
    match users::Entity::find()
        .filter(users::Column::Email.eq(email.clone()))
        .one(db)
        .await
    {
        Ok(Some(user)) => {
            // Check if already verified
            if user.email_verified {
                return Ok(ResendVerificationTemplate {
                    client,
                    error: None,
                    success: Some("This email is already verified.".to_string()),
                }
                .to_response());
            }

            // Generate new verification token
            let token = generate_verification_token();
            let expires_at = Utc::now().naive_utc() + Duration::hours(24);

            // Get username for email
            let username = match get_username_by_user_id(db, user.id).await? {
                Some(name) => name,
                None => {
                    log::warn!("User {} has no username entry", user.id);
                    return Ok(ResendVerificationTemplate {
                        client,
                        error: Some("Unable to send verification email.".to_string()),
                        success: None,
                    }
                    .to_response());
                }
            };

            // Save token to database
            let verification_token = email_verification_tokens::ActiveModel {
                token: Set(token.clone()),
                user_id: Set(user.id),
                email: Set(email.clone()),
                created_at: Set(Utc::now().naive_utc()),
                expires_at: Set(expires_at),
                used: Set(false),
            };

            verification_token.insert(db).await.map_err(|e| {
                log::error!("Failed to save verification token: {}", e);
                error::ErrorInternalServerError("Failed to process request")
            })?;

            // Send verification email
            let base_url =
                std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

            if let Err(e) = crate::email::templates::send_verification_email(
                &email, &username, &token, &base_url,
            )
            .await
            {
                log::error!("Failed to send verification email: {}", e);
                // Don't fail the request - token is saved, user can try again
            }

            log::info!("Verification email resent for user: {}", username);
        }
        Ok(None) => {
            // Don't reveal if email exists for security
            log::debug!(
                "Verification resend requested for non-existent email: {}",
                email
            );
        }
        Err(e) => {
            log::error!("Database error during verification resend: {}", e);
            return Err(error::ErrorInternalServerError("Failed to process request"));
        }
    }

    // Always show success message (don't reveal if email exists)
    Ok(ResendVerificationTemplate {
        client,
        error: None,
        success: Some(
            "If an account exists with that email, you will receive a verification link shortly."
                .to_string(),
        ),
    }
    .to_response())
}

/// Helper: Get username by user_id
async fn get_username_by_user_id(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<Option<String>, Error> {
    use crate::orm::user_names;

    let result = user_names::Entity::find()
        .filter(user_names::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find username: {}", e);
            error::ErrorInternalServerError("Database error")
        })?;

    Ok(result.map(|user_name| user_name.name))
}

/// Helper: Validate verification token (not expired, not used)
async fn validate_verification_token(
    db: &DatabaseConnection,
    token: &str,
) -> Result<Option<email_verification_tokens::Model>, Error> {
    let verification_token = email_verification_tokens::Entity::find_by_id(token.to_string())
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Failed to find verification token: {}", e);
            error::ErrorInternalServerError("Failed to validate token")
        })?;

    match verification_token {
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

/// Create a verification token for a user (called from registration)
pub async fn create_verification_token(
    user_id: i32,
    email: &str,
) -> Result<String, sea_orm::DbErr> {
    let db = get_db_pool();
    let token = generate_verification_token();
    let expires_at = Utc::now().naive_utc() + Duration::hours(24);

    let verification_token = email_verification_tokens::ActiveModel {
        token: Set(token.clone()),
        user_id: Set(user_id),
        email: Set(email.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        expires_at: Set(expires_at),
        used: Set(false),
    };

    verification_token.insert(db).await?;

    Ok(token)
}
