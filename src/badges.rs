//! Badge system for awarding achievements to users

use crate::db::get_db_pool;
use crate::orm::user_badges;
use chrono::Utc;
use sea_orm::{entity::*, query::*, DatabaseConnection, DbErr, FromQueryResult};

/// Badge information with user award status
#[derive(Clone, Debug, FromQueryResult)]
pub struct BadgeInfo {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub icon: String,
    pub color: Option<String>,
    pub condition_type: String,
    pub condition_value: Option<i32>,
    pub display_order: i32,
}

/// User badge for display
#[derive(Clone, Debug)]
pub struct UserBadge {
    pub badge: BadgeInfo,
    pub awarded_at: chrono::DateTime<Utc>,
    pub awarded_by: Option<i32>,
}

/// User statistics for badge condition checking
#[derive(Debug, FromQueryResult)]
pub struct UserStats {
    pub post_count: i64,
    pub thread_count: i64,
    pub reputation_score: i32,
    pub days_member: i64,
}

/// Get all active badges
pub async fn get_all_badges(db: &DatabaseConnection) -> Result<Vec<BadgeInfo>, DbErr> {
    BadgeInfo::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        r#"
        SELECT id, name, slug, description, icon, color,
               condition_type::text as condition_type, condition_value, display_order
        FROM badges
        WHERE is_active = true
        ORDER BY display_order
        "#,
        vec![],
    ))
    .all(db)
    .await
}

/// Get a badge by slug
pub async fn get_badge_by_slug(db: &DatabaseConnection, slug: &str) -> Result<Option<BadgeInfo>, DbErr> {
    BadgeInfo::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        r#"
        SELECT id, name, slug, description, icon, color,
               condition_type::text as condition_type, condition_value, display_order
        FROM badges
        WHERE slug = $1 AND is_active = true
        "#,
        vec![slug.into()],
    ))
    .one(db)
    .await
}

/// Get a badge by id
pub async fn get_badge_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<BadgeInfo>, DbErr> {
    BadgeInfo::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        r#"
        SELECT id, name, slug, description, icon, color,
               condition_type::text as condition_type, condition_value, display_order
        FROM badges
        WHERE id = $1 AND is_active = true
        "#,
        vec![id.into()],
    ))
    .one(db)
    .await
}

/// Get all badges for a user
pub async fn get_user_badges(db: &DatabaseConnection, user_id: i32) -> Result<Vec<UserBadge>, DbErr> {
    #[derive(Debug, FromQueryResult)]
    struct UserBadgeRow {
        id: i32,
        name: String,
        slug: String,
        description: Option<String>,
        icon: String,
        color: Option<String>,
        condition_type: String,
        condition_value: Option<i32>,
        display_order: i32,
        awarded_at: chrono::DateTime<Utc>,
        awarded_by: Option<i32>,
    }

    let rows = UserBadgeRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        r#"
        SELECT b.id, b.name, b.slug, b.description, b.icon, b.color,
               b.condition_type::text as condition_type, b.condition_value, b.display_order,
               ub.awarded_at, ub.awarded_by
        FROM user_badges ub
        JOIN badges b ON b.id = ub.badge_id
        WHERE ub.user_id = $1 AND b.is_active = true
        ORDER BY b.display_order
        "#,
        vec![user_id.into()],
    ))
    .all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| UserBadge {
            badge: BadgeInfo {
                id: row.id,
                name: row.name,
                slug: row.slug,
                description: row.description,
                icon: row.icon,
                color: row.color,
                condition_type: row.condition_type,
                condition_value: row.condition_value,
                display_order: row.display_order,
            },
            awarded_at: row.awarded_at,
            awarded_by: row.awarded_by,
        })
        .collect())
}

/// Check if a user has a specific badge
pub async fn user_has_badge(db: &DatabaseConnection, user_id: i32, badge_id: i32) -> Result<bool, DbErr> {
    let result = user_badges::Entity::find()
        .filter(user_badges::Column::UserId.eq(user_id))
        .filter(user_badges::Column::BadgeId.eq(badge_id))
        .one(db)
        .await?;
    Ok(result.is_some())
}

