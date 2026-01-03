use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{
    attachments, posts, profile_posts, threads, ugc_revisions, user_follows, user_names,
    user_social_links, users,
};
use crate::ugc::{create_ugc, NewUgcPartial};
use crate::user::Profile as UserProfile;
use actix_web::{error, get, post, web, Error, HttpRequest, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use chrono::{DateTime, Utc};
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{entity::*, query::*, sea_query::Expr, DatabaseConnection, QueryOrder, Set};
use serde::{Deserialize, Serialize};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(view_member)
        .service(view_member_by_name)
        .service(view_members)
        .service(search_usernames)
        .service(create_profile_post)
        .service(delete_profile_post)
        .service(follow_user)
        .service(unfollow_user)
        .service(view_followers)
        .service(view_following);
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
        pub is_following: bool,
    }

    let user_id = path.into_inner().0;
    let current_user_id = client.get_id();
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
    let profile_posts = get_profile_posts(db, user_id, 20).await.map_err(|e| {
        log::error!("error getting profile posts: {:?}", e);
        error::ErrorInternalServerError("Couldn't load profile posts.")
    })?;

    // Check if current user follows this profile
    let is_following = if let Some(current_id) = current_user_id {
        if current_id != user_id {
            user_follows::Entity::find()
                .filter(user_follows::Column::FollowerId.eq(current_id))
                .filter(user_follows::Column::FollowingId.eq(user_id))
                .one(db)
                .await
                .map_err(error::ErrorInternalServerError)?
                .is_some()
        } else {
            false // Can't follow yourself
        }
    } else {
        false
    };

    Ok(MemberTemplate {
        client,
        user,
        stats,
        badges,
        social_links,
        profile_posts,
        allow_profile_posts,
        is_following,
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
        .column_as(
            Expr::cust_with_values(
                "(SELECT COUNT(*) FROM posts WHERE posts.user_id = users.id)",
                std::iter::empty::<sea_orm::Value>(),
            ),
            "post_count",
        )
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
    req: HttpRequest,
    client: ClientCtx,
    query: web::Query<UsernameSearchQuery>,
) -> Result<HttpResponse, Error> {
    // Require authentication to prevent enumeration
    if !client.is_user() {
        return Err(error::ErrorUnauthorized("Must be logged in"));
    }

    // Rate limiting - use user_id or IP
    let rate_limit_id = client
        .get_id()
        .map(|id: i32| id.to_string())
        .unwrap_or_else(|| {
            crate::ip::extract_client_ip(&req)
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

    if let Err(e) = crate::rate_limit::check_api_rate_limit(&rate_limit_id) {
        log::warn!("User search rate limit exceeded for: {}", rate_limit_id);
        return Err(error::ErrorTooManyRequests(format!(
            "Too many requests. Please try again in {} seconds.",
            e.retry_after_seconds
        )));
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
    use sea_orm::{DbBackend, FromQueryResult, Statement};

    let results = user_names::Model::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"
            SELECT user_id, name
            FROM user_names
            WHERE LOWER(name) LIKE LOWER($1 || '%')
            ORDER BY name
            LIMIT 10
        "#,
        [search_term.into()],
    ))
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
    req: HttpRequest,
    path: web::Path<(i32,)>,
    form: web::Form<NewProfilePostForm>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&session, &form.csrf_token)?;

    // Require authentication
    let author_id = client
        .get_id()
        .ok_or_else(|| error::ErrorUnauthorized("Must be logged in to post on profiles"))?;

    // Rate limiting - uses post_creation rate limit (covers posts, profile posts, messages)
    if let Err(e) = crate::rate_limit::check_post_rate_limit(author_id) {
        log::warn!("Profile post rate limit exceeded for user: {}", author_id);
        return Err(error::ErrorTooManyRequests(format!(
            "Too many posts. Please try again in {} seconds.",
            e.retry_after_seconds
        )));
    }

    let profile_user_id = path.into_inner().0;
    let db = get_db_pool();

    // Check if profile user exists and allows profile posts
    let profile_user = users::Entity::find_by_id(profile_user_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    if !profile_user.allow_profile_posts {
        return Err(error::ErrorForbidden(
            "This user has disabled profile posts",
        ));
    }

    // Get the profile user's name for activity recording
    let profile_user_name = user_names::Entity::find()
        .filter(user_names::Column::UserId.eq(profile_user_id))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .map(|n| n.name)
        .unwrap_or_else(|| "Unknown".to_string());

    // Validate content
    let content = form.content.trim();
    if content.is_empty() {
        return Err(error::ErrorBadRequest("Post content cannot be empty"));
    }
    if content.len() > 10000 {
        return Err(error::ErrorBadRequest(
            "Post content too long (max 10000 characters)",
        ));
    }

    // Get IP address for moderation
    let ip_id = if let Some(ip_addr) = crate::ip::extract_client_ip(&req) {
        crate::ip::get_or_create_ip_id(&ip_addr)
            .await
            .ok()
            .flatten()
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

    // Record activity for the feed (async, non-blocking)
    let content_preview = if content.len() > 200 {
        format!("{}...", &content[..197])
    } else {
        content.to_string()
    };
    actix::spawn(async move {
        if let Err(e) = crate::activities::record_profile_post_created(
            author_id,
            profile_user_id,
            &profile_user_name,
            &content_preview,
        )
        .await
        {
            log::warn!("Failed to record profile post activity: {}", e);
        }
    });

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

// =============================================================================
// User Follow/Unfollow
// =============================================================================

#[derive(Deserialize)]
pub struct FollowForm {
    csrf_token: String,
}

/// Follow a user
#[post("/members/{user_id}/follow")]
pub async fn follow_user(
    client: ClientCtx,
    session: actix_session::Session,
    path: web::Path<(i32,)>,
    form: web::Form<FollowForm>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&session, &form.csrf_token)?;

    // Require authentication
    let follower_id = client
        .get_id()
        .ok_or_else(|| error::ErrorUnauthorized("Must be logged in to follow users"))?;

    let following_id = path.into_inner().0;

    // Can't follow yourself
    if follower_id == following_id {
        return Err(error::ErrorBadRequest("Cannot follow yourself"));
    }

    let db = get_db_pool();

    // Check if target user exists and get their name
    let _target_user = users::Entity::find_by_id(following_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Get the target user's name
    let target_user_name = user_names::Entity::find()
        .filter(user_names::Column::UserId.eq(following_id))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .map(|n| n.name)
        .unwrap_or_else(|| "Unknown".to_string());

    // Check if already following
    let existing = user_follows::Entity::find()
        .filter(user_follows::Column::FollowerId.eq(follower_id))
        .filter(user_follows::Column::FollowingId.eq(following_id))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    if existing.is_some() {
        // Already following - just redirect back
        return Ok(HttpResponse::SeeOther()
            .append_header(("Location", format!("/members/{}/", following_id)))
            .finish());
    }

    // Create follow relationship
    let follow = user_follows::ActiveModel {
        follower_id: Set(follower_id),
        following_id: Set(following_id),
        ..Default::default()
    };

    follow
        .insert(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Record activity for the feed (async, non-blocking)
    actix::spawn(async move {
        if let Err(e) =
            crate::activities::record_user_followed(follower_id, following_id, &target_user_name)
                .await
        {
            log::warn!("Failed to record user follow activity: {}", e);
        }
    });

    // Redirect back to profile
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/members/{}/", following_id)))
        .finish())
}

/// Unfollow a user
#[post("/members/{user_id}/unfollow")]
pub async fn unfollow_user(
    client: ClientCtx,
    session: actix_session::Session,
    path: web::Path<(i32,)>,
    form: web::Form<FollowForm>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&session, &form.csrf_token)?;

    // Require authentication
    let follower_id = client
        .get_id()
        .ok_or_else(|| error::ErrorUnauthorized("Must be logged in to unfollow users"))?;

    let following_id = path.into_inner().0;
    let db = get_db_pool();

    // Delete follow relationship if it exists
    user_follows::Entity::delete_many()
        .filter(user_follows::Column::FollowerId.eq(follower_id))
        .filter(user_follows::Column::FollowingId.eq(following_id))
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Redirect back to profile
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/members/{}/", following_id)))
        .finish())
}

// =============================================================================
// Followers/Following Lists
// =============================================================================

/// Display info for a user in followers/following list
#[derive(Debug, Clone)]
pub struct FollowUserDisplay {
    pub id: i32,
    pub name: String,
    pub avatar_filename: Option<String>,
    pub custom_title: Option<String>,
    pub followed_at: DateTime<Utc>,
}

/// Get a user's followers
async fn get_followers(
    db: &DatabaseConnection,
    user_id: i32,
    limit: u64,
) -> Result<Vec<FollowUserDisplay>, sea_orm::DbErr> {
    use sea_orm::{DbBackend, Statement};

    let sql = r#"
        SELECT
            uf.follower_id as id,
            un.name,
            a.filename as avatar_filename,
            u.custom_title,
            uf.created_at as followed_at
        FROM user_follows uf
        JOIN users u ON u.id = uf.follower_id
        LEFT JOIN user_names un ON un.user_id = u.id
        LEFT JOIN user_avatars ua ON ua.user_id = u.id
        LEFT JOIN attachments a ON a.id = ua.attachment_id
        WHERE uf.following_id = $1
        ORDER BY uf.created_at DESC
        LIMIT $2
    "#;

    let results = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            vec![user_id.into(), (limit as i64).into()],
        ))
        .await?;

    Ok(results
        .iter()
        .map(|row| FollowUserDisplay {
            id: row.try_get::<i32>("", "id").unwrap_or(0),
            name: row
                .try_get::<String>("", "name")
                .unwrap_or_else(|_| "Unknown".to_string()),
            avatar_filename: row
                .try_get::<Option<String>>("", "avatar_filename")
                .ok()
                .flatten(),
            custom_title: row
                .try_get::<Option<String>>("", "custom_title")
                .ok()
                .flatten(),
            followed_at: row
                .try_get::<DateTimeWithTimeZone>("", "followed_at")
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
        .collect())
}

/// Get users that a user follows
async fn get_following(
    db: &DatabaseConnection,
    user_id: i32,
    limit: u64,
) -> Result<Vec<FollowUserDisplay>, sea_orm::DbErr> {
    use sea_orm::{DbBackend, Statement};

    let sql = r#"
        SELECT
            uf.following_id as id,
            un.name,
            a.filename as avatar_filename,
            u.custom_title,
            uf.created_at as followed_at
        FROM user_follows uf
        JOIN users u ON u.id = uf.following_id
        LEFT JOIN user_names un ON un.user_id = u.id
        LEFT JOIN user_avatars ua ON ua.user_id = u.id
        LEFT JOIN attachments a ON a.id = ua.attachment_id
        WHERE uf.follower_id = $1
        ORDER BY uf.created_at DESC
        LIMIT $2
    "#;

    let results = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            vec![user_id.into(), (limit as i64).into()],
        ))
        .await?;

    Ok(results
        .iter()
        .map(|row| FollowUserDisplay {
            id: row.try_get::<i32>("", "id").unwrap_or(0),
            name: row
                .try_get::<String>("", "name")
                .unwrap_or_else(|_| "Unknown".to_string()),
            avatar_filename: row
                .try_get::<Option<String>>("", "avatar_filename")
                .ok()
                .flatten(),
            custom_title: row
                .try_get::<Option<String>>("", "custom_title")
                .ok()
                .flatten(),
            followed_at: row
                .try_get::<DateTimeWithTimeZone>("", "followed_at")
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
        .collect())
}

/// View a user's followers
#[get("/members/{user_id}/followers")]
pub async fn view_followers(
    client: ClientCtx,
    path: web::Path<(i32,)>,
) -> Result<impl Responder, Error> {
    #[derive(Template)]
    #[template(path = "member_followers.html")]
    pub struct FollowersTemplate {
        pub client: ClientCtx,
        pub user: UserProfile,
        pub followers: Vec<FollowUserDisplay>,
        pub list_type: &'static str,
    }

    let user_id = path.into_inner().0;
    let db = get_db_pool();

    let user = UserProfile::get_by_id(db, user_id)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    let followers = get_followers(db, user_id, 100)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(FollowersTemplate {
        client,
        user,
        followers,
        list_type: "followers",
    }
    .to_response())
}

/// View users that a user follows
#[get("/members/{user_id}/following")]
pub async fn view_following(
    client: ClientCtx,
    path: web::Path<(i32,)>,
) -> Result<impl Responder, Error> {
    #[derive(Template)]
    #[template(path = "member_followers.html")]
    pub struct FollowingTemplate {
        pub client: ClientCtx,
        pub user: UserProfile,
        pub followers: Vec<FollowUserDisplay>,
        pub list_type: &'static str,
    }

    let user_id = path.into_inner().0;
    let db = get_db_pool();

    let user = UserProfile::get_by_id(db, user_id)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    let followers = get_following(db, user_id, 100)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(FollowingTemplate {
        client,
        user,
        followers,
        list_type: "following",
    }
    .to_response())
}
