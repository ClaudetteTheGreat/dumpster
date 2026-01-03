use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::users;
use crate::session::get_argon2;
use crate::template::CreateUserTemplate;
use actix_web::{error, get, post, web, Error, HttpRequest, HttpResponse, Responder};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    PasswordHasher,
};
use askama_actix::TemplateToResponse;
use chrono::Utc;
use sea_orm::{
    entity::*, ConnectionTrait, DbErr, InsertResult, QueryFilter, Statement, TransactionTrait,
};
use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct FormData {
    #[validate(length(min = 1, max = 255))]
    username: String,
    #[validate(length(min = 8, max = 1000))]
    password: String,
    #[validate(email)]
    email: String,
    /// CAPTCHA response token (optional if CAPTCHA is disabled)
    #[serde(rename = "h-captcha-response")]
    hcaptcha_response: Option<String>,
    #[serde(rename = "cf-turnstile-response")]
    turnstile_response: Option<String>,
}

/// Error type for user creation
#[derive(Debug)]
pub enum CreateUserError {
    UsernameExists,
    EmailExists,
    Database(DbErr),
}

impl From<DbErr> for CreateUserError {
    fn from(err: DbErr) -> Self {
        CreateUserError::Database(err)
    }
}

impl std::fmt::Display for CreateUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateUserError::UsernameExists => write!(f, "Username already exists"),
            CreateUserError::EmailExists => write!(f, "Email already exists"),
            CreateUserError::Database(e) => write!(f, "Database error: {}", e),
        }
    }
}

async fn insert_new_user(
    name: &str,
    pass: &str,
    email: &str,
) -> Result<InsertResult<users::ActiveModel>, CreateUserError> {
    use crate::orm::{user_name_history, user_names};
    use futures::join;

    let db = get_db_pool();

    // Check if username already exists (case-insensitive)
    let existing_user = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT user_id FROM user_names WHERE LOWER(name) = LOWER($1) LIMIT 1",
            vec![name.into()],
        ))
        .await?;

    if existing_user.is_some() {
        return Err(CreateUserError::UsernameExists);
    }

    // Check if email already exists
    let existing_email = users::Entity::find()
        .filter(users::Column::Email.eq(email.to_lowercase()))
        .one(db)
        .await?;

    if existing_email.is_some() {
        return Err(CreateUserError::EmailExists);
    }

    let txn = db.begin().await?;
    let now = Utc::now().naive_utc();

    // Insert user
    let user = users::ActiveModel {
        created_at: Set(now),
        password: Set(pass.to_owned()),
        password_cipher: Set(users::Cipher::Argon2id),
        email: Set(Some(email.to_owned())),
        email_verified: Set(false),
        ..Default::default() // all other attributes are `Unset`
    };
    let res = users::Entity::insert(user).exec(db).await?;

    let user_name_ins = user_names::ActiveModel {
        user_id: Set(res.last_insert_id),
        name: Set(name.to_owned()),
    };

    let user_name_history_ins = user_name_history::ActiveModel {
        user_id: Set(res.last_insert_id),
        created_at: Set(now),
        approved_at: Set(now),
        name: Set(name.to_owned()),
        is_public: Set(true),
        ..Default::default()
    };

    // exec secondary inserts
    let (un_result, unh_result) = join!(
        user_names::Entity::insert(user_name_ins).exec(db),
        user_name_history::Entity::insert(user_name_history_ins).exec(db)
    );

    un_result?;
    unh_result?;
    txn.commit().await?;

    Ok(res)
}