/// Award a badge to a user (manual award)
pub async fn award_badge(
    db: &DatabaseConnection,
    user_id: i32,
    badge_id: i32,
    awarded_by: Option<i32>,
) -> Result<bool, DbErr> {
    // Check if already has badge
    if user_has_badge(db, user_id, badge_id).await? {
        return Ok(false);
    }

    let now = Utc::now();
    let user_badge = user_badges::ActiveModel {
        user_id: Set(user_id),
        badge_id: Set(badge_id),
        awarded_at: Set(now.into()),
        awarded_by: Set(awarded_by),
    };

    match user_badge.insert(db).await {
        Ok(_) => {
            log::info!(
                "Awarded badge {} to user {} (by {:?})",
                badge_id,
                user_id,
                awarded_by
            );
            Ok(true)
        }
        Err(e) => {
            // Log the error but return false (assume duplicate key)
            log::debug!(
                "Could not award badge {} to user {} (likely already has it): {}",
                badge_id,
                user_id,
                e
            );
            Ok(false)
        }
    }
}

/// Revoke a badge from a user
pub async fn revoke_badge(db: &DatabaseConnection, user_id: i32, badge_id: i32) -> Result<bool, DbErr> {
    let result = user_badges::Entity::delete_many()
        .filter(user_badges::Column::UserId.eq(user_id))
        .filter(user_badges::Column::BadgeId.eq(badge_id))
        .exec(db)
        .await?;

    if result.rows_affected > 0 {
        log::info!("Revoked badge {} from user {}", badge_id, user_id);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Get user statistics for badge condition checking
async fn get_user_stats(db: &DatabaseConnection, user_id: i32) -> Result<Option<UserStats>, DbErr> {
    UserStats::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        r#"
        SELECT
            (SELECT COUNT(*) FROM posts WHERE user_id = $1) as post_count,
            (SELECT COUNT(*) FROM threads WHERE user_id = $1) as thread_count,
            u.reputation_score,
            EXTRACT(DAY FROM (NOW() - u.created_at))::bigint as days_member
        FROM users u
        WHERE u.id = $1
        "#,
        vec![user_id.into()],
    ))
    .one(db)
    .await
}

/// Check and award automatic badges based on user milestones.
/// This should be called after actions that might trigger badge awards
/// (post creation, thread creation, reputation changes, login).
pub async fn check_and_award_automatic_badges(user_id: i32) {
    let db = get_db_pool();

    let stats = match get_user_stats(db, user_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            log::warn!("User {} not found for badge check", user_id);
            return;
        }
        Err(e) => {
            log::error!("Failed to get user stats for badge check: {}", e);
            return;
        }
    };

    // Get all active automatic badges
    let badges = match get_automatic_badges(db).await {
        Ok(b) => b,
        Err(e) => {
            log::error!("Failed to get automatic badges: {}", e);
            return;
        }
    };

    for badge in badges {
        // Check if condition is met
        let condition_met = match badge.condition_type.as_str() {
            "post_count" => {
                badge.condition_value.map_or(false, |v| stats.post_count >= v as i64)
            }
            "thread_count" => {
                badge.condition_value.map_or(false, |v| stats.thread_count >= v as i64)
            }
            "time_member" => {
                badge.condition_value.map_or(false, |v| stats.days_member >= v as i64)
            }
            "reputation" => {
                badge.condition_value.map_or(false, |v| stats.reputation_score >= v)
            }
            _ => false, // Skip manual badges
        };

        if condition_met {
            // Try to award (will skip if already has it)
            if let Err(e) = award_badge(db, user_id, badge.id, None).await {
                log::error!(
                    "Failed to award badge {} to user {}: {}",
                    badge.slug,
                    user_id,
                    e
                );
            }
        }
    }
}

/// Get all automatic (non-manual) badges
async fn get_automatic_badges(db: &DatabaseConnection) -> Result<Vec<BadgeInfo>, DbErr> {
    BadgeInfo::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        r#"
        SELECT id, name, slug, description, icon, color,
               condition_type::text as condition_type, condition_value, display_order
        FROM badges
        WHERE is_active = true AND condition_type != 'manual'
        ORDER BY display_order
        "#,
        vec![],
    ))
    .all(db)
    .await
}

/// Count badges for a user
pub async fn count_user_badges(db: &DatabaseConnection, user_id: i32) -> Result<i64, DbErr> {
    #[derive(FromQueryResult)]
    struct CountResult {
        count: i64,
    }

    let result = CountResult::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT COUNT(*) as count FROM user_badges ub JOIN badges b ON b.id = ub.badge_id WHERE ub.user_id = $1 AND b.is_active = true",
        vec![user_id.into()],
    ))
    .one(db)
    .await?;

    Ok(result.map(|r| r.count).unwrap_or(0))
}
