use super::thread::{validate_thread_form, NewThreadFormData, ThreadForTemplate};
use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{poll_options, polls, posts, tags, thread_tags, threads, user_names};
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use serde::Deserialize;
use askama_actix::{Template, TemplateToResponse};
use sea_orm::{entity::*, query::*, sea_query::Expr, FromQueryResult};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(create_thread)
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
pub async fn get_forum_moderators(forum_id: i32) -> Result<Vec<ModeratorForTemplate>, sea_orm::DbErr> {
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

#[derive(Debug, FromQueryResult)]
pub struct ForumWithStats {
    pub id: i32,
    pub label: String,
    pub description: Option<String>,
    pub last_post_id: Option<i32>,
    pub last_thread_id: Option<i32>,
    pub thread_count: i64,
    pub post_count: i64,
}

#[derive(Template)]
#[template(path = "forums.html")]
pub struct ForumIndexTemplate<'a> {
    pub client: ClientCtx,
    pub forums: &'a Vec<ForumWithStats>,
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

    // Run form data through validator.
    let (form, validated_poll) = validate_thread_form(form)?;

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
            content: &form.content,
        },
    )
    .await
    .map_err(error::ErrorInternalServerError)?;

    // Step 2. Create a thread.
    let thread = threads::ActiveModel {
        user_id: Set(Some(user_id)),
        forum_id: Set(forum_id),
        created_at: Set(revision.created_at),
        title: Set(form.title.trim().to_owned()),
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

    // Step 6. Create/link tags if provided.
    if !form.tags.is_empty() {
        for tag_name in &form.tags {
            // Create slug from tag name
            let slug = tag_name
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect::<String>();

            if slug.is_empty() {
                continue;
            }

            // Find or create the tag (forum-specific or global)
            let existing_tag = tags::Entity::find()
                .filter(tags::Column::Slug.eq(slug.clone()))
                .filter(
                    tags::Column::ForumId
                        .eq(forum_id)
                        .or(tags::Column::ForumId.is_null()),
                )
                .one(&txn)
                .await
                .map_err(error::ErrorInternalServerError)?;

            let tag_id = if let Some(tag) = existing_tag {
                tag.id
            } else {
                // Create new forum-specific tag
                let new_tag = tags::ActiveModel {
                    name: Set(tag_name.clone()),
                    slug: Set(slug),
                    forum_id: Set(Some(forum_id)),
                    created_at: Set(revision.created_at),
                    ..Default::default()
                };
                let tag_res = tags::Entity::insert(new_tag)
                    .exec(&txn)
                    .await
                    .map_err(error::ErrorInternalServerError)?;
                tag_res.last_insert_id
            };

            // Link tag to thread
            let thread_tag = thread_tags::ActiveModel {
                thread_id: Set(thread_res.last_insert_id),
                tag_id: Set(tag_id),
                created_at: Set(revision.created_at),
                ..Default::default()
            };
            // Ignore duplicate key errors (tag already linked)
            let _ = thread_tags::Entity::insert(thread_tag).exec(&txn).await;
        }
    }

    // Close transaction
    txn.commit()
        .await
        .map_err(error::ErrorInternalServerError)?;

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

    // Check if filtering by tag
    let (threads, active_tag) = if let Some(ref tag_slug) = query.tag {
        // Find the tag
        let tag = tags::Entity::find()
            .filter(tags::Column::Slug.eq(tag_slug.clone()))
            .filter(
                tags::Column::ForumId
                    .eq(forum_id)
                    .or(tags::Column::ForumId.is_null()),
            )
            .one(get_db_pool())
            .await
            .map_err(error::ErrorInternalServerError)?;

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

    // Build breadcrumbs
    let breadcrumbs = vec![
        super::thread::Breadcrumb {
            title: "Forums".to_string(),
            url: Some("/forums".to_string()),
        },
        super::thread::Breadcrumb {
            title: forum.label.clone(),
            url: None, // Current page, no link
        },
    ];

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

    Ok(ForumTemplate {
        client: client.to_owned(),
        forum: &forum,
        threads: &threads_with_tags,
        breadcrumbs,
        active_tag,
        moderators,
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

    // Query forums with thread and post counts using subqueries
    let sql = r#"
        SELECT
            f.id,
            f.label,
            f.description,
            f.last_post_id,
            f.last_thread_id,
            COALESCE(COUNT(DISTINCT t.id), 0) as thread_count,
            COALESCE(COUNT(DISTINCT p.id), 0) as post_count
        FROM forums f
        LEFT JOIN threads t ON t.forum_id = f.id
        LEFT JOIN posts p ON p.thread_id = t.id
        GROUP BY f.id, f.label, f.description, f.last_post_id, f.last_thread_id
        ORDER BY f.id
    "#;

    let forums = ForumWithStats::find_by_statement(Statement::from_string(
        DbBackend::Postgres,
        sql.to_string(),
    ))
    .all(db)
    .await
    .unwrap_or_default();

    Ok(ForumIndexTemplate {
        client: client.to_owned(),
        forums: &forums,
    }
    .to_response())
}
