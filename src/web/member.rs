use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{attachments, posts, threads, user_names, user_social_links, users};
use crate::user::Profile as UserProfile;
use actix_web::{error, get, web, Error, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use sea_orm::{entity::*, query::*, sea_query::Expr, DatabaseConnection, QueryOrder};
use serde::{Deserialize, Serialize};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(view_member)
        .service(view_member_by_name)
        .service(view_members)
        .service(search_usernames);
}

/// User statistics for profile display
#[derive(Debug, Clone)]
pub struct UserStatistics {
    pub post_count: i64,
    pub thread_count: i64,
    pub member_since: chrono::NaiveDateTime,
}

/// Get user statistics for profile display
async fn get_user_statistics(
    db: &DatabaseConnection,
    user_id: i32,
    created_at: chrono::NaiveDateTime,
) -> Result<UserStatistics, sea_orm::DbErr> {
    // Count posts by this user
    let post_count = posts::Entity::find()
        .filter(posts::Column::UserId.eq(user_id))
        .count(db)
        .await?;

    // Count threads created by this user
    let thread_count = threads::Entity::find()
        .filter(threads::Column::UserId.eq(user_id))
        .count(db)
        .await?;

    Ok(UserStatistics {
        post_count: post_count as i64,
        thread_count: thread_count as i64,
        member_since: created_at,
    })
}

#[get("/members/{user_id}/")]
pub async fn view_member(
    client: ClientCtx,
    path: web::Path<(i32,)>,
) -> Result<impl Responder, Error> {
    #[derive(Template)]
    #[template(path = "member.html")]
    pub struct MemberTemplate {
        pub client: ClientCtx,
        pub user: UserProfile,
        pub stats: UserStatistics,
        pub badges: Vec<crate::badges::UserBadge>,
        pub social_links: Vec<user_social_links::Model>,
    }

    let user_id = path.into_inner().0;
    let db = get_db_pool();

    let user = users::Entity::find_by_id(user_id)
        .left_join(user_names::Entity)
        .column_as(user_names::Column::Name, "name")
        .left_join(attachments::Entity)
        .column_as(attachments::Column::Filename, "avatar_filename")
        .column_as(attachments::Column::FileHeight, "avatar_height")
        .column_as(attachments::Column::FileWidth, "avatar_width")
        .column(users::Column::Bio)
        .column(users::Column::Location)
        .column(users::Column::WebsiteUrl)
        .column(users::Column::Signature)
        .into_model::<UserProfile>()
        .one(db)
        .await
        .map_err(|e| {
            log::error!("error {:?}", e);
            error::ErrorInternalServerError("Couldn't load user.")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found."))?;

    // Get user statistics
    let stats = get_user_statistics(db, user_id, user.created_at)
        .await
        .map_err(|e| {
            log::error!("error getting user stats: {:?}", e);
            error::ErrorInternalServerError("Couldn't load user statistics.")
        })?;

    // Get user badges
    let badges = crate::badges::get_user_badges(db, user_id)
        .await
        .map_err(|e| {
            log::error!("error getting user badges: {:?}", e);
            error::ErrorInternalServerError("Couldn't load user badges.")
        })?;

    // Get user social links (only visible ones)
    let social_links = user_social_links::Entity::find()
        .filter(user_social_links::Column::UserId.eq(user_id))
        .filter(user_social_links::Column::IsVisible.eq(true))
        .order_by_asc(user_social_links::Column::DisplayOrder)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("error getting user social links: {:?}", e);
            error::ErrorInternalServerError("Couldn't load user social links.")
        })?;

    Ok(MemberTemplate {
        client,
        user,
        stats,
        badges,
        social_links,
    }
    .to_response())
}

/// View member profile by username (for @mention links)
#[get("/members/@{username}")]
pub async fn view_member_by_name(path: web::Path<String>) -> Result<impl Responder, Error> {
    let username = path.into_inner();
    let db = get_db_pool();

    // Look up user by username
    let user_name = user_names::Entity::find()
        .filter(user_names::Column::Name.eq(username.clone()))
        .one(db)
        .await
        .map_err(|e| {
            log::error!("error looking up username: {:?}", e);
            error::ErrorInternalServerError("Couldn't look up user.")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found."))?;

    // Redirect to canonical URL with user_id
    Ok(actix_web::HttpResponse::Found()
        .append_header(("Location", format!("/members/{}/", user_name.user_id)))
        .finish())
}

#[get("/members")]
pub async fn view_members(client: ClientCtx) -> impl Responder {
    #[derive(Template)]
    #[template(path = "members.html")]
    pub struct MembersTemplate {
        pub client: ClientCtx,
        pub users: Vec<UserProfile>,
    }

    match users::Entity::find()
        .left_join(user_names::Entity)
        .column_as(user_names::Column::Name, "name")
        .left_join(attachments::Entity)
        .column_as(attachments::Column::Filename, "avatar_filename")
        .column_as(attachments::Column::FileHeight, "avatar_height")
        .column_as(attachments::Column::FileWidth, "avatar_width")
        .column(users::Column::Bio)
        .column(users::Column::Location)
        .column(users::Column::WebsiteUrl)
        .column(users::Column::Signature)
        .into_model::<UserProfile>()
        .all(get_db_pool())
        .await
    {
        Ok(users) => Ok(MembersTemplate { client, users }.to_response()),
        Err(e) => {
            log::error!("error {:?}", e);
            Err(error::ErrorInternalServerError("Couldn't load users"))
        }
    }
}

/// Query parameters for username search
#[derive(Deserialize)]
pub struct UsernameSearchQuery {
    q: String,
}

/// Response item for username search
#[derive(Serialize)]
pub struct UsernameSearchResult {
    id: i32,
    username: String,
}

/// Search usernames by prefix for @mention autocomplete
#[get("/api/users/search")]
pub async fn search_usernames(
    client: ClientCtx,
    query: web::Query<UsernameSearchQuery>,
) -> Result<HttpResponse, Error> {
    // Require authentication to prevent enumeration
    if !client.is_user() {
        return Err(error::ErrorUnauthorized("Must be logged in"));
    }

    let search_term = query.q.trim();

    // Require at least 1 character
    if search_term.is_empty() {
        return Ok(HttpResponse::Ok().json(Vec::<UsernameSearchResult>::new()));
    }

    // Limit search term length
    let search_term = if search_term.len() > 50 {
        &search_term[..50]
    } else {
        search_term
    };

    let db = get_db_pool();

    // Search for usernames starting with the search term (case-insensitive)
    let results = user_names::Entity::find()
        .filter(Expr::cust_with_values(
            "LOWER(name) LIKE $1",
            [format!("{}%", search_term.to_lowercase())],
        ))
        .order_by_asc(user_names::Column::Name)
        .limit(10)
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let response: Vec<UsernameSearchResult> = results
        .into_iter()
        .map(|u| UsernameSearchResult {
            id: u.user_id,
            username: u.name,
        })
        .collect();

    Ok(HttpResponse::Ok().json(response))
}
