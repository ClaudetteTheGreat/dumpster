use super::thread::{validate_thread_form, NewThreadFormData, ThreadForTemplate};
use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{forum_read, forums, poll_options, polls, posts, tag_forums, tags, thread_tags, threads, user_names};
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use sea_orm::{entity::*, query::*, sea_query::Expr, FromQueryResult};
use serde::Deserialize;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(create_thread)
        .service(mark_forum_read)
        .service(mark_all_forums_read)
        .service(view_forums)
        .service(view_forum);
}

/// Thread with tags for template display
pub struct ThreadWithTags {
    pub thread: ThreadForTemplate,
    pub tags: Vec<super::thread::TagForTemplate>,
}

#[derive(Template)]
#[template(path = "forum.html")]
pub struct ForumTemplate<'a> {
    pub client: ClientCtx,
    pub forum: &'a crate::orm::forums::Model,
    pub threads: &'a Vec<ThreadWithTags>,
    pub breadcrumbs: Vec<super::thread::Breadcrumb>,
    pub active_tag: Option<super::thread::TagForTemplate>,
    pub moderators: Vec<ModeratorForTemplate>,
    pub sub_forums: Vec<ForumWithStats>,
    pub available_tags: Vec<super::thread::TagForTemplate>,
}

#[derive(Deserialize)]
pub struct ForumQuery {
    pub tag: Option<String>,
}

/// Moderator info for template display
#[derive(Debug, Clone)]
pub struct ModeratorForTemplate {
    pub user_id: i32,
    pub username: String,
}

/// Fetch moderators for a forum
pub async fn get_forum_moderators(
    forum_id: i32,
) -> Result<Vec<ModeratorForTemplate>, sea_orm::DbErr> {
    use sea_orm::{DbBackend, Statement};

    let db = get_db_pool();

    // Use raw SQL to join forum_moderators with user_names
    let sql = r#"
        SELECT fm.user_id, un.name as username
        FROM forum_moderators fm
        LEFT JOIN user_names un ON un.user_id = fm.user_id
        WHERE fm.forum_id = $1
        ORDER BY un.name
    "#;

    let moderators = ModeratorQueryResult::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [forum_id.into()],
    ))
    .all(db)
    .await?;

    Ok(moderators
        .into_iter()
        .map(|m| ModeratorForTemplate {
            user_id: m.user_id,
            username: m.username.unwrap_or_else(|| "Unknown".to_string()),
        })
        .collect())
}

#[derive(Debug, FromQueryResult)]
struct ModeratorQueryResult {
    user_id: i32,
    username: Option<String>,
}

/// Fetch available tags for a forum (global tags + forum-specific tags)
pub async fn get_available_tags_for_forum(
    forum_id: i32,
) -> Result<Vec<super::thread::TagForTemplate>, sea_orm::DbErr> {
    let db = get_db_pool();

    // Fetch global tags
    let global_tags = tags::Entity::find()
        .filter(tags::Column::IsGlobal.eq(true))
        .order_by_asc(tags::Column::Name)
        .all(db)
        .await?;

    // Fetch forum-specific tags via tag_forums junction table
    let forum_tag_ids: Vec<i32> = tag_forums::Entity::find()
        .filter(tag_forums::Column::ForumId.eq(forum_id))
        .all(db)
        .await?
        .into_iter()
        .map(|tf| tf.tag_id)
        .collect();

    let forum_specific_tags = if forum_tag_ids.is_empty() {
        Vec::new()
    } else {
        tags::Entity::find()
            .filter(tags::Column::Id.is_in(forum_tag_ids))
            .filter(tags::Column::IsGlobal.eq(false))
            .order_by_asc(tags::Column::Name)
            .all(db)
            .await?
    };

    // Combine and convert to TagForTemplate
    let mut all_tags: Vec<super::thread::TagForTemplate> = global_tags
        .into_iter()
        .chain(forum_specific_tags.into_iter())
        .map(|t| super::thread::TagForTemplate {
            id: t.id,
            name: t.name,
            slug: t.slug,
            color: t.color.unwrap_or_else(|| "#6c757d".to_string()),
        })
        .collect();

    // Sort by name
    all_tags.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(all_tags)
}

