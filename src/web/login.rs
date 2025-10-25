use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{user_2fa, user_names, users};
use crate::session;
use crate::session::{authenticate_by_cookie, get_argon2, get_sess};
use actix_web::{error, get, post, web, Error, Responder};
use argon2::password_hash::{PasswordHash, PasswordVerifier};
use askama::Template;
use askama_actix::TemplateToResponse;
use google_authenticator::GoogleAuthenticator;
use sea_orm::{entity::*, query::*, DbErr, QueryFilter};
use serde::Deserialize;
use validator::Validate;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(post_login)
        .service(post_login_2fa)
        .service(view_login);
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate<'a> {
    pub client: ClientCtx,
    pub logged_in: bool,
    pub user_id: Option<i32>,
    pub username: Option<&'a str>,
    pub token: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "login_2fa.html")]
pub struct Login2FATemplate<'a> {
    pub client: ClientCtx,
    pub error: Option<&'a str>,
}

#[derive(Deserialize, Validate)]
pub struct FormData {
    #[validate(length(min = 1, max = 255))]
    username: String,

    #[validate(length(min = 1, max = 1000))]
    password: String,

    #[validate(custom = "validate_totp")]
    totp: Option<String>,

    csrf_token: String,
}

/// Validate TOTP code format (must be exactly 6 digits)
fn validate_totp(code: &str) -> Result<(), validator::ValidationError> {
    if code.len() != 6 {
        return Err(validator::ValidationError::new("totp_length"));
    }
    if !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(validator::ValidationError::new("totp_format"));
    }
    Ok(())
}

#[derive(Deserialize, Validate)]
pub struct TotpFormData {
    #[validate(custom = "validate_totp")]
    totp: String,

    csrf_token: String,
}

#[derive(Debug)]
pub enum LoginResultStatus {
    Success,
    BadName,
    BadPassword,
    Bad2FA,
    Missing2FA,
    AccountLocked,
}

pub struct LoginResult {
    pub result: LoginResultStatus,
    pub user_id: Option<i32>,
}

impl LoginResult {
    fn success(user_id: i32) -> Self {
        Self {
            result: LoginResultStatus::Success,
            user_id: Some(user_id),
        }
    }
    fn fail(result: LoginResultStatus) -> Self {
        Self {
            result,
            user_id: None,
        }
    }
}

