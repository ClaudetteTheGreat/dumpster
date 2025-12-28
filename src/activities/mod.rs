//! Activity feed system for tracking user actions

use crate::db::get_db_pool;
use crate::orm::activities::{self, ActivityType};
use chrono::{DateTime, Utc};
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{entity::*, query::*, DbErr, Set};

pub use crate::orm::activities::ActivityType as Type;

// =============================================================================
// Activity Recording Functions
// =============================================================================

/// Record a thread creation activity
pub async fn record_thread_created(
    user_id: i32,
    thread_id: i32,
    forum_id: i32,
    title: &str,
) -> Result<i32, DbErr> {
    let db = get_db_pool();

    let activity = activities::ActiveModel {
        activity_type: Set(ActivityType::ThreadCreated),
        user_id: Set(user_id),
        target_thread_id: Set(Some(thread_id)),
        target_forum_id: Set(Some(forum_id)),
        title: Set(Some(title.to_string())),
        ..Default::default()
    };

    let result = activity.insert(db).await?;
    Ok(result.id)
}

/// Record a post creation activity
pub async fn record_post_created(
    user_id: i32,
    thread_id: i32,
    post_id: i32,
    forum_id: i32,
    thread_title: &str,
    content_preview: &str,
) -> Result<i32, DbErr> {
    let db = get_db_pool();

    // Truncate content preview to 200 chars
    let preview = if content_preview.len() > 200 {
        format!("{}...", &content_preview[..197])
    } else {
        content_preview.to_string()
    };

    let activity = activities::ActiveModel {
        activity_type: Set(ActivityType::PostCreated),
        user_id: Set(user_id),
        target_thread_id: Set(Some(thread_id)),
        target_post_id: Set(Some(post_id)),
        target_forum_id: Set(Some(forum_id)),
        title: Set(Some(thread_title.to_string())),
        content_preview: Set(Some(preview)),
        ..Default::default()
    };

    let result = activity.insert(db).await?;
    Ok(result.id)
}

/// Record a user follow activity
pub async fn record_user_followed(
    follower_id: i32,
    following_id: i32,
    following_name: &str,
) -> Result<i32, DbErr> {
    let db = get_db_pool();

    let activity = activities::ActiveModel {
        activity_type: Set(ActivityType::UserFollowed),
        user_id: Set(follower_id),
        target_user_id: Set(Some(following_id)),
        title: Set(Some(following_name.to_string())),
        ..Default::default()
    };

    let result = activity.insert(db).await?;
    Ok(result.id)
}

/// Record a profile post activity
pub async fn record_profile_post_created(
    author_id: i32,
    profile_user_id: i32,
    profile_user_name: &str,
    content_preview: &str,
) -> Result<i32, DbErr> {
    let db = get_db_pool();

    // Truncate content preview to 200 chars
    let preview = if content_preview.len() > 200 {
        format!("{}...", &content_preview[..197])
    } else {
        content_preview.to_string()
    };

    let activity = activities::ActiveModel {
        activity_type: Set(ActivityType::ProfilePostCreated),
        user_id: Set(author_id),
        target_user_id: Set(Some(profile_user_id)),
        title: Set(Some(profile_user_name.to_string())),
        content_preview: Set(Some(preview)),
        ..Default::default()
    };

    let result = activity.insert(db).await?;
    Ok(result.id)
}

/// Record a reaction activity
pub async fn record_reaction_given(
    user_id: i32,
    target_post_id: i32,
    thread_id: i32,
    forum_id: i32,
    reaction_emoji: &str,
    thread_title: &str,
) -> Result<i32, DbErr> {
    let db = get_db_pool();

    let activity = activities::ActiveModel {
        activity_type: Set(ActivityType::ReactionGiven),
        user_id: Set(user_id),
        target_post_id: Set(Some(target_post_id)),
        target_thread_id: Set(Some(thread_id)),
        target_forum_id: Set(Some(forum_id)),
        reaction_emoji: Set(Some(reaction_emoji.to_string())),
        title: Set(Some(thread_title.to_string())),
        ..Default::default()
    };

    let result = activity.insert(db).await?;
    Ok(result.id)
}

// =============================================================================
// Activity Display Structures
// =============================================================================

