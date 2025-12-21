/// CSRF (Cross-Site Request Forgery) protection
///
/// Protects against CSRF attacks by requiring a valid CSRF token on all
/// state-changing HTTP methods (POST, PUT, PATCH, DELETE).
///
/// The token is:
/// - Generated once per session
/// - Stored in the session cookie
/// - Must be included in forms as a hidden field named "csrf_token"
/// - Validated in handlers before processing state-changing requests
///
/// Usage in templates:
/// ```html,ignore
/// <form method="post">
///     <input type="hidden" name="csrf_token" value="{{ client.get_csrf_token() }}">
///     <!-- other form fields -->
/// </form>
/// ```
///
/// Usage in handlers:
/// ```rust,ignore
/// use crate::middleware::csrf::validate_csrf_token;
///
/// #[post("/some-endpoint")]
/// async fn handler(
///     session: Session,
///     form: web::Form<MyFormData>,
/// ) -> Result<impl Responder, Error> {
///     // Validate CSRF token
///     validate_csrf_token(&session, &form.csrf_token)?;
///
///     // Process form...
/// }
/// ```
use actix_web::{error, Error};
use rand::{distributions::Alphanumeric, Rng};

pub const CSRF_TOKEN_LENGTH: usize = 32;
const CSRF_SESSION_KEY: &str = "csrf_token";

/// Generate a new CSRF token
pub fn generate_csrf_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(CSRF_TOKEN_LENGTH)
        .map(char::from)
        .collect()
}

/// Get or create CSRF token for the current session
///
/// This is automatically called when ClientCtx is created from session,
/// ensuring every request has a CSRF token available.
pub fn get_or_create_csrf_token(session: &actix_session::Session) -> Result<String, Error> {
    // Try to get existing token
    match session.get::<String>(CSRF_SESSION_KEY) {
        Ok(Some(token)) => Ok(token),
        _ => {
            // Generate new token
            let token = generate_csrf_token();
            session
                .insert(CSRF_SESSION_KEY, token.clone())
                .map_err(|_| error::ErrorInternalServerError("Failed to store CSRF token"))?;
            Ok(token)
        }
    }
}

/// Validate CSRF token from form data
///
/// Call this at the beginning of any handler that processes state-changing requests.
///
/// # Example
/// ```rust,ignore
/// #[derive(Deserialize)]
/// struct MyForm {
///     csrf_token: String,
///     // other fields...
/// }
///
/// #[post("/update")]
/// async fn update(
///     session: Session,
///     form: web::Form<MyForm>,
/// ) -> Result<impl Responder, Error> {
///     validate_csrf_token(&session, &form.csrf_token)?;
///     // Process request...
/// }
/// ```
pub fn validate_csrf_token(
    session: &actix_session::Session,
    provided_token: &str,
) -> Result<(), Error> {
    let expected_token = session
        .get::<String>(CSRF_SESSION_KEY)
        .map_err(|_| error::ErrorInternalServerError("Failed to get CSRF token"))?
        .ok_or_else(|| error::ErrorForbidden("CSRF token not found in session"))?;

    if provided_token != expected_token {
        log::warn!("CSRF token validation failed");
        return Err(error::ErrorForbidden("Invalid CSRF token"));
    }

    Ok(())
}
