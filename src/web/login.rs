use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{user_2fa, user_bans, user_names, users};
use crate::session;
use crate::session::{authenticate_by_cookie, get_argon2, get_sess};
use actix_web::{error, get, post, web, Error, Responder};
use argon2::password_hash::{PasswordHash, PasswordVerifier};
use askama::Template;
use askama_actix::TemplateToResponse;
use google_authenticator::GoogleAuthenticator;
use sea_orm::{entity::*, query::*, ConnectionTrait, DbErr, QueryFilter, Statement};
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
    pub success_message: Option<&'a str>,
    /// Whether CAPTCHA is required for this login attempt
    pub captcha_required: bool,
    /// CAPTCHA provider name if enabled ("hcaptcha" or "turnstile")
    pub captcha_provider: Option<String>,
    /// CAPTCHA site key if enabled
    pub captcha_site_key: Option<String>,
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

    #[serde(default)]
    remember_me: bool,

    /// CAPTCHA response token (required after multiple failed attempts)
    #[serde(rename = "h-captcha-response")]
    hcaptcha_response: Option<String>,
    #[serde(rename = "cf-turnstile-response")]
    turnstile_response: Option<String>,
}

/// Validate TOTP code format (must be exactly 6 digits, or empty)
fn validate_totp(code: &str) -> Result<(), validator::ValidationError> {
    // Allow empty string (no TOTP provided)
    if code.is_empty() {
        return Ok(());
    }
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
    EmailNotVerified,
    Banned(BanInfo),
    IpBanned(BanInfo),
}

/// Information about an active ban
#[derive(Debug, Clone)]
pub struct BanInfo {
    pub reason: String,
    pub expires_at: Option<chrono::NaiveDateTime>,
    pub is_permanent: bool,
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
    // Case-insensitive username lookup using LOWER()
    let user_id: Option<user_names::Model> = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT user_id, name FROM user_names WHERE LOWER(name) = LOWER($1) LIMIT 1",
            vec![name.into()],
        ))
        .await?
        .map(|row| user_names::Model {
            user_id: row.try_get("", "user_id").unwrap(),
            name: row.try_get("", "name").unwrap(),
        });

    let user_id = match user_id {
        Some(user) => user.user_id,
        None => return Ok(LoginResult::fail(LoginResultStatus::BadName)),
    };

    let user = users::Entity::find_by_id(user_id).one(db).await?;

    let user = match user {
        Some(user) => user,
        None => return Ok(LoginResult::fail(LoginResultStatus::BadName)),
    };

    // Check if user is banned
    let active_ban = user_bans::Entity::find()
        .filter(user_bans::Column::UserId.eq(user_id))
        .filter(
            // Permanent ban OR not yet expired
            user_bans::Column::IsPermanent
                .eq(true)
                .or(user_bans::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .order_by_desc(user_bans::Column::CreatedAt)
        .one(db)
        .await?;

    if let Some(ban) = active_ban {
        return Ok(LoginResult::fail(LoginResultStatus::Banned(BanInfo {
            reason: ban.reason,
            expires_at: ban.expires_at,
            is_permanent: ban.is_permanent,
        })));
    }

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
            let lock_until =
                Utc::now().naive_utc() + chrono::Duration::minutes(LOCKOUT_DURATION_MINUTES);
            active_user.locked_until = Set(Some(lock_until));
            log::warn!(
                "Account locked due to {} failed login attempts: user_id={}",
                new_attempts,
                user.id
            );
        }

        active_user.update(db).await?;
        return Ok(LoginResult::fail(LoginResultStatus::BadPassword));
    }

    // Check if email is verified (only if email exists)
    if user.email.is_some() && !user.email_verified {
        log::info!("Login blocked - email not verified: user_id={}", user.id);
        return Ok(LoginResult::fail(LoginResultStatus::EmailNotVerified));
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

/// Check if an IP address is banned
///
/// Returns Some(BanInfo) if the IP is banned, None otherwise.
/// Supports both exact IP matches and range bans (CIDR notation).
/// Uses raw SQL for proper PostgreSQL INET type handling and containment checking.
pub async fn check_ip_ban(ip_address: &str) -> Result<Option<BanInfo>, sea_orm::DbErr> {
    use chrono::Utc;
    use sea_orm::{ConnectionTrait, Statement};

    let db = get_db_pool();
    let now = Utc::now().naive_utc();
    let now_str = format!("{}", now.format("%Y-%m-%d %H:%M:%S"));

    // Use PostgreSQL's INET type containment operator (>>=) for range matching
    // ip_address >>= banned_ip checks if banned_ip contains ip_address
    // This handles both exact matches and CIDR range bans
    let sql = r#"
        SELECT reason, expires_at, is_permanent
        FROM ip_bans
        WHERE ip_address >>= $1::INET
        AND (is_permanent = true OR expires_at > $2::TIMESTAMP)
        ORDER BY created_at DESC
        LIMIT 1
    "#;

    let result = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            sql,
            vec![ip_address.into(), now_str.into()],
        ))
        .await?;

    if let Some(row) = result {
        let reason: String = row.try_get("", "reason")?;
        let expires_at: Option<chrono::NaiveDateTime> = row.try_get("", "expires_at")?;
        let is_permanent: bool = row.try_get("", "is_permanent")?;

        Ok(Some(BanInfo {
            reason,
            expires_at,
            is_permanent,
        }))
    } else {
        Ok(None)
    }
}