/// Build breadcrumbs for a forum, including parent forums
pub async fn build_forum_breadcrumbs(
    forum: &crate::orm::forums::Model,
) -> Vec<super::thread::Breadcrumb> {
    use crate::orm::forums;

    let mut breadcrumbs = vec![super::thread::Breadcrumb {
        title: "Forums".to_string(),
        url: Some("/forums".to_string()),
    }];

    // Build list of parent forums (in reverse order)
    let mut parent_chain = Vec::new();
    let mut current_parent_id = forum.parent_id;

    while let Some(parent_id) = current_parent_id {
        if let Ok(Some(parent)) = forums::Entity::find_by_id(parent_id)
            .one(get_db_pool())
            .await
        {
            parent_chain.push(super::thread::Breadcrumb {
                title: parent.label.clone(),
                url: Some(format!("/forums/{}/", parent.id)),
            });
            current_parent_id = parent.parent_id;
        } else {
            break;
        }
    }

    // Add parents in correct order (top-level first)
    parent_chain.reverse();
    breadcrumbs.extend(parent_chain);

    // Add current forum
    breadcrumbs.push(super::thread::Breadcrumb {
        title: forum.label.clone(),
        url: None, // Current page, no link
    });

    breadcrumbs
}

/// Fetch sub-forums for a parent forum
pub async fn get_sub_forums(parent_forum_id: i32) -> Result<Vec<ForumWithStats>, sea_orm::DbErr> {
    use sea_orm::{DbBackend, Statement};

    let db = get_db_pool();

    let sql = r#"
        SELECT
            f.id,
            f.label,
            f.description,
            f.last_post_id,
            f.last_thread_id,
            COALESCE(COUNT(DISTINCT t.id), 0) as thread_count,
            COALESCE(COUNT(DISTINCT p.id), 0) as post_count,
            f.parent_id,
            f.display_order,
            MAX(p.created_at) as last_post_at,
            f.icon,
            f.icon_new,
            f.icon_attachment_id,
            f.icon_new_attachment_id,
            a1.hash as icon_hash,
            a1.filename as icon_filename,
            a2.hash as icon_new_hash,
            a2.filename as icon_new_filename
        FROM forums f
        LEFT JOIN threads t ON t.forum_id = f.id
        LEFT JOIN posts p ON p.thread_id = t.id
        LEFT JOIN attachments a1 ON a1.id = f.icon_attachment_id
        LEFT JOIN attachments a2 ON a2.id = f.icon_new_attachment_id
        WHERE f.parent_id = $1
        GROUP BY f.id, f.label, f.description, f.last_post_id, f.last_thread_id, f.parent_id, f.display_order, f.icon, f.icon_new, f.icon_attachment_id, f.icon_new_attachment_id, a1.hash, a1.filename, a2.hash, a2.filename
        ORDER BY f.display_order, f.id
    "#;

    ForumWithStats::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [parent_forum_id.into()],
    ))
    .all(db)
    .await
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct ForumWithStats {
    pub id: i32,
    pub label: String,
    pub description: Option<String>,
    pub last_post_id: Option<i32>,
    pub last_thread_id: Option<i32>,
    pub thread_count: i64,
    pub post_count: i64,
    pub parent_id: Option<i32>,
    pub display_order: i32,
    pub last_post_at: Option<chrono::NaiveDateTime>,
    pub icon: String,
    pub icon_new: String,
    pub icon_attachment_id: Option<i32>,
    pub icon_new_attachment_id: Option<i32>,
    // Attachment info for icon images
    pub icon_hash: Option<String>,
    pub icon_filename: Option<String>,
    pub icon_new_hash: Option<String>,
    pub icon_new_filename: Option<String>,
}

/// Forum with its sub-forums for hierarchical display
#[derive(Debug, Clone)]
pub struct ForumWithChildren {
    pub forum: ForumWithStats,
    pub children: Vec<ForumWithStats>,
}

use std::collections::HashSet;

#[derive(Template)]
#[template(path = "forums.html")]
pub struct ForumIndexTemplate<'a> {
    pub client: ClientCtx,
    pub forums: &'a Vec<ForumWithChildren>,
    pub unread_forums: HashSet<i32>,
    pub online_users: Vec<crate::user::OnlineUser>,
    pub online_count: i64,
    pub online_users_len: i64,
}

