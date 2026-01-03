use crate::session::{get_sess, remove_session};
use actix_web::{get, http::header, Error, HttpResponse, Responder};
use uuid::Uuid;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(view_logout);
}

#[get("/logout")]
pub async fn view_logout(cookies: actix_session::Session) -> Result<impl Responder, Error> {
    // Remove session from database and session cache
    match cookies.get::<String>("token") {
        Ok(Some(uuid)) => match Uuid::parse_str(&uuid) {
            Ok(uuid) => {
                if let Err(e) = remove_session(get_sess(), uuid).await {
                    log::error!("view_logout: remove_session() {}", e);
                }
            }
            Err(e) => {
                log::error!("view_logout: parse_str() {}", e);
            }
        },
        Ok(None) => {
            log::debug!("view_logout: missing token (already logged out?)");
        }
        Err(e) => {
            log::error!("view_logout: cookies.get() {}", e);
        }
    }

    // Remove session cookies and cached data
    cookies.remove("logged_in");
    cookies.remove("token");
    crate::group::invalidate_session_groups(&cookies);

    // Redirect to home page
    // This ensures the page loads with fresh guest context and avoids any caching issues
    Ok(HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/"))
        .finish())
}