/// Display model for activity feed items
#[derive(Debug, Clone)]
pub struct ActivityDisplay {
    pub id: i32,
    pub activity_type: ActivityType,
    pub created_at: DateTime<Utc>,
    pub actor_id: i32,
    pub actor_name: String,
    pub actor_avatar: Option<String>,
    pub title: Option<String>,
    pub content_preview: Option<String>,
    pub target_url: String,
    pub reaction_emoji: Option<String>,
}

/// Pagination cursor for activity feeds
#[derive(Debug, Clone)]
pub struct ActivityCursor {
    pub created_at: DateTime<Utc>,
    pub id: i32,
}

impl ActivityCursor {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('_').collect();
        if parts.len() != 2 {
            return None;
        }
        let timestamp = parts[0].parse::<i64>().ok()?;
        let id = parts[1].parse::<i32>().ok()?;
        Some(Self {
            created_at: DateTime::from_timestamp(timestamp, 0)?,
            id,
        })
    }
}

impl std::fmt::Display for ActivityCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.created_at.timestamp(), self.id)
    }
}

// =============================================================================
// Activity Query Functions
// =============================================================================

/// Get personal feed (activities from users you follow)
pub async fn get_personal_feed(
    user_id: i32,
    cursor: Option<ActivityCursor>,
    limit: u64,
) -> Result<Vec<ActivityDisplay>, DbErr> {
    use sea_orm::{DbBackend, Statement};

    let db = get_db_pool();

    let (cursor_clause, values) = match &cursor {
        Some(c) => (
            "AND (a.created_at, a.id) < ($3, $4)",
            vec![
                user_id.into(),
                (limit as i64).into(),
                c.created_at.into(),
                c.id.into(),
            ],
        ),
        None => ("", vec![user_id.into(), (limit as i64).into()]),
    };

    let sql = format!(
        r#"
        SELECT
            a.id,
            a.activity_type::text,
            a.created_at,
            a.user_id as actor_id,
            un.name as actor_name,
            att.filename as actor_avatar,
            a.title,
            a.content_preview,
            a.target_thread_id,
            a.target_post_id,
            a.target_user_id,
            a.reaction_emoji
        FROM activities a
        JOIN user_follows uf ON a.user_id = uf.following_id
        LEFT JOIN user_names un ON un.user_id = a.user_id
        LEFT JOIN user_avatars ua ON ua.user_id = a.user_id
        LEFT JOIN attachments att ON att.id = ua.attachment_id
        WHERE uf.follower_id = $1
        {}
        ORDER BY a.created_at DESC, a.id DESC
        LIMIT $2
        "#,
        cursor_clause
    );

    let results = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            values,
        ))
        .await?;

    Ok(results.iter().map(parse_activity_row).collect())
}

/// Get user profile feed (specific user's activities)
pub async fn get_user_feed(
    profile_user_id: i32,
    cursor: Option<ActivityCursor>,
    limit: u64,
) -> Result<Vec<ActivityDisplay>, DbErr> {
    use sea_orm::{DbBackend, Statement};

    let db = get_db_pool();

    let (cursor_clause, values) = match &cursor {
        Some(c) => (
            "AND (a.created_at, a.id) < ($3, $4)",
            vec![
                profile_user_id.into(),
                (limit as i64).into(),
                c.created_at.into(),
                c.id.into(),
            ],
        ),
        None => ("", vec![profile_user_id.into(), (limit as i64).into()]),
    };

    let sql = format!(
        r#"
        SELECT
            a.id,
            a.activity_type::text,
            a.created_at,
            a.user_id as actor_id,
            un.name as actor_name,
            att.filename as actor_avatar,
            a.title,
            a.content_preview,
            a.target_thread_id,
            a.target_post_id,
            a.target_user_id,
            a.reaction_emoji
        FROM activities a
        LEFT JOIN user_names un ON un.user_id = a.user_id
        LEFT JOIN user_avatars ua ON ua.user_id = a.user_id
        LEFT JOIN attachments att ON att.id = ua.attachment_id
        WHERE a.user_id = $1
        {}
        ORDER BY a.created_at DESC, a.id DESC
        LIMIT $2
        "#,
        cursor_clause
    );

    let results = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            values,
        ))
        .await?;

    Ok(results.iter().map(parse_activity_row).collect())
}