#[post("/forums/{forum}/post-thread")]
pub async fn create_thread(
    req: actix_web::HttpRequest,
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<NewThreadFormData>,
    path: web::Path<i32>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Require authentication for thread creation
    let user_id = client.require_login()?;

    // Extract and store IP address for moderation
    let ip_id = if let Some(ip_addr) = crate::ip::extract_client_ip(&req) {
        crate::ip::get_or_create_ip_id(&ip_addr)
            .await
            .map_err(error::ErrorInternalServerError)?
    } else {
        None
    };

    // Rate limiting - prevent thread spam
    if let Err(e) = crate::rate_limit::check_thread_rate_limit(user_id) {
        log::warn!(
            "Rate limit exceeded for thread creation: user_id={}",
            user_id
        );
        return Err(error::ErrorTooManyRequests(format!(
            "You're creating threads too quickly. Please wait {} seconds.",
            e.retry_after_seconds
        )));
    }

    use crate::ugc::{create_ugc, NewUgcPartial};
    let forum_id = path.into_inner();

    // Check forum-specific permission for thread creation
    if !client.can_create_thread_in_forum(&forum_id) {
        return Err(error::ErrorForbidden(
            "You do not have permission to create threads in this forum.",
        ));
    }

    // Fetch forum for tag settings
    let forum = forums::Entity::find_by_id(forum_id)
        .one(get_db_pool())
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Forum not found"))?;

    // Run form data through validator.
    let (form, validated_poll) = validate_thread_form(form)?;

    // Spam detection for thread content
    let user_post_count = posts::Entity::find()
        .filter(posts::Column::UserId.eq(user_id))
        .count(get_db_pool())
        .await
        .unwrap_or(0) as i32;

    // Check both title and content for spam
    let title_spam = crate::spam::analyze_content(&form.title, user_post_count);
    let content_spam = crate::spam::analyze_content(&form.content, user_post_count);

    if title_spam.is_spam || content_spam.is_spam {
        log::warn!(
            "Spam detected in thread: user_id={}, title_score={:.2}, content_score={:.2}",
            user_id,
            title_spam.score,
            content_spam.score
        );
        return Err(error::ErrorBadRequest(
            "Your thread has been flagged as potential spam. Please revise your content.",
        ));
    }

    // Word filter: check title and content
    let title_filter = crate::word_filter::apply_filters(&form.title);
    if title_filter.blocked {
        log::warn!(
            "Thread title blocked by word filter: user_id={}, patterns={:?}",
            user_id,
            title_filter.matched_patterns
        );
        return Err(error::ErrorBadRequest(
            title_filter
                .block_reason
                .unwrap_or_else(|| "Your thread title contains blocked content.".to_string()),
        ));
    }

    let content_filter = crate::word_filter::apply_filters(&form.content);
    if content_filter.blocked {
        log::warn!(
            "Thread content blocked by word filter: user_id={}, patterns={:?}",
            user_id,
            content_filter.matched_patterns
        );
        return Err(error::ErrorBadRequest(
            content_filter
                .block_reason
                .unwrap_or_else(|| "Your thread content contains blocked content.".to_string()),
        ));
    }

    // Use filtered content (with replacements applied)
    let filtered_title = title_filter.content;
    let filtered_content = content_filter.content;

    // Begin Transaction
    let txn = get_db_pool()
        .begin()
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Step 1. Create the UGC.
    let revision = create_ugc(
        &txn,
        NewUgcPartial {
            ip_id,
            user_id: Some(user_id),
            content: &filtered_content,
        },
    )
    .await
    .map_err(error::ErrorInternalServerError)?;

    // Step 2. Create a thread.
    let thread = threads::ActiveModel {
        user_id: Set(Some(user_id)),
        forum_id: Set(forum_id),
        created_at: Set(revision.created_at),
        title: Set(filtered_title.trim().to_owned()),
        subtitle: Set(form
            .subtitle
            .to_owned()
            .map(|s| s.trim().to_owned())
            .filter(|s| s.is_empty())),
        view_count: Set(0),
        post_count: Set(1),
        ..Default::default()
    };
    let thread_res = threads::Entity::insert(thread)
        .exec(&txn)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Step 3. Create a post with the correct associations.
    let new_post = posts::ActiveModel {
        user_id: Set(client.get_id()),
        thread_id: Set(thread_res.last_insert_id),
        ugc_id: Set(revision.ugc_id),
        created_at: Set(revision.created_at),
        position: Set(1),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(error::ErrorInternalServerError)?;

    // Step 4. Update the thread to include last, first post id info.
    threads::Entity::update_many()
        .col_expr(threads::Column::PostCount, Expr::value(1))
        .col_expr(threads::Column::FirstPostId, Expr::value(new_post.id))
        .col_expr(threads::Column::LastPostId, Expr::value(new_post.id))
        .col_expr(
            threads::Column::LastPostAt,
            Expr::value(revision.created_at),
        )
        .filter(threads::Column::Id.eq(thread_res.last_insert_id))
        .exec(&txn)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Step 5. Create poll if provided.
    if let Some(poll_data) = validated_poll {
        let poll = polls::ActiveModel {
            thread_id: Set(thread_res.last_insert_id),
            question: Set(poll_data.question),
            max_choices: Set(poll_data.max_choices),
            allow_change_vote: Set(poll_data.allow_change_vote),
            show_results_before_vote: Set(poll_data.show_results_before_vote),
            closes_at: Set(poll_data.closes_at),
            created_at: Set(revision.created_at),
            ..Default::default()
        };
        let poll_res = polls::Entity::insert(poll)
            .exec(&txn)
            .await
            .map_err(error::ErrorInternalServerError)?;

        // Create poll options
        for (i, option_text) in poll_data.options.iter().enumerate() {
            let option = poll_options::ActiveModel {
                poll_id: Set(poll_res.last_insert_id),
                option_text: Set(option_text.clone()),
                display_order: Set(i as i32),
                vote_count: Set(0),
                ..Default::default()
            };
            poll_options::Entity::insert(option)
                .exec(&txn)
                .await
                .map_err(error::ErrorInternalServerError)?;
        }
    }

    // Step 6. Create/link tags if provided and enabled for this forum.
    if !form.tags.is_empty() && forum.tags_enabled {
        for tag_name in &form.tags {
            // Create slug from tag name
            let slug = tag_name
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect::<String>();

            if slug.is_empty() {
                continue;
            }

            // Find existing tag by slug
            let existing_tag = tags::Entity::find()
                .filter(tags::Column::Slug.eq(slug.clone()))
                .one(&txn)
                .await
                .map_err(error::ErrorInternalServerError)?;

            let tag_id = if let Some(tag) = existing_tag {
                // Check if tag is available in this forum (global or has tag_forums entry)
                if tag.is_global {
                    Some(tag.id)
                } else {
                    // Check if tag is assigned to this forum
                    let has_forum = tag_forums::Entity::find()
                        .filter(tag_forums::Column::TagId.eq(tag.id))
                        .filter(tag_forums::Column::ForumId.eq(forum_id))
                        .one(&txn)
                        .await
                        .map_err(error::ErrorInternalServerError)?
                        .is_some();

                    if has_forum {
                        Some(tag.id)
                    } else {
                        None // Tag exists but not available in this forum
                    }
                }
            } else if forum.restrict_tags {
                // Forum restricts to predefined tags only - skip non-existent tags
                None
            } else {
                // Create new forum-specific tag
                let new_tag = tags::ActiveModel {
                    name: Set(tag_name.clone()),
                    slug: Set(slug),
                    is_global: Set(false),
                    created_at: Set(revision.created_at),
                    ..Default::default()
                };
                let tag_res = tags::Entity::insert(new_tag)
                    .exec(&txn)
                    .await
                    .map_err(error::ErrorInternalServerError)?;

                let new_tag_id = tag_res.last_insert_id;

                // Associate the new tag with this forum
                let tag_forum = tag_forums::ActiveModel {
                    tag_id: Set(new_tag_id),
                    forum_id: Set(forum_id),
                    ..Default::default()
                };
                let _ = tag_forums::Entity::insert(tag_forum).exec(&txn).await;

                Some(new_tag_id)
            };

            // Link tag to thread if we have a valid tag_id
            if let Some(tid) = tag_id {
                let thread_tag = thread_tags::ActiveModel {
                    thread_id: Set(thread_res.last_insert_id),
                    tag_id: Set(tid),
                    created_at: Set(revision.created_at),
                    ..Default::default()
                };
                // Ignore duplicate key errors (tag already linked)
                let _ = thread_tags::Entity::insert(thread_tag).exec(&txn).await;
            }
        }
    }

    // Close transaction
    txn.commit()
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Check and award any automatic badges the user may have earned (async, non-blocking)
    actix::spawn(async move {
        crate::badges::check_and_award_automatic_badges(user_id).await;
    });

    // Record activity for the feed (async, non-blocking)
    let thread_id = thread_res.last_insert_id;
    let title_for_activity = filtered_title.clone();
    actix::spawn(async move {
        if let Err(e) = crate::activities::record_thread_created(
            user_id,
            thread_id,
            forum_id,
            &title_for_activity,
        )
        .await
        {
            log::warn!("Failed to record thread creation activity: {}", e);
        }
    });

    Ok(HttpResponse::Found()
        .append_header((
            "Location",
            format!("/threads/{}/", thread_res.last_insert_id),
        ))
        .finish())
}

#[get("/forums/{forum}/")]
pub async fn view_forum(
    client: ClientCtx,
    path: web::Path<i32>,
    query: web::Query<ForumQuery>,
) -> Result<impl Responder, Error> {
    use crate::orm::forums;

    let forum_id = path.into_inner();
    let forum = forums::Entity::find_by_id(forum_id)
        .one(get_db_pool())
        .await
        .map_err(|_| error::ErrorInternalServerError("Could not look up forum."))?
        .ok_or_else(|| error::ErrorNotFound("Forum not found."))?;

    // Check forum-specific view permission
    if !client.can_view_forum(&forum_id) {
        return Err(error::ErrorForbidden(
            "You do not have permission to view this forum.",
        ));
    }

    // Check if filtering by tag
    let (threads, active_tag) = if let Some(ref tag_slug) = query.tag {
        // Find the tag by slug
        let tag_opt = tags::Entity::find()
            .filter(tags::Column::Slug.eq(tag_slug.clone()))
            .one(get_db_pool())
            .await
            .map_err(error::ErrorInternalServerError)?;

        // Check if tag is available in this forum (global or has tag_forums entry)
        let tag = if let Some(t) = tag_opt {
            if t.is_global {
                Some(t)
            } else {
                // Check if tag is assigned to this forum
                let has_forum = tag_forums::Entity::find()
                    .filter(tag_forums::Column::TagId.eq(t.id))
                    .filter(tag_forums::Column::ForumId.eq(forum_id))
                    .one(get_db_pool())
                    .await
                    .map_err(error::ErrorInternalServerError)?
                    .is_some();

                if has_forum {
                    Some(t)
                } else {
                    None
                }
            }
        } else {
            None
        };

        if let Some(tag) = tag {
            // Get thread IDs that have this tag
            let thread_tag_records = thread_tags::Entity::find()
                .filter(thread_tags::Column::TagId.eq(tag.id))
                .all(get_db_pool())
                .await
                .unwrap_or_default();

            let tagged_thread_ids: Vec<i32> =
                thread_tag_records.iter().map(|tt| tt.thread_id).collect();

            let threads: Vec<ThreadForTemplate> = if tagged_thread_ids.is_empty() {
                Vec::new()
            } else {
                threads::Entity::find()
                    .left_join(user_names::Entity)
                    .column_as(user_names::Column::Name, "username")
                    .filter(threads::Column::ForumId.eq(forum_id))
                    .filter(threads::Column::Id.is_in(tagged_thread_ids))
                    .order_by_desc(threads::Column::IsPinned)
                    .order_by_desc(threads::Column::LastPostAt)
                    .into_model::<ThreadForTemplate>()
                    .all(get_db_pool())
                    .await
                    .unwrap_or_default()
            };

            let active_tag = super::thread::TagForTemplate {
                id: tag.id,
                name: tag.name,
                slug: tag.slug,
                color: tag.color.unwrap_or_else(|| "#6c757d".to_string()),
            };

            (threads, Some(active_tag))
        } else {
            // Tag not found, show all threads
            let threads: Vec<ThreadForTemplate> = threads::Entity::find()
                .left_join(user_names::Entity)
                .column_as(user_names::Column::Name, "username")
                .filter(threads::Column::ForumId.eq(forum_id))
                .order_by_desc(threads::Column::IsPinned)
                .order_by_desc(threads::Column::LastPostAt)
                .into_model::<ThreadForTemplate>()
                .all(get_db_pool())
                .await
                .unwrap_or_default();
            (threads, None)
        }
    } else {
        // No tag filter
        let threads: Vec<ThreadForTemplate> = threads::Entity::find()
            .left_join(user_names::Entity)
            .column_as(user_names::Column::Name, "username")
            .filter(threads::Column::ForumId.eq(forum_id))
            .order_by_desc(threads::Column::IsPinned)
            .order_by_desc(threads::Column::LastPostAt)
            .into_model::<ThreadForTemplate>()
            .all(get_db_pool())
            .await
            .unwrap_or_default();
        (threads, None)
    };

    // Build breadcrumbs (including parent forums)
    let breadcrumbs = build_forum_breadcrumbs(&forum).await;

    // Fetch tags for all threads
    let thread_ids: Vec<i32> = threads.iter().map(|t| t.id).collect();
    let mut thread_tags_map = super::thread::get_tags_for_threads(&thread_ids)
        .await
        .unwrap_or_default();

    // Combine threads with their tags
    let threads_with_tags: Vec<ThreadWithTags> = threads
        .into_iter()
        .map(|t| {
            let tags = thread_tags_map.remove(&t.id).unwrap_or_default();
            ThreadWithTags { thread: t, tags }
        })
        .collect();

    // Fetch forum moderators
    let moderators = get_forum_moderators(forum_id).await.unwrap_or_default();

    // Fetch sub-forums
    let sub_forums = get_sub_forums(forum_id).await.unwrap_or_default();

    // Fetch available tags for this forum (global tags + forum-specific tags)
    let available_tags = get_available_tags_for_forum(forum_id).await.unwrap_or_default();

    Ok(ForumTemplate {
        client: client.to_owned(),
        forum: &forum,
        threads: &threads_with_tags,
        breadcrumbs,
        active_tag,
        moderators,
        sub_forums,
        available_tags,
    }
    .to_response())
}

#[get("/forums")]
pub async fn view_forums(client: ClientCtx) -> Result<impl Responder, Error> {
    render_forum_list(client).await
}

pub async fn render_forum_list(client: ClientCtx) -> Result<impl Responder, Error> {
    #[allow(unused_imports)]
    use sea_orm::sea_query::Alias;
    use sea_orm::{DbBackend, Statement};

    let db = get_db_pool();

    // Query forums with thread and post counts and latest post timestamp
    let sql = r#"
        SELECT
            f.id,
            f.label,
            f.description,
            f.last_post_id,
            f.last_thread_id,
            COALESCE(COUNT(DISTINCT t.id), 0) as thread_count,
            COALESCE(COUNT(DISTINCT p.id), 0) as post_count,
            f.parent_id,
            f.display_order,
            MAX(p.created_at) as last_post_at,
            f.icon,
            f.icon_new,
            f.icon_attachment_id,
            f.icon_new_attachment_id,
            a1.hash as icon_hash,
            a1.filename as icon_filename,
            a2.hash as icon_new_hash,
            a2.filename as icon_new_filename
        FROM forums f
        LEFT JOIN threads t ON t.forum_id = f.id
        LEFT JOIN posts p ON p.thread_id = t.id
        LEFT JOIN attachments a1 ON a1.id = f.icon_attachment_id
        LEFT JOIN attachments a2 ON a2.id = f.icon_new_attachment_id
        GROUP BY f.id, f.label, f.description, f.last_post_id, f.last_thread_id, f.parent_id, f.display_order, f.icon, f.icon_new, f.icon_attachment_id, f.icon_new_attachment_id, a1.hash, a1.filename, a2.hash, a2.filename
        ORDER BY f.display_order, f.id
    "#;

    let all_forums = ForumWithStats::find_by_statement(Statement::from_string(
        DbBackend::Postgres,
        sql.to_string(),
    ))
    .all(db)
    .await
    .unwrap_or_default();

    // Get unread forums for logged-in users
    let unread_forums = if let Some(user_id) = client.get_id() {
        get_unread_forums(user_id, &all_forums)
            .await
            .unwrap_or_default()
    } else {
        HashSet::new()
    };

    // Organize forums into hierarchical structure
    // Top-level forums (parent_id = NULL) with their children
    let forums = organize_forums_hierarchy(&all_forums);

    // Get online users for display
    let online_users = crate::user::get_online_users(20).await.unwrap_or_default();
    let online_count = crate::user::count_online_users().await.unwrap_or(0);

    let online_users_len = online_users.len() as i64;
    Ok(ForumIndexTemplate {
        client: client.to_owned(),
        forums: &forums,
        unread_forums,
        online_users,
        online_count,
        online_users_len,
    }
    .to_response())
}

/// Get set of forum IDs that have unread posts for the given user
async fn get_unread_forums(
    user_id: i32,
    forums: &[ForumWithStats],
) -> Result<HashSet<i32>, sea_orm::DbErr> {
    let db = get_db_pool();

    // Get all forum read timestamps for this user
    let read_records = forum_read::Entity::find()
        .filter(forum_read::Column::UserId.eq(user_id))
        .all(db)
        .await?;

    // Build a map of forum_id -> read_at
    let read_map: std::collections::HashMap<i32, chrono::NaiveDateTime> = read_records
        .into_iter()
        .map(|r| (r.forum_id, r.read_at))
        .collect();

    // Find forums with posts newer than the read timestamp
    let mut unread = HashSet::new();
    for forum in forums {
        if let Some(last_post_at) = forum.last_post_at {
            match read_map.get(&forum.id) {
                Some(read_at) if last_post_at > *read_at => {
                    unread.insert(forum.id);
                }
                None => {
                    // Never marked as read, so it's unread if it has posts
                    unread.insert(forum.id);
                }
                _ => {}
            }
        }
    }

    Ok(unread)
}

/// Organize flat list of forums into hierarchical structure
fn organize_forums_hierarchy(all_forums: &[ForumWithStats]) -> Vec<ForumWithChildren> {
    use std::collections::HashMap;

    // Build a map of parent_id -> children
    let mut children_map: HashMap<i32, Vec<ForumWithStats>> = HashMap::new();
    let mut top_level: Vec<ForumWithStats> = Vec::new();

    for forum in all_forums {
        if let Some(parent_id) = forum.parent_id {
            children_map
                .entry(parent_id)
                .or_default()
                .push(forum.clone());
        } else {
            top_level.push(forum.clone());
        }
    }

    // Sort children by display_order
    for children in children_map.values_mut() {
        children.sort_by(|a, b| a.display_order.cmp(&b.display_order));
    }

    // Build the final structure
    top_level
        .into_iter()
        .map(|forum| {
            let children = children_map.remove(&forum.id).unwrap_or_default();
            ForumWithChildren { forum, children }
        })
        .collect()
}

/// Mark a specific forum as read
#[post("/forums/{forum}/mark-read")]
pub async fn mark_forum_read(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<CsrfForm>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Require authentication
    let user_id = client.require_login()?;
    let forum_id = path.into_inner();

    let db = get_db_pool();
    let now = chrono::Utc::now().naive_utc();

    // Delete existing record if any
    forum_read::Entity::delete_many()
        .filter(forum_read::Column::UserId.eq(user_id))
        .filter(forum_read::Column::ForumId.eq(forum_id))
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Insert new record
    let record = forum_read::ActiveModel {
        user_id: Set(user_id),
        forum_id: Set(forum_id),
        read_at: Set(now),
    };
    forum_read::Entity::insert(record)
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Redirect back to forum
    Ok(HttpResponse::Found()
        .append_header(("Location", format!("/forums/{}/", forum_id)))
        .finish())
}

#[derive(Deserialize)]
pub struct CsrfForm {
    pub csrf_token: String,
}

/// Mark all forums as read
#[post("/forums/mark-all-read")]
pub async fn mark_all_forums_read(
    client: ClientCtx,
    cookies: actix_session::Session,
    form: web::Form<CsrfForm>,
) -> Result<impl Responder, Error> {
    use crate::orm::forums;

    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Require authentication
    let user_id = client.require_login()?;

    let db = get_db_pool();
    let now = chrono::Utc::now().naive_utc();

    // Delete all existing read records for this user
    forum_read::Entity::delete_many()
        .filter(forum_read::Column::UserId.eq(user_id))
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Get all forum IDs and insert read records
    let all_forums = forums::Entity::find()
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    for forum in all_forums {
        let record = forum_read::ActiveModel {
            user_id: Set(user_id),
            forum_id: Set(forum.id),
            read_at: Set(now),
        };

        forum_read::Entity::insert(record)
            .exec(db)
            .await
            .map_err(error::ErrorInternalServerError)?;
    }

    // Redirect back to forums list
    Ok(HttpResponse::Found()
        .append_header(("Location", "/forums"))
        .finish())
}