pub async fn login<S: AsRef<str>>(
    name: &str,
    pass: &str,
    totp: &Option<S>,
) -> Result<LoginResult, DbErr> {
    use chrono::Utc;
    use sea_orm::ActiveValue::Set;

    const MAX_FAILED_ATTEMPTS: i32 = 5;
    const LOCKOUT_DURATION_MINUTES: i64 = 15;

    // Trim whitespace from username for consistent lookups
    let name = name.trim();

    let db = get_db_pool();
    let user_id = user_names::Entity::find()
        .filter(user_names::Column::Name.eq(name))
        .one(db)
        .await?;

    let user_id = match user_id {
        Some(user) => user.user_id,
        None => return Ok(LoginResult::fail(LoginResultStatus::BadName)),
    };

    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await?;

    let user = match user {
        Some(user) => user,
        None => return Ok(LoginResult::fail(LoginResultStatus::BadName)),
    };

    // Check if account is locked
    if let Some(locked_until) = user.locked_until {
        if locked_until > Utc::now().naive_utc() {
            return Ok(LoginResult::fail(LoginResultStatus::AccountLocked));
        } else {
            // Lock has expired, reset failed attempts
            let mut active_user: users::ActiveModel = user.clone().into();
            active_user.failed_login_attempts = Set(0);
            active_user.locked_until = Set(None);
            active_user.update(db).await?;
        }
    }

    let parsed_hash = PasswordHash::new(&user.password).unwrap();
    if get_argon2()
        .verify_password(pass.as_bytes(), &parsed_hash)
        .is_err()
    {
        // Increment failed login attempts
        let mut active_user: users::ActiveModel = user.clone().into();
        let new_attempts = user.failed_login_attempts + 1;
        active_user.failed_login_attempts = Set(new_attempts);

        // Lock account if max attempts reached
        if new_attempts >= MAX_FAILED_ATTEMPTS {
            let lock_until = Utc::now().naive_utc() + chrono::Duration::minutes(LOCKOUT_DURATION_MINUTES);
            active_user.locked_until = Set(Some(lock_until));
            log::warn!(
                "Account locked due to {} failed login attempts: user_id={}",
                new_attempts, user.id
            );
        }

        active_user.update(db).await?;
        return Ok(LoginResult::fail(LoginResultStatus::BadPassword));
    }

    let totp_exists = user_2fa::Entity::find()
        .limit(1)
        .filter(user_2fa::Column::UserId.eq(user_id))
        .count(db)
        .await?;

    if totp_exists > 0 {
        if let Some(totp) = totp {
            let secret = user_2fa::Entity::find_by_id(user_id).one(db).await?;
            if let Some(secret) = secret {
                let auth = GoogleAuthenticator::new();
                // Trim secret (DB uses CHAR which pads with spaces)
                let verify = auth.verify_code(secret.secret.trim(), totp.as_ref(), 60, 0);
                if verify {
                    // Reset failed login attempts on successful login
                    if user.failed_login_attempts > 0 || user.locked_until.is_some() {
                        let mut active_user: users::ActiveModel = user.clone().into();
                        active_user.failed_login_attempts = Set(0);
                        active_user.locked_until = Set(None);
                        active_user.update(db).await?;
                    }
                    return Ok(LoginResult::success(user.id));
                }
                return Ok(LoginResult::fail(LoginResultStatus::Bad2FA));
            }
        }
        // User has 2FA enabled but didn't provide code
        // Include user_id for pending auth state
        return Ok(LoginResult {
            result: LoginResultStatus::Missing2FA,
            user_id: Some(user.id),
        });
    }

    // Reset failed login attempts on successful login
    if user.failed_login_attempts > 0 || user.locked_until.is_some() {
        let mut active_user: users::ActiveModel = user.into();
        active_user.failed_login_attempts = Set(0);
        active_user.locked_until = Set(None);
        active_user.update(db).await?;
    }

    Ok(LoginResult::success(user_id))
}

#[post("/login")]
pub async fn post_login(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<FormData>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Validate input
    form.validate().map_err(|e| {
        log::debug!("Login form validation failed: {}", e);
        error::ErrorBadRequest("Invalid login credentials format")
    })?;

    // Sanitize username (trim whitespace, prevent injection)
    let username = form.username.trim();

    let user_id = login(username, &form.password, &form.totp)
        .await
        .map_err(|e| {
            log::error!("error {:?}", e);
            error::ErrorInternalServerError("DB error")
        })?;

    let user_id = match user_id.result {
        LoginResultStatus::Success => user_id.user_id.unwrap(),
        LoginResultStatus::Missing2FA => {
            // User has 2FA enabled but didn't provide TOTP code
            // Store pending auth state in session
            cookies
                .insert("pending_2fa_user_id", user_id.user_id.unwrap())
                .map_err(|_| error::ErrorInternalServerError("Session error"))?;

            // Show 2FA input form
            return Ok(Login2FATemplate {
                client,
                error: None,
            }
            .to_response());
        }
        LoginResultStatus::AccountLocked => {
            log::warn!("Login attempt on locked account: {}", form.username);
            return Err(error::ErrorForbidden("Account locked due to too many failed login attempts. Please try again in 15 minutes."));
        }
        LoginResultStatus::Bad2FA => {
            log::debug!("login failure: invalid 2FA code for {}", form.username);
            return Err(error::ErrorUnauthorized("Invalid two-factor authentication code."));
        }
        LoginResultStatus::BadName | LoginResultStatus::BadPassword => {
            log::debug!("login failure: {:?} for {}", user_id.result, form.username);
            // Use generic message to avoid username enumeration
            return Err(error::ErrorUnauthorized("Invalid username or password."));
        }
    };

    let uuid = session::new_session(get_sess(), user_id)
        .await
        .map_err(|e| {
            log::error!("error {:?}", e);
            error::ErrorInternalServerError("DB error")
        })?
        .to_string();

    cookies
        .insert("logged_in", true)
        .map_err(|_| error::ErrorInternalServerError("middleware error"))?;

    cookies
        .insert("token", uuid.to_owned())
        .map_err(|_| error::ErrorInternalServerError("middleware error"))?;

    Ok(LoginTemplate {
        client: ClientCtx::from_session(&cookies, client.get_permissions().clone()).await,
        user_id: Some(user_id),
        logged_in: true,
        username: Some(&form.username),
        token: Some(&uuid),
    }
    .to_response())
}