#[post("/login")]
pub async fn post_login(
    client: ClientCtx,
    cookies: actix_session::Session,
    req: actix_web::HttpRequest,
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

    // Get client IP
    let ip = crate::ip::extract_client_ip(&req)
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Rate limiting - prevent brute force attacks
    if let Err(e) = crate::rate_limit::check_login_rate_limit(&ip, username) {
        log::warn!(
            "Rate limit exceeded for login: ip={}, username={}",
            ip,
            username
        );
        return Err(error::ErrorTooManyRequests(format!(
            "Too many login attempts. Please try again in {} seconds.",
            e.retry_after_seconds
        )));
    }

    // Check IP ban before proceeding with login
    if let Some(ban_info) = check_ip_ban(&ip).await.map_err(|e| {
        log::error!("Failed to check IP ban: {}", e);
        error::ErrorInternalServerError("Database error")
    })? {
        log::warn!("Login attempt from banned IP: {}", ip);
        let message = if ban_info.is_permanent {
            format!(
                "Access denied. Your IP address has been banned. Reason: {}",
                ban_info.reason
            )
        } else if let Some(expires) = ban_info.expires_at {
            format!(
                "Access denied. Your IP address is banned until {}. Reason: {}",
                expires.format("%Y-%m-%d %H:%M UTC"),
                ban_info.reason
            )
        } else {
            format!(
                "Access denied. Your IP address has been banned. Reason: {}",
                ban_info.reason
            )
        };
        return Err(error::ErrorForbidden(message));
    }

    // Check if CAPTCHA is required based on failed attempts
    let failed_attempts = crate::rate_limit::get_failed_login_count(&ip);
    if crate::captcha::should_require_for_login(failed_attempts) {
        let captcha_response = form
            .hcaptcha_response
            .as_deref()
            .or(form.turnstile_response.as_deref())
            .unwrap_or("");

        if captcha_response.is_empty() {
            return Err(error::ErrorBadRequest(
                "CAPTCHA verification required due to multiple failed login attempts",
            ));
        }

        crate::captcha::verify(captcha_response, Some(&ip))
            .await
            .map_err(|e| {
                log::warn!("CAPTCHA verification failed for login: {}", e);
                error::ErrorBadRequest("CAPTCHA verification failed. Please try again.")
            })?;
    }

    let user_id = login(username, &form.password, &form.totp)
        .await
        .map_err(|e| {
            log::error!("error {:?}", e);
            error::ErrorInternalServerError("DB error")
        })?;

    let user_id = match user_id.result {
        LoginResultStatus::Success => {
            // Clear failed login attempts on success
            crate::rate_limit::clear_failed_logins(&ip);
            user_id.user_id.unwrap()
        }
        LoginResultStatus::Missing2FA => {
            // Password was correct, clear failed attempts
            // (2FA failures are tracked separately)
            crate::rate_limit::clear_failed_logins(&ip);

            // User has 2FA enabled but didn't provide TOTP code
            // Store pending auth state in session
            cookies
                .insert("pending_2fa_user_id", user_id.user_id.unwrap())
                .map_err(|_| error::ErrorInternalServerError("Session error"))?;

            // Store remember_me preference for after 2FA completes
            cookies
                .insert("pending_2fa_remember_me", form.remember_me)
                .map_err(|_| error::ErrorInternalServerError("Session error"))?;

            // Show 2FA input form
            return Ok(Login2FATemplate {
                client,
                error: None,
            }
            .to_response());
        }
        LoginResultStatus::AccountLocked => {
            crate::rate_limit::record_failed_login(&ip);
            log::warn!("Login attempt on locked account: {}", form.username);
            return Err(error::ErrorForbidden("Account locked due to too many failed login attempts. Please try again in 15 minutes."));
        }
        LoginResultStatus::Banned(ban_info) | LoginResultStatus::IpBanned(ban_info) => {
            log::warn!("Login attempt on banned account/IP: {}", form.username);
            let message = if ban_info.is_permanent {
                format!(
                    "Your account has been permanently banned. Reason: {}",
                    ban_info.reason
                )
            } else if let Some(expires) = ban_info.expires_at {
                format!(
                    "Your account is banned until {}. Reason: {}",
                    expires.format("%Y-%m-%d %H:%M UTC"),
                    ban_info.reason
                )
            } else {
                format!("Your account has been banned. Reason: {}", ban_info.reason)
            };
            return Err(error::ErrorForbidden(message));
        }
        LoginResultStatus::EmailNotVerified => {
            log::info!("Login attempt with unverified email: {}", form.username);
            return Err(error::ErrorForbidden("Please verify your email address before logging in. Check your email for a verification link, or <a href=\"/verify-email/resend\">request a new one</a>."));
        }
        LoginResultStatus::Bad2FA => {
            crate::rate_limit::record_failed_login(&ip);
            log::debug!("login failure: invalid 2FA code for {}", form.username);
            return Err(error::ErrorUnauthorized(
                "Invalid two-factor authentication code.",
            ));
        }
        LoginResultStatus::BadName | LoginResultStatus::BadPassword => {
            crate::rate_limit::record_failed_login(&ip);
            log::debug!("login failure: {:?} for {}", user_id.result, form.username);
            // Use generic message to avoid username enumeration
            return Err(error::ErrorUnauthorized("Invalid username or password."));
        }
    };

    let uuid = session::new_session_with_duration(get_sess(), user_id, form.remember_me)
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

    // Redirect to home page on successful login
    Ok(actix_web::HttpResponse::SeeOther()
        .append_header(("Location", "/"))
        .finish())
}