/// Get global feed (all site activity, respecting privacy)
pub async fn get_global_feed(
    cursor: Option<ActivityCursor>,
    limit: u64,
) -> Result<Vec<ActivityDisplay>, DbErr> {
    use sea_orm::{DbBackend, Statement};

    let db = get_db_pool();

    let (cursor_clause, values) = match &cursor {
        Some(c) => (
            "AND (a.created_at, a.id) < ($2, $3)",
            vec![(limit as i64).into(), c.created_at.into(), c.id.into()],
        ),
        None => ("", vec![(limit as i64).into()]),
    };

    let sql = format!(
        r#"
        SELECT
            a.id,
            a.activity_type::text,
            a.created_at,
            a.user_id as actor_id,
            un.name as actor_name,
            att.filename as actor_avatar,
            a.title,
            a.content_preview,
            a.target_thread_id,
            a.target_post_id,
            a.target_user_id,
            a.reaction_emoji
        FROM activities a
        JOIN users u ON a.user_id = u.id
        LEFT JOIN user_names un ON un.user_id = a.user_id
        LEFT JOIN user_avatars ua ON ua.user_id = a.user_id
        LEFT JOIN attachments att ON att.id = ua.attachment_id
        WHERE u.show_online = TRUE
        {}
        ORDER BY a.created_at DESC, a.id DESC
        LIMIT $1
        "#,
        cursor_clause
    );

    let results = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            values,
        ))
        .await?;

    Ok(results.iter().map(parse_activity_row).collect())
}

/// Parse a database row into an ActivityDisplay
fn parse_activity_row(row: &sea_orm::QueryResult) -> ActivityDisplay {
    let activity_type_str: String = row
        .try_get::<String>("", "activity_type")
        .unwrap_or_else(|_| "post_created".to_string());

    let activity_type = match activity_type_str.as_str() {
        "thread_created" => ActivityType::ThreadCreated,
        "profile_post_created" => ActivityType::ProfilePostCreated,
        "user_followed" => ActivityType::UserFollowed,
        "reaction_given" => ActivityType::ReactionGiven,
        _ => ActivityType::PostCreated,
    };

    let id: i32 = row.try_get("", "id").unwrap_or(0);
    let target_thread_id: Option<i32> = row.try_get("", "target_thread_id").ok();
    let target_post_id: Option<i32> = row.try_get("", "target_post_id").ok();
    let target_user_id: Option<i32> = row.try_get("", "target_user_id").ok();

    // Build target URL based on activity type
    let target_url = match activity_type {
        ActivityType::ThreadCreated => {
            format!("/threads/{}/", target_thread_id.unwrap_or(0))
        }
        ActivityType::PostCreated => {
            if let (Some(tid), Some(pid)) = (target_thread_id, target_post_id) {
                format!("/threads/{}/#post-{}", tid, pid)
            } else {
                "/".to_string()
            }
        }
        ActivityType::ProfilePostCreated | ActivityType::UserFollowed => {
            format!("/members/{}/", target_user_id.unwrap_or(0))
        }
        ActivityType::ReactionGiven => {
            if let (Some(tid), Some(pid)) = (target_thread_id, target_post_id) {
                format!("/threads/{}/#post-{}", tid, pid)
            } else {
                "/".to_string()
            }
        }
    };

    ActivityDisplay {
        id,
        activity_type,
        created_at: row
            .try_get::<DateTimeWithTimeZone>("", "created_at")
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        actor_id: row.try_get("", "actor_id").unwrap_or(0),
        actor_name: row
            .try_get::<String>("", "actor_name")
            .unwrap_or_else(|_| "Unknown".to_string()),
        actor_avatar: row
            .try_get::<Option<String>>("", "actor_avatar")
            .ok()
            .flatten(),
        title: row.try_get::<Option<String>>("", "title").ok().flatten(),
        content_preview: row
            .try_get::<Option<String>>("", "content_preview")
            .ok()
            .flatten(),
        target_url,
        reaction_emoji: row
            .try_get::<Option<String>>("", "reaction_emoji")
            .ok()
            .flatten(),
    }
}
