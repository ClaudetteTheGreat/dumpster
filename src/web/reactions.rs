//! Post reaction endpoints

use crate::config::Config;
use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{posts, reaction_types, threads, ugc_reactions};
use actix_web::{error, get, post, web, Error, HttpResponse};
use chrono::Utc;
use sea_orm::{entity::*, query::*, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(toggle_reaction)
        .service(get_reactions)
        .service(get_reaction_types);
}

/// Response for reaction toggle
#[derive(Serialize)]
struct ToggleReactionResponse {
    success: bool,
    added: bool,
    reaction_count: i32,
    user_reactions: Vec<i32>,
}

/// Response for getting reactions on a post
#[derive(Serialize)]
struct ReactionsResponse {
    reactions: Vec<ReactionSummary>,
    user_reactions: Vec<i32>,
}

#[derive(Serialize)]
struct ReactionSummary {
    reaction_type_id: i32,
    name: String,
    emoji: String,
    count: i64,
}

#[derive(Serialize)]
struct ReactionTypeInfo {
    id: i32,
    name: String,
    emoji: String,
    is_positive: bool,
}

/// Toggle a reaction on a UGC item (add if not present, remove if present)
#[post("/reactions/{ugc_id}/{reaction_type_id}")]
async fn toggle_reaction(
    client: ClientCtx,
    session: actix_session::Session,
    path: web::Path<(i32, i32)>,
    form: web::Form<CsrfForm>,
    config: web::Data<Arc<Config>>,
) -> Result<HttpResponse, Error> {
    let user_id = client
        .get_id()
        .ok_or_else(|| error::ErrorUnauthorized("Must be logged in to react"))?;

    // Validate CSRF
    crate::middleware::csrf::validate_csrf_token(&session, &form.csrf_token)?;

    let (ugc_id, reaction_type_id) = path.into_inner();
    let db = get_db_pool();

    // Verify reaction type exists and is active
    let reaction_type = reaction_types::Entity::find_by_id(reaction_type_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Reaction type not found"))?;

    if !reaction_type.is_active {
        return Err(error::ErrorBadRequest(
            "This reaction type is not available",
        ));
    }

    // Check if this UGC belongs to a post and if the user is the post author
    let post = posts::Entity::find()
        .filter(posts::Column::UgcId.eq(ugc_id))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    if let Some(ref post) = post {
        // Cannot react to own posts
        if post.user_id == Some(user_id) {
            return Err(error::ErrorForbidden("Cannot react to your own posts"));
        }
    }

    // Check minimum post count requirement
    let min_posts: i64 = config.get_int_or("min_posts_to_vote", 5);

    // Get user's post count
    let user_post_count: i64 = posts::Entity::find()
        .filter(posts::Column::UserId.eq(user_id))
        .count(db)
        .await
        .map_err(error::ErrorInternalServerError)? as i64;

    if user_post_count < min_posts {
        return Err(error::ErrorForbidden(format!(
            "You need at least {} posts to give reactions (you have {})",
            min_posts, user_post_count
        )));
    }

    // Check if user already has this reaction
    let existing = ugc_reactions::Entity::find()
        .filter(ugc_reactions::Column::UgcId.eq(ugc_id))
        .filter(ugc_reactions::Column::UserId.eq(user_id))
        .filter(ugc_reactions::Column::ReactionTypeId.eq(reaction_type_id))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let added = if let Some(existing_reaction) = existing {
        // Remove reaction
        ugc_reactions::Entity::delete_by_id(existing_reaction.id)
            .exec(db)
            .await
            .map_err(error::ErrorInternalServerError)?;
        false
    } else {
        // Add reaction
        let new_reaction = ugc_reactions::ActiveModel {
            ugc_id: Set(ugc_id),
            user_id: Set(user_id),
            reaction_type_id: Set(reaction_type_id),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        };
        new_reaction
            .insert(db)
            .await
            .map_err(error::ErrorInternalServerError)?;

        // Record activity for the feed (async, non-blocking)
        if let Some(ref p) = post {
            let post_id = p.id;
            let thread_id = p.thread_id;
            let emoji = reaction_type.emoji.clone();

            actix::spawn(async move {
                // Get thread info for activity
                let db = get_db_pool();
                if let Ok(Some(thread)) = threads::Entity::find_by_id(thread_id).one(db).await {
                    if let Err(e) = crate::activities::record_reaction_given(
                        user_id,
                        post_id,
                        thread_id,
                        thread.forum_id,
                        &emoji,
                        &thread.title,
                    )
                    .await
                    {
                        log::warn!("Failed to record reaction activity: {}", e);
                    }
                }
            });
        }

        true
    };

    // Get updated reaction count from ugc table
    let ugc = crate::orm::ugc::Entity::find_by_id(ugc_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Content not found"))?;

    // Get user's current reactions on this content
    let user_reactions: Vec<i32> = ugc_reactions::Entity::find()
        .filter(ugc_reactions::Column::UgcId.eq(ugc_id))
        .filter(ugc_reactions::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .iter()
        .map(|r| r.reaction_type_id)
        .collect();

    Ok(HttpResponse::Ok().json(ToggleReactionResponse {
        success: true,
        added,
        reaction_count: ugc.reaction_count,
        user_reactions,
    }))
}

#[derive(Deserialize)]
struct CsrfForm {
    csrf_token: String,
}

/// Get reactions for a UGC item
#[get("/reactions/{ugc_id}")]
async fn get_reactions(client: ClientCtx, path: web::Path<i32>) -> Result<HttpResponse, Error> {
    let ugc_id = path.into_inner();
    let db = get_db_pool();

    // Get reaction counts grouped by type
    let reactions = ugc_reactions::Entity::find()
        .filter(ugc_reactions::Column::UgcId.eq(ugc_id))
        .find_also_related(reaction_types::Entity)
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Count reactions by type
    let mut reaction_counts: std::collections::HashMap<i32, (String, String, i64)> =
        std::collections::HashMap::new();
    for (reaction, reaction_type) in &reactions {
        if let Some(rt) = reaction_type {
            let entry = reaction_counts.entry(reaction.reaction_type_id).or_insert((
                rt.name.clone(),
                rt.emoji.clone(),
                0,
            ));
            entry.2 += 1;
        }
    }

    let summaries: Vec<ReactionSummary> = reaction_counts
        .into_iter()
        .map(|(id, (name, emoji, count))| ReactionSummary {
            reaction_type_id: id,
            name,
            emoji,
            count,
        })
        .collect();

    // Get user's reactions if logged in
    let user_reactions = if let Some(user_id) = client.get_id() {
        reactions
            .iter()
            .filter(|(r, _)| r.user_id == user_id)
            .map(|(r, _)| r.reaction_type_id)
            .collect()
    } else {
        vec![]
    };

    Ok(HttpResponse::Ok().json(ReactionsResponse {
        reactions: summaries,
        user_reactions,
    }))
}

/// Get all available reaction types
#[get("/reactions/types")]
async fn get_reaction_types() -> Result<HttpResponse, Error> {
    let db = get_db_pool();

    let types = reaction_types::Entity::find()
        .filter(reaction_types::Column::IsActive.eq(true))
        .order_by_asc(reaction_types::Column::DisplayOrder)
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let response: Vec<ReactionTypeInfo> = types
        .into_iter()
        .map(|t| ReactionTypeInfo {
            id: t.id,
            name: t.name,
            emoji: t.emoji,
            is_positive: t.is_positive,
        })
        .collect();

    Ok(HttpResponse::Ok().json(response))
}

/// Type alias for reaction summary: (reaction_type_id, name, emoji, count)
pub type ReactionSummaryTuple = (i32, String, String, i64);
/// Type alias for reactions data: (summaries, user_reaction_type_ids)
pub type ReactionsData = (Vec<ReactionSummaryTuple>, Vec<i32>);

/// Helper to get reaction summary for a list of UGC IDs (for templates)
#[allow(dead_code)]
pub async fn get_reactions_for_ugc_ids(
    ugc_ids: &[i32],
    user_id: Option<i32>,
) -> Result<std::collections::HashMap<i32, ReactionsData>, sea_orm::DbErr> {
    let db = get_db_pool();

    if ugc_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    // Get all reactions for these UGC IDs
    let reactions = ugc_reactions::Entity::find()
        .filter(ugc_reactions::Column::UgcId.is_in(ugc_ids.to_vec()))
        .find_also_related(reaction_types::Entity)
        .all(db)
        .await?;

    // Group by UGC ID
    let mut result: std::collections::HashMap<i32, ReactionsData> =
        std::collections::HashMap::new();

    // Initialize empty entries for all requested IDs
    for &id in ugc_ids {
        result.insert(id, (vec![], vec![]));
    }

    // Count reactions by type for each UGC
    let mut counts: std::collections::HashMap<(i32, i32), (String, String, i64)> =
        std::collections::HashMap::new();

    for (reaction, reaction_type) in &reactions {
        if let Some(rt) = reaction_type {
            let key = (reaction.ugc_id, reaction.reaction_type_id);
            let entry = counts
                .entry(key)
                .or_insert((rt.name.clone(), rt.emoji.clone(), 0));
            entry.2 += 1;
        }

        // Track user's reactions
        if let Some(uid) = user_id {
            if reaction.user_id == uid {
                if let Some((_, user_reactions)) = result.get_mut(&reaction.ugc_id) {
                    if !user_reactions.contains(&reaction.reaction_type_id) {
                        user_reactions.push(reaction.reaction_type_id);
                    }
                }
            }
        }
    }

    // Build reaction summaries
    for ((ugc_id, type_id), (name, emoji, count)) in counts {
        if let Some((summaries, _)) = result.get_mut(&ugc_id) {
            summaries.push((type_id, name, emoji, count));
        }
    }

    Ok(result)
}