#[post("/login/2fa")]
pub async fn post_login_2fa(
    client: ClientCtx,
    cookies: actix_session::Session,
    req: actix_web::HttpRequest,
    form: web::Form<TotpFormData>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Validate TOTP format
    form.validate().map_err(|e| {
        log::debug!("2FA form validation failed: {}", e);
        error::ErrorBadRequest("Invalid authentication code format")
    })?;

    // Rate limiting for 2FA attempts
    let ip = req
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if let Err(e) = crate::rate_limit::check_login_rate_limit(&ip, "2fa") {
        log::warn!("Rate limit exceeded for 2FA: ip={}", ip);
        return Err(error::ErrorTooManyRequests(format!(
            "Too many 2FA attempts. Please try again in {} seconds.",
            e.retry_after_seconds
        )));
    }

    // Get pending auth state from session
    let user_id: i32 = match cookies.get("pending_2fa_user_id") {
        Ok(Some(id)) => id,
        _ => {
            log::warn!("2FA attempt without pending auth state");
            return Err(error::ErrorBadRequest(
                "No pending authentication. Please login again.",
            ));
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
            return Err(error::ErrorInternalServerError(
                "Authentication configuration error",
            ));
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

    // Retrieve remember_me preference from session (stored during initial login)
    let remember_me = cookies
        .get::<bool>("pending_2fa_remember_me")
        .unwrap_or(Some(false))
        .unwrap_or(false);

    // Clear pending 2FA state
    let _ = cookies.remove("pending_2fa_remember_me");

    // Create session with remember_me preference
    let uuid = session::new_session_with_duration(get_sess(), user_id, remember_me)
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

    // Redirect to home page on successful 2FA login
    Ok(actix_web::HttpResponse::SeeOther()
        .append_header(("Location", "/"))
        .finish())
}

/// Query parameters for login page
#[derive(Deserialize, Default)]
pub struct LoginQuery {
    reset: Option<String>,
}

#[get("/login")]
pub async fn view_login(
    client: ClientCtx,
    cookies: actix_session::Session,
    req: actix_web::HttpRequest,
    query: web::Query<LoginQuery>,
) -> Result<impl Responder, Error> {
    // Check for password reset success message
    let success_message = if query.reset.as_deref() == Some("success") {
        Some("Your password has been reset successfully. Please log in with your new password.")
    } else {
        None
    };

    // Check if CAPTCHA is required based on failed attempts from this IP
    let ip = crate::ip::extract_client_ip(&req)
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let failed_attempts = crate::rate_limit::get_failed_login_count(&ip);
    let captcha_required = crate::captcha::should_require_for_login(failed_attempts);

    let mut tmpl = LoginTemplate {
        client,
        user_id: None,
        logged_in: false,
        username: None,
        token: None,
        success_message,
        captcha_required,
        captcha_provider: crate::captcha::get_provider_name().map(String::from),
        captcha_site_key: crate::captcha::get_site_key().map(String::from),
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
