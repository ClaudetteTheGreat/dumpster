use crate::attachment::AttachmentSize;
use crate::db::get_db_pool;
use crate::orm::{attachments, user_avatars, user_names, users};
use crate::url::UrlToken;
use chrono::{DateTime, Duration, Utc};
use once_cell::sync::Lazy;
use sea_orm::{entity::*, query::*, DatabaseConnection, FromQueryResult};
use std::collections::HashMap;
use std::sync::RwLock;

/// Users are considered "online" if they were active within this many minutes
pub const ONLINE_THRESHOLD_MINUTES: i64 = 15;

/// Minimum seconds between activity updates for the same user (rate limiting)
const ACTIVITY_UPDATE_INTERVAL_SECS: i64 = 60;

/// Cache of last activity update times to avoid database spam
static ACTIVITY_UPDATE_CACHE: Lazy<RwLock<HashMap<i32, DateTime<Utc>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Base URL fragment for resource.
pub static RESOURCE_URL: &str = "members";

/// Joins user data (name, avatar, post_count) to a query.
/// Optimized: uses denormalized post_count column instead of correlated subquery.
pub fn find_also_user<E, C>(sel: Select<E>, col: C) -> SelectTwo<E, users::Entity>
where
    E: EntityTrait<Column = C>,
    C: IntoSimpleExpr + ColumnTrait,
{
    sel.select_also(users::Entity)
        .join(
            JoinType::LeftJoin,
            E::belongs_to(users::Entity)
                .from(col)
                .to(users::Column::Id)
                .into(),
        )
        .join(JoinType::LeftJoin, users::Relation::UserName.def())
        .column_as(user_names::Column::Name, "B_name")
        .join(JoinType::LeftJoin, users::Relation::UserAvatar.def())
        .join(
            JoinType::LeftJoin,
            user_avatars::Relation::Attachments.def(),
        )
        .column_as(attachments::Column::Filename, "B_avatar_filename")
        .column_as(attachments::Column::FileHeight, "B_avatar_height")
        .column_as(attachments::Column::FileWidth, "B_avatar_width")
        // Use denormalized post_count column instead of subquery
        .column_as(users::Column::PostCount, "B_post_count")
}

/// A struct to hold all information for a user, including relational information.
#[derive(Clone, Debug, FromQueryResult)]
pub struct Profile {
    pub id: i32,
    pub name: String,
    pub created_at: chrono::NaiveDateTime,
    pub password_cipher: String,
    pub avatar_filename: Option<String>,
    pub avatar_height: Option<i32>,
    pub avatar_width: Option<i32>,
    pub posts_per_page: i32,
    /// Denormalized post count from users table (no longer computed via COUNT)
    pub post_count: i32,
    pub theme: Option<String>,
    pub theme_auto: bool,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub website_url: Option<String>,
    pub signature: Option<String>,
    pub custom_title: Option<String>,
    pub show_online: bool,
    pub reputation_score: i32,
    pub allow_profile_posts: bool,
    pub follower_count: i32,
    pub following_count: i32,
    pub default_chat_room: Option<i32>,
}

impl Profile {
    /// Returns a fully qualified user profile by id.
    /// Optimized: uses denormalized post_count column instead of COUNT(*).
    pub async fn get_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<Self>, sea_orm::DbErr> {
        use sea_orm::{DbBackend, Statement};

        // Optimized query using denormalized post_count column
        // No longer needs to join posts table or use GROUP BY
        let sql = r#"
            SELECT
                u.id,
                un.name,
                u.created_at,
                u.password_cipher::text as password_cipher,
                a.filename as avatar_filename,
                a.file_height as avatar_height,
                a.file_width as avatar_width,
                u.posts_per_page,
                u.post_count,
                u.theme,
                u.theme_auto,
                u.bio,
                u.location,
                u.website_url,
                u.signature,
                u.custom_title,
                u.show_online,
                u.reputation_score,
                u.allow_profile_posts,
                u.follower_count,
                u.following_count,
                u.default_chat_room
            FROM users u
            LEFT JOIN user_names un ON un.user_id = u.id
            LEFT JOIN user_avatars ua ON ua.user_id = u.id
            LEFT JOIN attachments a ON a.id = ua.attachment_id
            WHERE u.id = $1
        "#;

        Self::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            vec![id.into()],
        ))
        .one(db)
        .await
    }

    /// Returns multiple user profiles by IDs in a single batch query.
    /// Used for efficient loading of post authors without N+1 queries.
    pub async fn get_by_ids(
        db: &DatabaseConnection,
        ids: &[i32],
    ) -> Result<HashMap<i32, Self>, sea_orm::DbErr> {
        use sea_orm::{DbBackend, Statement};

        if ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Build placeholders for IN clause: $1, $2, $3, ...
        let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("${}", i)).collect();
        let placeholders_str = placeholders.join(", ");

        let sql = format!(
            r#"
            SELECT
                u.id,
                un.name,
                u.created_at,
                u.password_cipher::text as password_cipher,
                a.filename as avatar_filename,
                a.file_height as avatar_height,
                a.file_width as avatar_width,
                u.posts_per_page,
                u.post_count,
                u.theme,
                u.theme_auto,
                u.bio,
                u.location,
                u.website_url,
                u.signature,
                u.custom_title,
                u.show_online,
                u.reputation_score,
                u.allow_profile_posts,
                u.follower_count,
                u.following_count,
                u.default_chat_room
            FROM users u
            LEFT JOIN user_names un ON un.user_id = u.id
            LEFT JOIN user_avatars ua ON ua.user_id = u.id
            LEFT JOIN attachments a ON a.id = ua.attachment_id
            WHERE u.id IN ({})
        "#,
            placeholders_str
        );

        let values: Vec<sea_orm::Value> = ids.iter().map(|&id| id.into()).collect();

        let profiles = Self::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            values,
        ))
        .all(db)
        .await?;

        Ok(profiles.into_iter().map(|p| (p.id, p)).collect())
    }

    /// Provides semantically correct HTML for an avatar.
    pub fn get_avatar_html(&self, size: AttachmentSize) -> String {
        if let (Some(filename), Some(width), Some(height)) = (
            self.avatar_filename.as_ref(),
            self.avatar_width,
            self.avatar_width,
        ) {
            crate::attachment::get_avatar_html(filename, (width, height), size)
        } else {
            "".to_owned()
        }
    }

    /// Provides a URL token for this resource.
    pub fn get_url_token(&self) -> UrlToken<'static> {
        UrlToken {
            id: Some(self.id),
            name: self.name.to_owned(),
            base_url: RESOURCE_URL,
            class: "username",
        }
    }

    /// Renders the user's signature as HTML using BBCode parser.
    pub fn get_signature_html(&self) -> Option<String> {
        self.signature
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|sig| crate::bbcode::parse(sig))
    }
}

