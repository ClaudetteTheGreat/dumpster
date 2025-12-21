use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::user::Profile as UserProfile;
use actix_multipart::Multipart;
use actix_web::{error, get, post, Error, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use chrono::Utc;
use sea_orm::{entity::*, ColumnTrait, EntityTrait, QueryFilter};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(update_avatar)
        .service(delete_avatar)
        .service(update_preferences)
        .service(update_profile)
        .service(view_account);
}

#[derive(Template)]
#[template(path = "account.html")]
pub struct AccountTemplate {
    pub client: ClientCtx,
    pub profile: UserProfile,
}

#[post("/account/avatar")]
async fn update_avatar(
    client: ClientCtx,
    cookies: actix_session::Session,
    mutipart: Option<Multipart>,
) -> impl Responder {
    use crate::filesystem::{
        deduplicate_payload, insert_payload_as_attachment, save_field_as_temp_file,
    };
    use crate::orm::user_avatars;
    use futures::{StreamExt, TryStreamExt};
    use std::str;

    if !client.is_user() {
        return Err(error::ErrorUnauthorized(
            "You must be logged in to do that.",
        ));
    }

    let mut csrf_token: Option<String> = None;
    let mut avatar_processed = false;

    if let Some(mut fields) = mutipart {
        while let Ok(Some(mut field)) = fields.try_next().await {
            let disposition = field.content_disposition();
            if let Some(field_name) = disposition.get_name() {
                match field_name {
                    "csrf_token" => {
                        let mut buf: Vec<u8> = Vec::with_capacity(128);
                        while let Some(chunk) = field.next().await {
                            let bytes = chunk.map_err(|e| {
                                log::error!("update_avatar: multipart read error: {}", e);
                                actix_web::error::ErrorBadRequest("Error interpreting user input.")
                            })?;
                            buf.extend(bytes.to_owned());
                        }
                        csrf_token = Some(str::from_utf8(&buf).unwrap().to_owned());
                    }
                    "avatar" => {
                        // Validate CSRF token before processing avatar
                        if csrf_token.is_none() {
                            return Err(error::ErrorBadRequest(
                                "CSRF token must be provided before file upload",
                            ));
                        }
                        let token = csrf_token.as_ref().unwrap();
                        crate::middleware::csrf::validate_csrf_token(&cookies, token)?;

                        avatar_processed = true;
                        // Save the file to a temporary location and get payload data.
                        let payload = match save_field_as_temp_file(&mut field).await? {
                            Some(payload) => payload,
                            None => {
                                return Err(error::ErrorBadRequest("Upload is empty or improper."))
                            }
                        };

                        // Pass file through deduplication and receive a response..
                        let response = match deduplicate_payload(&payload).await {
                            Some(response) => response,
                            None => match insert_payload_as_attachment(payload, None).await? {
                                Some(response) => response,
                                None => {
                                    return Err(error::ErrorBadRequest(
                                        "Upload is empty or improper.",
                                    ))
                                }
                            },
                        };

                        match user_avatars::Entity::insert(user_avatars::ActiveModel {
                            user_id: Set(client.get_id().unwrap()),
                            attachment_id: Set(response.id),
                            created_at: Set(Utc::now().naive_utc()),
                        })
                        .exec(get_db_pool())
                        .await
                        {
                            Ok(_) => {}
                            Err(err) => log::warn!("SQL error when inserting avatar: {:?}", err),
                        };
                    }
                    _ => {
                        return Err(error::ErrorBadRequest(format!(
                            "Unknown field '{}'",
                            field_name
                        )))
                    }
                }
            }
        }
    }

    // Validate CSRF token if avatar wasn't processed (token might come after avatar in stream)
    if !avatar_processed {
        let token = csrf_token.ok_or_else(|| error::ErrorBadRequest("CSRF token missing"))?;
        crate::middleware::csrf::validate_csrf_token(&cookies, &token)?;
    }

    Ok(HttpResponse::Found()
        .append_header(("Location", "/account"))
        .finish())
}

#[post("/account/avatar/delete")]
async fn delete_avatar(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: actix_web::web::Form<std::collections::HashMap<String, String>>,
) -> Result<impl Responder, Error> {
    use crate::orm::user_avatars;

    let user_id = client.require_login()?;

    // Validate CSRF token
    let csrf_token = form
        .get("csrf_token")
        .ok_or_else(|| error::ErrorBadRequest("CSRF token missing"))?;
    crate::middleware::csrf::validate_csrf_token(&cookies, csrf_token)?;

    // Delete all avatars for this user
    // Note: We don't delete the attachment file itself because it might be
    // deduplicated and used by other users. Attachment cleanup should be
    // handled by a separate garbage collection process.
    user_avatars::Entity::delete_many()
        .filter(user_avatars::Column::UserId.eq(user_id))
        .exec(get_db_pool())
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", "/account"))
        .finish())
}