#[post("/login/2fa")]
pub async fn post_login_2fa(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<TotpFormData>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Validate TOTP format
    form.validate().map_err(|e| {
        log::debug!("2FA form validation failed: {}", e);
        error::ErrorBadRequest("Invalid authentication code format")
    })?;

    // Get pending auth state from session
    let user_id: i32 = match cookies.get("pending_2fa_user_id") {
        Ok(Some(id)) => id,
        _ => {
            log::warn!("2FA attempt without pending auth state");
            return Err(error::ErrorBadRequest("No pending authentication. Please login again."));
        }
    };

    // Get user's 2FA secret from database
    let db = get_db_pool();
    let secret = user_2fa::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            log::error!("Database error fetching 2FA secret: {:?}", e);
            error::ErrorInternalServerError("Authentication error")
        })?;

    let secret = match secret {
        Some(s) => s,
        None => {
            log::error!("User {} has no 2FA secret but reached 2FA flow", user_id);
            cookies.remove("pending_2fa_user_id");
            return Err(error::ErrorInternalServerError("Authentication configuration error"));
        }
    };

    // Verify TOTP code
    let auth = GoogleAuthenticator::new();
    // Trim secret (DB uses CHAR which pads with spaces)
    if !auth.verify_code(secret.secret.trim(), &form.totp, 60, 0) {
        log::debug!("Invalid 2FA code for user {}", user_id);
        return Ok(Login2FATemplate {
            client,
            error: Some("Invalid authentication code. Please try again."),
        }
        .to_response());
    }

    // TOTP verification successful - clear pending state
    cookies.remove("pending_2fa_user_id");

    // Reset any failed login attempts (user successfully authenticated)
    use sea_orm::ActiveValue::Set;
    use users::Entity as Users;
    if let Ok(Some(user)) = Users::find_by_id(user_id).one(db).await {
        if user.failed_login_attempts > 0 || user.locked_until.is_some() {
            let mut active_user: users::ActiveModel = user.into();
            active_user.failed_login_attempts = Set(0);
            active_user.locked_until = Set(None);
            let _ = active_user.update(db).await;
        }
    }

    // Create session
    let uuid = session::new_session(get_sess(), user_id)
        .await
        .map_err(|e| {
            log::error!("Error creating session: {:?}", e);
            error::ErrorInternalServerError("Session creation error")
        })?
        .to_string();

    cookies
        .insert("logged_in", true)
        .map_err(|_| error::ErrorInternalServerError("Session error"))?;

    cookies
        .insert("token", uuid.to_owned())
        .map_err(|_| error::ErrorInternalServerError("Session error"))?;

    // Redirect to home or show success
    Ok(LoginTemplate {
        client: ClientCtx::from_session(&cookies, client.get_permissions().clone()).await,
        user_id: Some(user_id),
        logged_in: true,
        username: None,
        token: Some(&uuid),
    }
    .to_response())
}

#[get("/login")]
pub async fn view_login(
    client: ClientCtx,
    cookies: actix_session::Session,
) -> Result<impl Responder, Error> {
    let mut tmpl = LoginTemplate {
        client,
        user_id: None,
        logged_in: false,
        username: None,
        token: None,
    };

    let uuid_str: String;
    if let Some((uuid, session)) = authenticate_by_cookie(&cookies).await {
        tmpl.user_id = Some(session.user_id);
        tmpl.logged_in = true;
        uuid_str = uuid.to_string();
        tmpl.token = Some(&uuid_str);
    }

    Ok(tmpl.to_response())
}
