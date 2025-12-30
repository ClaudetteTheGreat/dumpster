use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::themes;
use crate::orm::user_social_links::{self, SocialPlatform};
use crate::user::Profile as UserProfile;
use actix_multipart::Multipart;
use actix_web::{error, get, post, Error, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use chrono::Utc;
use sea_orm::{entity::*, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(update_avatar)
        .service(delete_avatar)
        .service(update_preferences)
        .service(update_profile)
        .service(update_social_links)
        .service(delete_social_link)
        .service(view_account);
}

#[derive(Template)]
#[template(path = "account.html")]
pub struct AccountTemplate {
    pub client: ClientCtx,
    pub profile: UserProfile,
    pub social_links: Vec<user_social_links::Model>,
    pub available_platforms: Vec<SocialPlatform>,
    pub available_themes: Vec<themes::Model>,
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
    let theme_slug = form
        .get("theme")
        .ok_or_else(|| error::ErrorBadRequest("theme missing"))?;

    // Handle "auto" specially - set theme_auto flag and use light as fallback
    let (theme_value, theme_auto) = if theme_slug == "auto" {
        (Some("light".to_string()), true)
    } else {
        // Validate theme slug exists and is active
        if !crate::theme::theme_exists(theme_slug) {
            return Err(error::ErrorBadRequest("Invalid theme selection"));
        }
        (Some(theme_slug.to_string()), false)
    };

    // Get show_online preference (checkbox, so may not be present if unchecked)
    let show_online = form
        .get("show_online")
        .map(|v| v == "true")
        .unwrap_or(false);

    // Update the user's preferences
    let mut user: users::ActiveModel = users::Entity::find_by_id(user_id)
        .one(get_db_pool())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?
        .into();

    user.posts_per_page = Set(posts_per_page);
    user.theme = Set(theme_value);
    user.theme_auto = Set(theme_auto);
    user.show_online = Set(show_online);
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

    // Get and validate custom title (max 100 chars)
    let custom_title = form
        .get("custom_title")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(ref title) = custom_title {
        if title.len() > 100 {
            return Err(error::ErrorBadRequest(
                "Custom title must be 100 characters or less",
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
    user.custom_title = Set(custom_title);

    user.update(get_db_pool())
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", "/account"))
        .finish())
}

#[post("/account/social-links")]
async fn update_social_links(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: actix_web::web::Form<std::collections::HashMap<String, String>>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    // Validate CSRF token
    let csrf_token = form
        .get("csrf_token")
        .ok_or_else(|| error::ErrorBadRequest("CSRF token missing"))?;
    crate::middleware::csrf::validate_csrf_token(&cookies, csrf_token)?;

    let db = get_db_pool();

    // Get platform and username from form
    let platform_str = form
        .get("platform")
        .ok_or_else(|| error::ErrorBadRequest("Platform is required"))?;
    let platform = SocialPlatform::parse(platform_str)
        .ok_or_else(|| error::ErrorBadRequest("Invalid platform"))?;

    let username = form
        .get("username")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error::ErrorBadRequest("Username is required"))?;

    // Validate username length
    if username.len() > 255 {
        return Err(error::ErrorBadRequest(
            "Username must be 255 characters or less",
        ));
    }

    // Get optional custom URL (for Discord, Website, Other)
    let url = form
        .get("url")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Validate URL if provided
    if let Some(ref url_str) = url {
        if url_str.len() > 500 {
            return Err(error::ErrorBadRequest("URL must be 500 characters or less"));
        }
        // Validate URL format for platforms that need custom URLs
        if matches!(
            platform,
            SocialPlatform::Website | SocialPlatform::Discord | SocialPlatform::Other
        ) {
            match url::Url::parse(url_str) {
                Ok(parsed) => {
                    if parsed.scheme() != "http" && parsed.scheme() != "https" {
                        return Err(error::ErrorBadRequest("URL must use http or https"));
                    }
                }
                Err(_) => {
                    return Err(error::ErrorBadRequest("Invalid URL format"));
                }
            }
        }
    }

    // Check if user already has this platform linked
    let existing = user_social_links::Entity::find()
        .filter(user_social_links::Column::UserId.eq(user_id))
        .filter(user_social_links::Column::Platform.eq(platform.clone()))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    if let Some(existing_link) = existing {
        // Update existing link
        let mut link: user_social_links::ActiveModel = existing_link.into();
        link.username = Set(username);
        link.url = Set(url);
        link.updated_at = Set(Utc::now());
        link.update(db)
            .await
            .map_err(error::ErrorInternalServerError)?;
    } else {
        // Get next display order
        let max_order: Option<i32> = user_social_links::Entity::find()
            .filter(user_social_links::Column::UserId.eq(user_id))
            .order_by_desc(user_social_links::Column::DisplayOrder)
            .one(db)
            .await
            .map_err(error::ErrorInternalServerError)?
            .map(|l| l.display_order);
        let next_order = max_order.unwrap_or(-1) + 1;

        // Insert new link
        let new_link = user_social_links::ActiveModel {
            user_id: Set(user_id),
            platform: Set(platform),
            username: Set(username),
            url: Set(url),
            display_order: Set(next_order),
            is_visible: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };
        new_link
            .insert(db)
            .await
            .map_err(error::ErrorInternalServerError)?;
    }

    Ok(HttpResponse::Found()
        .append_header(("Location", "/account"))
        .finish())
}

#[post("/account/social-links/delete")]
async fn delete_social_link(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: actix_web::web::Form<std::collections::HashMap<String, String>>,
) -> Result<impl Responder, Error> {
    let user_id = client.require_login()?;

    // Validate CSRF token
    let csrf_token = form
        .get("csrf_token")
        .ok_or_else(|| error::ErrorBadRequest("CSRF token missing"))?;
    crate::middleware::csrf::validate_csrf_token(&cookies, csrf_token)?;

    let db = get_db_pool();

    // Get platform to delete
    let platform_str = form
        .get("platform")
        .ok_or_else(|| error::ErrorBadRequest("Platform is required"))?;
    let platform = SocialPlatform::parse(platform_str)
        .ok_or_else(|| error::ErrorBadRequest("Invalid platform"))?;

    // Delete the link (only if it belongs to this user)
    user_social_links::Entity::delete_many()
        .filter(user_social_links::Column::UserId.eq(user_id))
        .filter(user_social_links::Column::Platform.eq(platform))
        .exec(db)
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
    let user_id = client.get_id().unwrap();

    let profile = UserProfile::get_by_id(db, user_id)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorInternalServerError("Unable to find account."))?;

    // Fetch user's social links
    let social_links = user_social_links::Entity::find()
        .filter(user_social_links::Column::UserId.eq(user_id))
        .order_by_asc(user_social_links::Column::DisplayOrder)
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Get available platforms (exclude ones already added)
    let used_platforms: Vec<_> = social_links.iter().map(|l| l.platform.clone()).collect();
    let available_platforms: Vec<_> = SocialPlatform::all()
        .into_iter()
        .filter(|p| !used_platforms.contains(p))
        .collect();

    // Get available themes
    let available_themes = crate::theme::get_active_themes();

    Ok(AccountTemplate {
        client,
        profile,
        social_links,
        available_platforms,
        available_themes,
    }
    .to_response())
}