pub async fn get_user_id_from_name(db: &DatabaseConnection, name: &str) -> Option<i32> {
    user_names::Entity::find()
        .filter(user_names::Column::Name.eq(name))
        .one(db)
        .await
        .unwrap_or(None)
        .map(|user_name| user_name.user_id)
}

/// Update user's last activity timestamp.
/// This is rate-limited to avoid database spam - only updates if enough time has passed.
pub async fn update_last_activity(user_id: i32) {
    let now = Utc::now();

    // Check if we should update (rate limiting)
    {
        let cache = ACTIVITY_UPDATE_CACHE.read().unwrap();
        if let Some(last_update) = cache.get(&user_id) {
            if now.signed_duration_since(*last_update)
                < Duration::seconds(ACTIVITY_UPDATE_INTERVAL_SECS)
            {
                return; // Too soon, skip update
            }
        }
    }

    // Update the cache
    {
        let mut cache = ACTIVITY_UPDATE_CACHE.write().unwrap();
        cache.insert(user_id, now);
    }

    // Update the database asynchronously
    let db = get_db_pool();
    if let Err(e) = users::Entity::update_many()
        .col_expr(
            users::Column::LastActivityAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(users::Column::Id.eq(user_id))
        .exec(db)
        .await
    {
        log::warn!("Failed to update last activity for user {}: {}", user_id, e);
    }
}

/// Check if a user is considered "online" based on their last activity
pub fn is_user_online(last_activity: Option<DateTime<Utc>>) -> bool {
    match last_activity {
        Some(activity) => {
            let threshold = Utc::now() - Duration::minutes(ONLINE_THRESHOLD_MINUTES);
            activity > threshold
        }
        None => false,
    }
}

/// Get the count of currently online users
pub async fn count_online_users() -> Result<i64, sea_orm::DbErr> {
    let db = get_db_pool();
    let threshold = Utc::now() - Duration::minutes(ONLINE_THRESHOLD_MINUTES);

    #[derive(FromQueryResult)]
    struct CountResult {
        count: i64,
    }

    let result = CountResult::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT COUNT(*) as count FROM users WHERE last_activity_at > $1 AND show_online = true",
        vec![threshold.into()],
    ))
    .one(db)
    .await?;

    Ok(result.map(|r| r.count).unwrap_or(0))
}

/// Get list of online users (respecting privacy settings)
pub async fn get_online_users(limit: u64) -> Result<Vec<OnlineUser>, sea_orm::DbErr> {
    let db = get_db_pool();
    let threshold = Utc::now() - Duration::minutes(ONLINE_THRESHOLD_MINUTES);

    let users = OnlineUser::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        r#"
        SELECT u.id, un.name, u.last_activity_at
        FROM users u
        LEFT JOIN user_names un ON un.user_id = u.id
        WHERE u.last_activity_at > $1
          AND u.show_online = true
        ORDER BY u.last_activity_at DESC
        LIMIT $2
        "#,
        vec![threshold.into(), (limit as i64).into()],
    ))
    .all(db)
    .await?;

    Ok(users)
}

/// Simple struct for online user display
#[derive(Clone, Debug, FromQueryResult)]
pub struct OnlineUser {
    pub id: i32,
    pub name: String,
    pub last_activity_at: Option<DateTime<Utc>>,
}

/// Cleanup old entries from the activity update cache
/// Should be called periodically to prevent memory growth
pub fn cleanup_activity_cache() {
    let threshold = Utc::now() - Duration::minutes(ONLINE_THRESHOLD_MINUTES * 2);
    let mut cache = ACTIVITY_UPDATE_CACHE.write().unwrap();
    cache.retain(|_, v| *v > threshold);
}
