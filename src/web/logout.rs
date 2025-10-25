use crate::middleware::ClientCtx;
use crate::session::{get_sess, remove_session};
use actix_web::{get, Error, Responder};
use askama_actix::{Template, TemplateToResponse};
use uuid::Uuid;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(view_logout);
}

#[derive(Template)]
#[template(path = "logout.html")]
struct LogoutTemplate {
    client: ClientCtx,
}

#[get("/logout")]
pub async fn view_logout(
    client: ClientCtx,
    cookies: actix_session::Session,
) -> Result<impl Responder, Error> {
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

    // Remove session cookies
    cookies.remove("logged_in");
    cookies.remove("token");

    // Create a new guest context for the logout page
    // This ensures the template shows the user as logged out
    let guest_client = ClientCtx::from_session(&cookies, client.get_permissions().clone()).await;

    Ok(LogoutTemplate {
        client: guest_client,
    }
    .to_response())
}