#[get("/create_user")]
pub async fn create_user_get(client: ClientCtx) -> impl Responder {
    CreateUserTemplate {
        client,
        logged_in: true,
        username: None,
        captcha_enabled: crate::captcha::is_enabled(),
        captcha_provider: crate::captcha::get_provider_name().map(String::from),
        captcha_site_key: crate::captcha::get_site_key().map(String::from),
    }
    .to_response()
}
#[post("/create_user")]
pub async fn create_user_post(
    req: HttpRequest,
    form: web::Form<FormData>,
) -> Result<HttpResponse, Error> {
    // Get client IP for rate limiting
    let ip = crate::ip::extract_client_ip(&req)
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Rate limiting - prevent registration spam
    if let Err(e) = crate::rate_limit::check_registration_rate_limit(&ip) {
        log::warn!("Rate limit exceeded for registration: ip={}", ip);
        return Err(error::ErrorTooManyRequests(format!(
            "Too many registration attempts. Please wait {} seconds.",
            e.retry_after_seconds
        )));
    }

    // Check IP ban before proceeding with registration
    if let Some(ban_info) = crate::web::login::check_ip_ban(&ip).await.map_err(|e| {
        log::error!("Failed to check IP ban: {}", e);
        error::ErrorInternalServerError("Database error")
    })? {
        log::warn!("Registration attempt from banned IP: {}", ip);
        let message = if ban_info.is_permanent {
            format!(
                "Registration denied. Your IP address has been banned. Reason: {}",
                ban_info.reason
            )
        } else if let Some(expires) = ban_info.expires_at {
            format!(
                "Registration denied. Your IP address is banned until {}. Reason: {}",
                expires.format("%Y-%m-%d %H:%M UTC"),
                ban_info.reason
            )
        } else {
            format!(
                "Registration denied. Your IP address has been banned. Reason: {}",
                ban_info.reason
            )
        };
        return Err(error::ErrorForbidden(message));
    }

    // Verify CAPTCHA if enabled
    if crate::captcha::is_enabled() {
        let captcha_response = form
            .hcaptcha_response
            .as_deref()
            .or(form.turnstile_response.as_deref())
            .unwrap_or("");

        if captcha_response.is_empty() {
            return Err(error::ErrorBadRequest("CAPTCHA verification required"));
        }

        crate::captcha::verify(captcha_response, Some(&ip))
            .await
            .map_err(|e| {
                log::warn!("CAPTCHA verification failed for registration: {}", e);
                error::ErrorBadRequest("CAPTCHA verification failed. Please try again.")
            })?;
    }

    // Validate form input
    form.validate().map_err(|e| {
        log::debug!("User registration validation failed: {}", e);
        error::ErrorBadRequest("Invalid registration data")
    })?;

    // Sanitize inputs
    let username = form.username.trim();
    let email = form.email.trim().to_lowercase();

    // Hash password
    let password_hash = get_argon2()
        .hash_password(form.password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|e| {
            log::error!("Failed to hash password: {}", e);
            error::ErrorInternalServerError("Failed to create user")
        })?
        .to_string();

    // Create user
    let result = match insert_new_user(username, &password_hash, &email).await {
        Ok(result) => result,
        Err(CreateUserError::UsernameExists) => {
            log::info!(
                "Registration failed - username already exists: {}",
                username
            );
            return Err(error::ErrorConflict(
                "Username already exists (usernames are case-insensitive)",
            ));
        }
        Err(CreateUserError::EmailExists) => {
            log::info!("Registration failed - email already exists: {}", email);
            return Err(error::ErrorConflict(
                "An account with this email address already exists",
            ));
        }
        Err(CreateUserError::Database(e)) => {
            log::error!("Failed to create user: {}", e);
            return Err(error::ErrorInternalServerError("Failed to create user"));
        }
    };

    let user_id = result.last_insert_id;

    // Create verification token
    let token = crate::web::email_verification::create_verification_token(user_id, &email)
        .await
        .map_err(|e| {
            log::error!("Failed to create verification token: {}", e);
            error::ErrorInternalServerError("Failed to create user")
        })?;

    // Send verification email
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    if let Err(e) =
        crate::email::templates::send_verification_email(&email, username, &token, &base_url).await
    {
        log::error!("Failed to send verification email: {}", e);
        // Don't fail registration - token is saved, user can request resend
    }

    log::info!("New user registered: {} (user_id: {})", username, user_id);

    // Return success - could redirect to a "check your email" page
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(format!(
            r#"
            <html>
                <head><title>Registration Successful</title></head>
                <body>
                    <h1>Registration Successful!</h1>
                    <p>A verification email has been sent to <strong>{}</strong>.</p>
                    <p>Please check your email and click the verification link to activate your account.</p>
                    <p><a href="/login">Go to Login</a></p>
                </body>
            </html>
            "#,
            email
        )))
}
