//! Activity feed routes

use crate::activities::{get_global_feed, get_personal_feed, get_user_feed, ActivityCursor, ActivityDisplay};
use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::user::Profile as UserProfile;
use actix_web::{error, get, web, Error, Responder};
use askama_actix::{Template, TemplateToResponse};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(view_personal_feed)
        .service(view_global_feed)
        .service(view_user_activity);
}

#[derive(serde::Deserialize)]
pub struct FeedQuery {
    pub cursor: Option<String>,
}

#[derive(Template)]
#[template(path = "activity_feed.html")]
pub struct ActivityFeedTemplate {
    pub client: ClientCtx,
    pub activities: Vec<ActivityDisplay>,
    pub next_cursor: Option<String>,
    pub feed_type: FeedType,
    pub profile_user: Option<UserProfile>,
}

#[derive(Debug, Clone, Copy)]
pub enum FeedType {
    Personal,
    Global,
    User,
}

impl FeedType {
    pub fn title(&self) -> &'static str {
        match self {
            FeedType::Personal => "Your Feed",
            FeedType::Global => "Global Activity",
            FeedType::User => "User Activity",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            FeedType::Personal => "Activity from users you follow",
            FeedType::Global => "Recent activity across the site",
            FeedType::User => "Recent activity from this user",
        }
    }
}

/// View personal feed (activities from users you follow)
/// Requires authentication
#[get("/activity")]
async fn view_personal_feed(
    client: ClientCtx,
    query: web::Query<FeedQuery>,
) -> Result<impl Responder, Error> {
    let user_id = client.get_id().ok_or_else(|| {
        error::ErrorUnauthorized("You must be logged in to view your personal feed")
    })?;

    let cursor = query.cursor.as_ref().and_then(|s| ActivityCursor::from_str(s));
    let limit = 25;

    let activities = get_personal_feed(user_id, cursor, limit + 1)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    let (activities, next_cursor) = paginate_activities(activities, limit);

    Ok(ActivityFeedTemplate {
        client,
        activities,
        next_cursor,
        feed_type: FeedType::Personal,
        profile_user: None,
    }
    .to_response())
}

/// View global activity feed (all site activity)
#[get("/activity/global")]
async fn view_global_feed(
    client: ClientCtx,
    query: web::Query<FeedQuery>,
) -> Result<impl Responder, Error> {
    let cursor = query.cursor.as_ref().and_then(|s| ActivityCursor::from_str(s));
    let limit = 25;

    let activities = get_global_feed(cursor, limit + 1)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    let (activities, next_cursor) = paginate_activities(activities, limit);

    Ok(ActivityFeedTemplate {
        client,
        activities,
        next_cursor,
        feed_type: FeedType::Global,
        profile_user: None,
    }
    .to_response())
}

/// View a specific user's activity feed
#[get("/members/{user_id}/activity")]
async fn view_user_activity(
    client: ClientCtx,
    path: web::Path<(i32,)>,
    query: web::Query<FeedQuery>,
) -> Result<impl Responder, Error> {
    let profile_user_id = path.into_inner().0;
    let db = get_db_pool();

    // Get the profile user
    let profile_user = UserProfile::get_by_id(db, profile_user_id)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    let cursor = query.cursor.as_ref().and_then(|s| ActivityCursor::from_str(s));
    let limit = 25;

    let activities = get_user_feed(profile_user_id, cursor, limit + 1)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    let (activities, next_cursor) = paginate_activities(activities, limit);

    Ok(ActivityFeedTemplate {
        client,
        activities,
        next_cursor,
        feed_type: FeedType::User,
        profile_user: Some(profile_user),
    }
    .to_response())
}

/// Helper to paginate activities and generate next cursor
fn paginate_activities(
    mut activities: Vec<ActivityDisplay>,
    limit: u64,
) -> (Vec<ActivityDisplay>, Option<String>) {
    let has_more = activities.len() > limit as usize;
    if has_more {
        activities.truncate(limit as usize);
    }

    let next_cursor = if has_more {
        activities.last().map(|a| ActivityCursor {
            created_at: a.created_at,
            id: a.id,
        }.to_string())
    } else {
        None
    };

    (activities, next_cursor)
}