#[post("/account/preferences")]
async fn update_preferences(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: actix_web::web::Form<std::collections::HashMap<String, String>>,
) -> Result<impl Responder, Error> {
    use crate::orm::users;

    let user_id = client.require_login()?;

    // Validate CSRF token
    let csrf_token = form
        .get("csrf_token")
        .ok_or_else(|| error::ErrorBadRequest("CSRF token missing"))?;
    crate::middleware::csrf::validate_csrf_token(&cookies, csrf_token)?;

    // Get and validate posts_per_page
    let posts_per_page_str = form
        .get("posts_per_page")
        .ok_or_else(|| error::ErrorBadRequest("posts_per_page missing"))?;

    let posts_per_page: i32 = posts_per_page_str
        .parse()
        .map_err(|_| error::ErrorBadRequest("Invalid posts_per_page value"))?;

    // Validate it's one of the allowed values
    if ![10, 25, 50, 100].contains(&posts_per_page) {
        return Err(error::ErrorBadRequest(
            "posts_per_page must be one of: 10, 25, 50, 100",
        ));
    }

    // Get and validate theme
    let theme = form
        .get("theme")
        .ok_or_else(|| error::ErrorBadRequest("theme missing"))?;

    // Validate it's one of the allowed values
    if !["light", "dark", "auto"].contains(&theme.as_str()) {
        return Err(error::ErrorBadRequest(
            "theme must be one of: light, dark, auto",
        ));
    }

    // Update the user's preferences
    let mut user: users::ActiveModel = users::Entity::find_by_id(user_id)
        .one(get_db_pool())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?
        .into();

    user.posts_per_page = Set(posts_per_page);
    user.theme = Set(theme.to_string());
    user.update(get_db_pool())
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", "/account"))
        .finish())
}

#[post("/account/profile")]
async fn update_profile(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: actix_web::web::Form<std::collections::HashMap<String, String>>,
) -> Result<impl Responder, Error> {
    use crate::orm::users;

    let user_id = client.require_login()?;

    // Validate CSRF token
    let csrf_token = form
        .get("csrf_token")
        .ok_or_else(|| error::ErrorBadRequest("CSRF token missing"))?;
    crate::middleware::csrf::validate_csrf_token(&cookies, csrf_token)?;

    // Get and validate bio (max 2000 chars)
    let bio = form
        .get("bio")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(ref b) = bio {
        if b.len() > 2000 {
            return Err(error::ErrorBadRequest(
                "Bio must be 2000 characters or less",
            ));
        }
    }

    // Get and validate location (max 255 chars)
    let location = form
        .get("location")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(ref l) = location {
        if l.len() > 255 {
            return Err(error::ErrorBadRequest(
                "Location must be 255 characters or less",
            ));
        }
    }

    // Get and validate website URL (max 2048 chars, must be valid URL)
    let website_url = form
        .get("website_url")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(ref url_str) = website_url {
        if url_str.len() > 2048 {
            return Err(error::ErrorBadRequest(
                "Website URL must be 2048 characters or less",
            ));
        }
        // Validate URL format
        match url::Url::parse(url_str) {
            Ok(parsed) => {
                // Only allow http/https schemes
                if parsed.scheme() != "http" && parsed.scheme() != "https" {
                    return Err(error::ErrorBadRequest("Website URL must use http or https"));
                }
            }
            Err(_) => {
                return Err(error::ErrorBadRequest("Invalid website URL format"));
            }
        }
    }

    // Get and validate signature (max 500 chars)
    let signature = form
        .get("signature")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(ref sig) = signature {
        if sig.len() > 500 {
            return Err(error::ErrorBadRequest(
                "Signature must be 500 characters or less",
            ));
        }
    }

    // Update the user's profile
    let mut user: users::ActiveModel = users::Entity::find_by_id(user_id)
        .one(get_db_pool())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?
        .into();

    user.bio = Set(bio);
    user.location = Set(location);
    user.website_url = Set(website_url);
    user.signature = Set(signature);

    user.update(get_db_pool())
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", "/account"))
        .finish())
}

#[get("/account")]
async fn view_account(client: ClientCtx) -> Result<impl Responder, Error> {
    if !client.is_user() {
        return Err(error::ErrorUnauthorized(
            "You must be logged in to do that.",
        ));
    }

    let db = get_db_pool();
    let profile = UserProfile::get_by_id(db, client.get_id().unwrap())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorInternalServerError("Unable to find account."))?;

    Ok(AccountTemplate { client, profile }.to_response())
}
