use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{
    attachments, posts, profile_posts, threads, ugc_revisions, user_names, user_social_links,
    users,
};
use crate::ugc::{create_ugc, NewUgcPartial};
use crate::user::Profile as UserProfile;
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use chrono::{DateTime, Utc};
use sea_orm::{entity::*, query::*, sea_query::Expr, DatabaseConnection, QueryOrder, Set};
use serde::{Deserialize, Serialize};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(view_member)
        .service(view_member_by_name)
        .service(view_members)
        .service(search_usernames)
        .service(create_profile_post)
        .service(delete_profile_post);
}

/// User statistics for profile display
#[derive(Debug, Clone)]
pub struct UserStatistics {
    pub post_count: i64,
    pub thread_count: i64,
    pub member_since: chrono::NaiveDateTime,
}

/// Display data for a profile wall post
#[derive(Debug, Clone)]
pub struct ProfilePostDisplay {
    pub id: i32,
    pub author_id: Option<i32>,
    pub author_name: Option<String>,
    pub author_avatar_filename: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
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

/// Get profile wall posts for a user
async fn get_profile_posts(
    db: &DatabaseConnection,
    profile_user_id: i32,
    limit: u64,
) -> Result<Vec<ProfilePostDisplay>, sea_orm::DbErr> {
    use sea_orm::FromQueryResult;

    #[derive(Debug, FromQueryResult)]
    struct ProfilePostRow {
        id: i32,
        author_id: Option<i32>,
        author_name: Option<String>,
        author_avatar_filename: Option<String>,
        content: String,
        created_at: chrono::DateTime<chrono::FixedOffset>,
    }

    let rows = profile_posts::Entity::find()
        .filter(profile_posts::Column::ProfileUserId.eq(profile_user_id))
        .left_join(user_names::Entity)
        .column_as(user_names::Column::Name, "author_name")
        .join(
            sea_orm::JoinType::LeftJoin,
            profile_posts::Relation::AuthorAvatar.def(),
        )
        .join(
            sea_orm::JoinType::LeftJoin,
            crate::orm::user_avatars::Relation::Attachments.def(),
        )
        .column_as(attachments::Column::Filename, "author_avatar_filename")
        .join(
            sea_orm::JoinType::InnerJoin,
            profile_posts::Relation::Ugc.def(),
        )
        .join(
            sea_orm::JoinType::InnerJoin,
            crate::orm::ugc::Relation::UgcRevisions.def(),
        )
        .column_as(ugc_revisions::Column::Content, "content")
        .order_by_desc(profile_posts::Column::CreatedAt)
        .limit(limit)
        .into_model::<ProfilePostRow>()
        .all(db)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| ProfilePostDisplay {
            id: row.id,
            author_id: row.author_id,
            author_name: row.author_name,
            author_avatar_filename: row.author_avatar_filename,
            content: row.content,
            created_at: row.created_at.with_timezone(&Utc),
        })
        .collect())
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
        pub profile_posts: Vec<ProfilePostDisplay>,
        pub allow_profile_posts: bool,
    }

    let user_id = path.into_inner().0;
    let db = get_db_pool();

    // Use Profile::get_by_id for full user data including allow_profile_posts
    let user = UserProfile::get_by_id(db, user_id)
        .await
        .map_err(|e| {
            log::error!("error {:?}", e);
            error::ErrorInternalServerError("Couldn't load user.")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found."))?;

    let allow_profile_posts = user.allow_profile_posts;

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

    // Get profile wall posts
    let profile_posts = get_profile_posts(db, user_id, 20)
        .await
        .map_err(|e| {
            log::error!("error getting profile posts: {:?}", e);
            error::ErrorInternalServerError("Couldn't load profile posts.")
        })?;

    Ok(MemberTemplate {
        client,
        user,
        stats,
        badges,
        social_links,
        profile_posts,
        allow_profile_posts,
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

/// Form data for creating a profile post
#[derive(Deserialize)]
pub struct NewProfilePostForm {
    pub content: String,
    pub csrf_token: String,
}

/// Create a new profile wall post
#[post("/members/{user_id}/posts")]
pub async fn create_profile_post(
    client: ClientCtx,
    session: actix_session::Session,
    req: actix_web::HttpRequest,
    path: web::Path<(i32,)>,
    form: web::Form<NewProfilePostForm>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&session, &form.csrf_token)?;

    // Require authentication
    let author_id = client
        .get_id()
        .ok_or_else(|| error::ErrorUnauthorized("Must be logged in to post on profiles"))?;

    let profile_user_id = path.into_inner().0;
    let db = get_db_pool();

    // Check if profile user exists and allows profile posts
    let profile_user = users::Entity::find_by_id(profile_user_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    if !profile_user.allow_profile_posts {
        return Err(error::ErrorForbidden("This user has disabled profile posts"));
    }

    // Validate content
    let content = form.content.trim();
    if content.is_empty() {
        return Err(error::ErrorBadRequest("Post content cannot be empty"));
    }
    if content.len() > 10000 {
        return Err(error::ErrorBadRequest("Post content too long (max 10000 characters)"));
    }

    // Get IP address for moderation
    let ip_id = if let Some(ip_addr) = crate::ip::extract_client_ip(&req) {
        crate::ip::get_or_create_ip_id(&ip_addr).await.ok().flatten()
    } else {
        None
    };

    // Create UGC content
    let ugc_revision = create_ugc(
        db,
        NewUgcPartial {
            ip_id,
            user_id: Some(author_id),
            content,
        },
    )
    .await?;

    // Create the profile post
    let new_post = profile_posts::ActiveModel {
        profile_user_id: Set(profile_user_id),
        author_id: Set(Some(author_id)),
        ugc_id: Set(ugc_revision.ugc_id),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    };
    new_post
        .insert(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Redirect back to profile
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/members/{}/", profile_user_id)))
        .finish())
}

/// Form data for deleting a profile post
#[derive(Deserialize)]
pub struct DeleteProfilePostForm {
    pub csrf_token: String,
}

/// Delete a profile wall post
#[post("/members/{user_id}/posts/{post_id}/delete")]
pub async fn delete_profile_post(
    client: ClientCtx,
    session: actix_session::Session,
    path: web::Path<(i32, i32)>,
    form: web::Form<DeleteProfilePostForm>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&session, &form.csrf_token)?;

    // Require authentication
    let user_id = client
        .get_id()
        .ok_or_else(|| error::ErrorUnauthorized("Must be logged in to delete posts"))?;

    let (profile_user_id, post_id) = path.into_inner();
    let db = get_db_pool();

    // Find the post
    let post = profile_posts::Entity::find_by_id(post_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Post not found"))?;

    // Check authorization: can delete if author or profile owner or admin
    let can_delete = post.author_id == Some(user_id)
        || post.profile_user_id == user_id
        || client.can("admin.user.edit");

    if !can_delete {
        return Err(error::ErrorForbidden("You cannot delete this post"));
    }

    // Delete the post (UGC will be cascade deleted)
    profile_posts::Entity::delete_by_id(post_id)
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Redirect back to profile
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/members/{}/", profile_user_id)))
        .finish())
}
