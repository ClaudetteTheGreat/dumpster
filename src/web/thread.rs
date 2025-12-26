use super::post::PostForTemplate;
use crate::attachment::AttachmentForTemplate;
use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::posts::Entity as Post;
use crate::orm::threads::Entity as Thread;
use crate::orm::{
    poll_options, poll_votes, polls, posts, tags, thread_read, thread_tags, threads, ugc_deletions,
};
use crate::template::{Paginator, PaginatorToHtml};
use crate::user::Profile as UserProfile;
use actix_multipart::Multipart;
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use sea_orm::{entity::*, query::*, sea_query::Expr, DbErr, FromQueryResult, QueryFilter};
use serde::Deserialize;
use std::{collections::HashMap, str};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(create_reply)
        .service(view_thread_unread)
        .service(view_thread)
        .service(view_thread_page)
        .service(delete_thread)
        .service(restore_thread)
        .service(legal_hold_thread)
        .service(remove_legal_hold_thread);
}

/// Breadcrumb item for navigation
#[derive(Debug, Clone)]
pub struct Breadcrumb {
    pub title: String,
    pub url: Option<String>,
}

/// Tag for template display
#[derive(Debug, Clone)]
pub struct TagForTemplate {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub color: String,
}

#[derive(Debug, FromQueryResult)]
pub struct ThreadForTemplate {
    pub id: i32,
    pub user_id: Option<i32>,
    pub created_at: chrono::naive::NaiveDateTime,
    pub title: String,
    pub subtitle: Option<String>,
    pub view_count: i32,
    pub post_count: i32,
    pub first_post_id: Option<i32>,
    pub last_post_id: Option<i32>,
    pub last_post_at: Option<chrono::naive::NaiveDateTime>,
    pub is_locked: bool,
    pub is_pinned: bool,
    pub prefix: Option<String>,
    // join user
    pub username: Option<String>,
}

#[derive(Deserialize)]
pub struct NewThreadFormData {
    pub title: String,
    pub subtitle: Option<String>,
    pub content: String,
    pub csrf_token: String,
    // Tags (comma-separated or multiple inputs)
    #[serde(default)]
    pub tags: Vec<String>,
    // Poll fields (all optional - only create poll if question is provided)
    pub poll_question: Option<String>,
    #[serde(default)]
    pub poll_options: Vec<String>,
    #[serde(default = "default_max_choices")]
    pub poll_max_choices: i32,
    #[serde(default)]
    pub poll_allow_change_vote: bool,
    #[serde(default)]
    pub poll_show_results_before_vote: bool,
    pub poll_closes_at: Option<String>,
}

fn default_max_choices() -> i32 {
    1
}

/// Validated poll data ready for insertion
#[derive(Debug, Clone)]
pub struct ValidatedPoll {
    pub question: String,
    pub options: Vec<String>,
    pub max_choices: i32,
    pub allow_change_vote: bool,
    pub show_results_before_vote: bool,
    pub closes_at: Option<chrono::NaiveDateTime>,
}

/// Poll option for template display
#[derive(Debug, Clone)]
pub struct PollOptionForTemplate {
    pub id: i32,
    pub option_text: String,
    pub vote_count: i32,
    pub percentage: f64,
    pub user_selected: bool,
}

/// Poll data for template display
#[derive(Debug, Clone)]
pub struct PollForTemplate {
    pub id: i32,
    pub question: String,
    pub max_choices: i32,
    pub allow_change_vote: bool,
    pub show_results_before_vote: bool,
    pub closes_at: Option<chrono::NaiveDateTime>,
    pub is_closed: bool,
    pub total_votes: i32,
    pub options: Vec<PollOptionForTemplate>,
    pub has_voted: bool,
}

/// Similar thread for display in sidebar
#[derive(Debug, Clone)]
pub struct SimilarThreadForTemplate {
    pub id: i32,
    pub title: String,
    pub post_count: i32,
    pub matching_tags: i32,
}

#[derive(Template)]
#[template(path = "thread.html")]
pub struct ThreadTemplate<'a> {
    pub client: ClientCtx,
    pub forum: crate::orm::forums::Model,
    pub thread: crate::orm::threads::Model,
    pub paginator: Paginator,
    pub posts: &'a Vec<(PostForTemplate, Option<UserProfile>)>,
    pub attachments: &'a HashMap<i32, Vec<AttachmentForTemplate>>,
    pub is_watching: bool,
    pub email_on_reply: bool,
    pub breadcrumbs: Vec<Breadcrumb>,
    pub poll: Option<PollForTemplate>,
    pub tags: Vec<TagForTemplate>,
    pub similar_threads: Vec<SimilarThreadForTemplate>,
}

mod filters {
    pub fn ugc(s: &str) -> ::askama::Result<String> {
        Ok(crate::bbcode::parse(s))
    }
}

pub const DEFAULT_POSTS_PER_PAGE: i32 = 25;

/// Returns which human-readable page number this position will appear in.
pub fn get_page_for_pos(pos: i32, posts_per_page: i32) -> i32 {
    ((std::cmp::max(1, pos) - 1) / posts_per_page) + 1
}

pub fn get_pages_in_thread(cnt: i32, posts_per_page: i32) -> i32 {
    ((std::cmp::max(1, cnt) - 1) / posts_per_page) + 1
}

/// Returns the relative URL for the thread at this position.
/// Uses default posts per page for URL generation.
pub fn get_url_for_pos(thread_id: i32, pos: i32) -> String {
    let page = get_page_for_pos(pos, DEFAULT_POSTS_PER_PAGE);
    format!(
        "/threads/{}/{}",
        thread_id,
        if page == 1 {
            String::new()
        } else {
            format!("page-{}", page)
        }
    )
}

/// Fetches poll data for a thread, if one exists.
pub async fn get_poll_for_thread(
    thread_id: i32,
    user_id: Option<i32>,
) -> Result<Option<PollForTemplate>, sea_orm::DbErr> {
    use sea_orm::EntityTrait;

    let db = get_db_pool();

    // Try to find poll for this thread
    let poll = polls::Entity::find()
        .filter(polls::Column::ThreadId.eq(thread_id))
        .one(db)
        .await?;

    let poll = match poll {
        Some(p) => p,
        None => return Ok(None),
    };

    // Fetch poll options
    let options = poll_options::Entity::find()
        .filter(poll_options::Column::PollId.eq(poll.id))
        .order_by_asc(poll_options::Column::DisplayOrder)
        .all(db)
        .await?;

    // Calculate total votes
    let total_votes: i32 = options.iter().map(|o| o.vote_count).sum();

    // Check if user has voted
    let user_voted_options = if let Some(uid) = user_id {
        poll_votes::Entity::find()
            .filter(poll_votes::Column::PollId.eq(poll.id))
            .filter(poll_votes::Column::UserId.eq(uid))
            .all(db)
            .await?
            .into_iter()
            .map(|v| v.option_id)
            .collect()
    } else {
        Vec::new()
    };

    // Check if poll is closed
    let is_closed = poll.closes_at.map_or(false, |closes_at| {
        closes_at < chrono::Utc::now().naive_utc()
    });

    // Build options with percentages
    let options_for_template: Vec<PollOptionForTemplate> = options
        .into_iter()
        .map(|opt| {
            let percentage = if total_votes > 0 {
                (opt.vote_count as f64 / total_votes as f64) * 100.0
            } else {
                0.0
            };
            let user_selected = user_voted_options.contains(&opt.id);
            PollOptionForTemplate {
                id: opt.id,
                option_text: opt.option_text,
                vote_count: opt.vote_count,
                percentage,
                user_selected,
            }
        })
        .collect();

    let has_voted = !user_voted_options.is_empty();

    Ok(Some(PollForTemplate {
        id: poll.id,
        question: poll.question,
        max_choices: poll.max_choices,
        allow_change_vote: poll.allow_change_vote,
        show_results_before_vote: poll.show_results_before_vote,
        closes_at: poll.closes_at,
        is_closed,
        total_votes,
        options: options_for_template,
        has_voted,
    }))
}

/// Fetches tags for a thread.
pub async fn get_tags_for_thread(thread_id: i32) -> Result<Vec<TagForTemplate>, sea_orm::DbErr> {
    use sea_orm::EntityTrait;

    let db = get_db_pool();

    // Find all tag IDs for this thread
    let thread_tag_records = thread_tags::Entity::find()
        .filter(thread_tags::Column::ThreadId.eq(thread_id))
        .all(db)
        .await?;

    if thread_tag_records.is_empty() {
        return Ok(Vec::new());
    }

    let tag_ids: Vec<i32> = thread_tag_records.iter().map(|tt| tt.tag_id).collect();

    // Fetch the actual tags
    let tag_records = tags::Entity::find()
        .filter(tags::Column::Id.is_in(tag_ids))
        .all(db)
        .await?;

    Ok(tag_records
        .into_iter()
        .map(|t| TagForTemplate {
            id: t.id,
            name: t.name,
            slug: t.slug,
            color: t.color.unwrap_or_else(|| "#6c757d".to_string()),
        })
        .collect())
}

/// Fetches tags for multiple threads at once (for listings).
pub async fn get_tags_for_threads(
    thread_ids: &[i32],
) -> Result<std::collections::HashMap<i32, Vec<TagForTemplate>>, sea_orm::DbErr> {
    use sea_orm::EntityTrait;

    if thread_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let db = get_db_pool();

    // Find all thread-tag associations for these threads
    let thread_tag_records = thread_tags::Entity::find()
        .filter(thread_tags::Column::ThreadId.is_in(thread_ids.to_vec()))
        .all(db)
        .await?;

    if thread_tag_records.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    // Get unique tag IDs
    let tag_ids: Vec<i32> = thread_tag_records
        .iter()
        .map(|tt| tt.tag_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Fetch all tags
    let tag_records = tags::Entity::find()
        .filter(tags::Column::Id.is_in(tag_ids))
        .all(db)
        .await?;

    // Create a map of tag_id -> TagForTemplate
    let tags_by_id: std::collections::HashMap<i32, TagForTemplate> = tag_records
        .into_iter()
        .map(|t| {
            (
                t.id,
                TagForTemplate {
                    id: t.id,
                    name: t.name,
                    slug: t.slug,
                    color: t.color.unwrap_or_else(|| "#6c757d".to_string()),
                },
            )
        })
        .collect();

    // Build the result map: thread_id -> Vec<TagForTemplate>
    let mut result: std::collections::HashMap<i32, Vec<TagForTemplate>> =
        std::collections::HashMap::new();

    for tt in thread_tag_records {
        if let Some(tag) = tags_by_id.get(&tt.tag_id) {
            result
                .entry(tt.thread_id)
                .or_insert_with(Vec::new)
                .push(tag.clone());
        }
    }

    Ok(result)
}

/// Fetches similar threads based on shared tags.
/// Returns up to 5 threads that share the most tags with the current thread.
pub async fn get_similar_threads(
    thread_id: i32,
    forum_id: i32,
    current_tag_ids: &[i32],
    limit: usize,
) -> Result<Vec<SimilarThreadForTemplate>, sea_orm::DbErr> {
    use sea_orm::{DbBackend, Statement};

    if current_tag_ids.is_empty() {
        return Ok(Vec::new());
    }

    let db = get_db_pool();

    // Build a query that:
    // 1. Finds all threads that share at least one tag with the current thread
    // 2. Counts how many tags they share
    // 3. Excludes the current thread
    // 4. Orders by number of matching tags (descending), then by recency
    // 5. Limits to specified count

    // Build the IN clause for tag_ids
    let tag_ids_str = current_tag_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let sql = format!(
        r#"
        SELECT
            t.id,
            t.title,
            t.post_count,
            COUNT(tt.tag_id) as matching_tags
        FROM threads t
        INNER JOIN thread_tags tt ON tt.thread_id = t.id
        WHERE tt.tag_id IN ({})
          AND t.id != $1
          AND t.forum_id = $2
        GROUP BY t.id, t.title, t.post_count
        ORDER BY matching_tags DESC, t.last_post_at DESC NULLS LAST
        LIMIT $3
        "#,
        tag_ids_str
    );

    #[derive(Debug, sea_orm::FromQueryResult)]
    struct SimilarThreadRow {
        id: i32,
        title: String,
        post_count: i32,
        matching_tags: i64,
    }

    let rows = SimilarThreadRow::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        &sql,
        vec![thread_id.into(), forum_id.into(), (limit as i32).into()],
    ))
    .all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| SimilarThreadForTemplate {
            id: r.id,
            title: r.title,
            post_count: r.post_count,
            matching_tags: r.matching_tags as i32,
        })
        .collect())
}

/// Returns a Responder for a thread at a specific page.
async fn get_thread_and_replies_for_page(
    client: ClientCtx,
    thread_id: i32,
    page: i32,
) -> Result<impl Responder, Error> {
    use super::post::get_replies_and_author_for_template;
    use crate::attachment::get_attachments_for_ugc_by_id;
    use crate::orm::forums;

    let db = get_db_pool();
    let thread = Thread::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Thread not found."))?;
    let forum = forums::Entity::find_by_id(thread.forum_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Forum not found."))?;

    // Get user's posts per page preference
    let posts_per_page = if let Some(user_id) = client.get_id() {
        use crate::orm::users;
        users::Entity::find_by_id(user_id)
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|u| u.posts_per_page)
            .unwrap_or(DEFAULT_POSTS_PER_PAGE)
    } else {
        DEFAULT_POSTS_PER_PAGE
    };

    // Update thread to include views.
    let db_clone = db;
    actix_web::rt::spawn(async move {
        Thread::update_many()
            .col_expr(
                threads::Column::ViewCount,
                Expr::value(thread.view_count + 1),
            )
            .filter(threads::Column::Id.eq(thread_id))
            .exec(db_clone)
            .await
    });

    // Load posts, their ugc associations, and their living revision.
    let posts = get_replies_and_author_for_template(db, thread_id, page, posts_per_page)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let attachments =
        get_attachments_for_ugc_by_id(posts.iter().map(|p| p.0.ugc_id).collect()).await;

    // Check if user is watching this thread and email preference
    let (is_watching, email_on_reply) = if let Some(user_id) = client.get_id() {
        // Update thread read timestamp (async, don't wait)
        let db_for_read = get_db_pool();
        actix_web::rt::spawn(async move {
            let _ = update_thread_read(user_id, thread_id, db_for_read).await;
        });

        match crate::notifications::get_watch_status(user_id, thread_id).await {
            Ok(Some(watch)) => (true, watch.email_on_reply),
            _ => (false, false),
        }
    } else {
        (false, false)
    };

    let paginator = Paginator {
        base_url: format!("/threads/{}/", thread_id),
        this_page: page,
        page_count: get_pages_in_thread(thread.post_count, posts_per_page),
    };

    // Build breadcrumbs (including parent forums)
    let mut breadcrumbs = super::forum::build_forum_breadcrumbs(&forum).await;
    // Change last item (forum) to have a link since we're in thread view
    if let Some(last) = breadcrumbs.last_mut() {
        last.url = Some(format!("/forums/{}/", forum.id));
    }
    // Add thread as current page
    breadcrumbs.push(Breadcrumb {
        title: thread.title.clone(),
        url: None, // Current page, no link
    });

    // Fetch poll if exists
    let poll = get_poll_for_thread(thread_id, client.get_id())
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Fetch tags for this thread
    let tags = get_tags_for_thread(thread_id)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Fetch similar threads based on shared tags
    let tag_ids: Vec<i32> = tags.iter().map(|t| t.id).collect();
    let similar_threads = get_similar_threads(thread_id, thread.forum_id, &tag_ids, 5)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(ThreadTemplate {
        client,
        forum,
        thread,
        posts: &posts,
        paginator,
        attachments: &attachments,
        is_watching,
        email_on_reply,
        breadcrumbs,
        poll,
        tags,
        similar_threads,
    }
    .to_response())
}

/// Updates the post_count and last_post information on a thread.
/// This DOES NOT update post positions. It only updates the thread.
pub async fn update_thread_after_reply_is_deleted(id: i32) -> Result<(), DbErr> {
    #[derive(Debug, FromQueryResult)]
    struct LastPost {
        id: i32,
        created_at: chrono::NaiveDateTime,
    }

    let db = get_db_pool();

    let last_post_query = Post::find()
        .select_only()
        .column_as(posts::Column::Id, "id")
        .column_as(posts::Column::CreatedAt, "created_at")
        .left_join(ugc_deletions::Entity)
        .filter(posts::Column::ThreadId.eq(id))
        .filter(ugc_deletions::Column::DeletedAt.is_null())
        .into_model::<LastPost>()
        .one(db);

    let post_count_query = Post::find()
        .left_join(ugc_deletions::Entity)
        .filter(posts::Column::ThreadId.eq(id))
        .filter(ugc_deletions::Column::DeletedAt.is_null())
        .into_model::<LastPost>()
        .count(db);

    let (last_post_res, post_count_res) = futures::join!(last_post_query, post_count_query);

    let post_count = match post_count_res {
        Ok(count) => count,
        Err(err) => {
            log::error!("post_count error in update_thread: {:#?}", err);
            return Err(err);
        }
    };

    match last_post_res {
        Err(err) => {
            log::error!("last_post error in update_thread: {:#?}", err);
            return Err(err);
        }
        Ok(Some(last_post)) => {
            if let Err(err) = Thread::update_many()
                .col_expr(threads::Column::PostCount, Expr::value(post_count as i32))
                .col_expr(threads::Column::LastPostId, Expr::value(last_post.id))
                .col_expr(
                    threads::Column::LastPostAt,
                    Expr::value(last_post.created_at),
                )
                .exec(db)
                .await
            {
                log::error!("update query error in update_thread: {:#?}", err);
                return Err(err);
            }
        }
        Ok(None) => {
            log::error!("thread has no last_post when trying to update thread.");
        }
    }

    Ok(())
}

#[post("/threads/{thread_id}/post-reply")]
pub async fn create_reply(
    req: actix_web::HttpRequest,
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<(i32,)>,
    mutipart: Option<Multipart>,
) -> Result<impl Responder, Error> {
    // Require authentication for posting replies
    let authenticated_user_id = client.require_login()?;

    // Extract and store IP address for moderation
    let ip_id = if let Some(ip_addr) = crate::ip::extract_client_ip(&req) {
        crate::ip::get_or_create_ip_id(&ip_addr)
            .await
            .map_err(error::ErrorInternalServerError)?
    } else {
        None
    };

    // Rate limiting - prevent post spam
    if let Err(e) = crate::rate_limit::check_post_rate_limit(authenticated_user_id) {
        log::warn!(
            "Rate limit exceeded for post creation: user_id={}",
            authenticated_user_id
        );
        return Err(error::ErrorTooManyRequests(format!(
            "You're posting too quickly. Please wait {} seconds.",
            e.retry_after_seconds
        )));
    }

    use crate::filesystem::{insert_field_as_attachment, UploadResponse};
    use crate::orm::{posts, threads, ugc_attachments};
    use crate::ugc::{create_ugc, NewUgcPartial};
    use futures::{future::try_join_all, StreamExt, TryStreamExt};

    let mut content: String = String::new();
    let mut uploads: Vec<(_, UploadResponse)> = Vec::new();
    let mut csrf_token: Option<String> = None;

    // interpret user input
    // iterate over multipart stream
    if let Some(mut fields) = mutipart {
        while let Ok(Some(mut field)) = fields.try_next().await {
            if let Some(field_name) = field.content_disposition().get_name() {
                match field_name {
                    "csrf_token" => {
                        let mut buf: Vec<u8> = Vec::with_capacity(128);
                        while let Some(chunk) = field.next().await {
                            let bytes = chunk.map_err(|e| {
                                log::error!("create_reply: multipart read error: {}", e);
                                actix_web::error::ErrorBadRequest("Error interpreting user input.")
                            })?;
                            buf.extend(bytes.to_owned());
                        }
                        csrf_token = Some(str::from_utf8(&buf).unwrap().to_owned());
                    }
                    "content" => {
                        // Stream multipart data to string.
                        // Note: Post size is validated after reading (see validation below)
                        let mut buf: Vec<u8> = Vec::with_capacity(65536);

                        while let Some(chunk) = field.next().await {
                            let bytes = chunk.map_err(|e| {
                                log::error!("create_reply: multipart read error: {}", e);
                                actix_web::error::ErrorBadRequest("Error interpreting user input.")
                            })?;

                            buf.extend(bytes.to_owned());
                        }

                        content = str::from_utf8(&buf).unwrap().to_owned();
                    }
                    "attachment" => {
                        if let Some(payload) = insert_field_as_attachment(&mut field).await? {
                            let filename = field
                                .content_disposition()
                                .get_filename()
                                .unwrap_or(&payload.filename)
                                .to_owned();
                            uploads.push((filename, payload))
                        }
                    }
                    _ => {
                        return Err(error::ErrorBadRequest(format!(
                            "Unrecognized field '{}'",
                            field_name,
                        )));
                    }
                }
            }
        }
    }

    // Validate CSRF token
    let token = csrf_token.ok_or_else(|| error::ErrorBadRequest("CSRF token missing"))?;
    crate::middleware::csrf::validate_csrf_token(&cookies, &token)?;

    // Validate post size
    let max_length = crate::constants::MAX_POST_LENGTH;
    if content.len() > max_length {
        return Err(error::ErrorBadRequest(format!(
            "Post is too long. Maximum length is {} characters, but your post is {} characters.",
            max_length,
            content.len()
        )));
    }

    // Spam detection
    let user_post_count = posts::Entity::find()
        .filter(posts::Column::UserId.eq(authenticated_user_id))
        .count(get_db_pool())
        .await
        .unwrap_or(0) as i32;

    let spam_result = crate::spam::analyze_content(&content, user_post_count);
    if spam_result.is_spam {
        log::warn!(
            "Spam detected: user_id={}, score={:.2}, reasons={:?}",
            authenticated_user_id,
            spam_result.score,
            spam_result.reasons
        );
        return Err(error::ErrorBadRequest(
            "Your post has been flagged as potential spam. Please revise your content.",
        ));
    }

    // Begin Transaction
    let db = get_db_pool();
    let txn = db.begin().await.map_err(error::ErrorInternalServerError)?;

    let thread_id = path.into_inner().0;
    let our_thread = Thread::find_by_id(thread_id)
        .one(&txn)
        .await
        .map_err(|_| error::ErrorInternalServerError("Could not look up thread."))?
        .ok_or_else(|| error::ErrorNotFound("Thread not found."))?;

    // Check if thread is locked
    if our_thread.is_locked {
        return Err(error::ErrorForbidden(
            "This thread is locked and no longer accepting replies.",
        ));
    }

    // Insert ugc and first revision
    let ugc_revision = create_ugc(
        &txn,
        NewUgcPartial {
            ip_id,
            user_id: Some(authenticated_user_id),
            content: &content,
        },
    )
    .await
    .map_err(error::ErrorInternalServerError)?;

    // Insert post
    let new_post = posts::ActiveModel {
        thread_id: Set(our_thread.id),
        user_id: Set(ugc_revision.user_id),
        ugc_id: Set(ugc_revision.ugc_id),
        created_at: Set(ugc_revision.created_at),
        position: Set(our_thread.post_count + 1),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(error::ErrorInternalServerError)?;

    // Insert attachments, if any.
    if !uploads.is_empty() {
        try_join_all(uploads.iter().map(|u| {
            ugc_attachments::ActiveModel {
                attachment_id: Set(u.1.id),
                ugc_id: Set(ugc_revision.ugc_id),
                ip_id: Set(ip_id),
                user_id: Set(ugc_revision.user_id),
                created_at: Set(ugc_revision.created_at),
                filename: Set(u.0.to_owned()),
                ..Default::default()
            }
            .insert(&txn)
        }))
        .await
        .map_err(error::ErrorInternalServerError)?;
    }

    // Commit transaction
    txn.commit()
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Update thread
    let post_id = new_post.id;
    threads::Entity::update_many()
        .col_expr(
            threads::Column::PostCount,
            Expr::value(our_thread.post_count + 1),
        )
        .col_expr(threads::Column::LastPostId, Expr::value(post_id))
        .col_expr(
            threads::Column::LastPostAt,
            Expr::value(new_post.created_at),
        )
        .filter(threads::Column::Id.eq(thread_id))
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Send notifications asynchronously (don't block on errors)
    let post_content = content.clone();
    actix::spawn(async move {
        // Detect and notify mentions
        if let Err(e) = crate::notifications::dispatcher::detect_and_notify_mentions(
            &post_content,
            post_id,
            thread_id,
            authenticated_user_id,
        )
        .await
        {
            log::error!("Failed to send mention notifications: {}", e);
        }

        // Notify thread participants
        if let Err(e) = crate::notifications::dispatcher::notify_thread_reply(
            thread_id,
            post_id,
            authenticated_user_id,
        )
        .await
        {
            log::error!("Failed to send thread reply notifications: {}", e);
        }
    });

    Ok(HttpResponse::Found()
        .append_header((
            "Location",
            get_url_for_pos(our_thread.id, our_thread.post_count + 1),
        ))
        .finish())
}

/// Redirect to first unread post in a thread
#[get("/threads/{thread_id}/unread")]
pub async fn view_thread_unread(
    client: ClientCtx,
    path: web::Path<i32>,
) -> Result<impl Responder, Error> {
    let thread_id = path.into_inner();

    // Check if user is logged in
    if let Some(user_id) = client.get_id() {
        // Try to find first unread post
        match get_first_unread_post_id(user_id, thread_id).await {
            Ok(Some(post_id)) => {
                // Redirect to the specific unread post
                return Ok(HttpResponse::Found()
                    .append_header((
                        "Location",
                        format!("/threads/{}/post-{}", thread_id, post_id),
                    ))
                    .finish());
            }
            Ok(None) => {
                // No unread posts or never read, go to first page
            }
            Err(_) => {
                // On error, just go to thread start
            }
        }
    }

    // Default: redirect to thread start
    Ok(HttpResponse::Found()
        .append_header(("Location", format!("/threads/{}/", thread_id)))
        .finish())
}

#[get("/threads/{thread_id}/")]
pub async fn view_thread(client: ClientCtx, path: web::Path<i32>) -> Result<impl Responder, Error> {
    get_thread_and_replies_for_page(client, path.into_inner(), 1).await
}

#[get("/threads/{thread_id}/page-{page}")]
pub async fn view_thread_page(
    client: ClientCtx,
    path: web::Path<(i32, i32)>,
) -> Result<impl Responder, Error> {
    let params = path.into_inner();
    if params.1 > 1 {
        get_thread_and_replies_for_page(client, params.0, params.1).await
    } else {
        get_thread_and_replies_for_page(client, params.0, 1).await
        //Ok(HttpResponse::Found()
        //    .append_header(("Location", format!("/threads/{}/", params.0)))
        //    .finish())
    }
}

pub fn validate_thread_form(
    form: web::Form<NewThreadFormData>,
) -> Result<(NewThreadFormData, Option<ValidatedPoll>), Error> {
    let title = form.title.trim().to_owned();
    let subtitle = form.subtitle.to_owned().filter(|x| !x.is_empty());

    if title.is_empty() {
        return Err(error::ErrorUnprocessableEntity(
            "Threads must have a title.",
        ));
    }

    // Validate post content size
    let max_length = crate::constants::MAX_POST_LENGTH;
    if form.content.len() > max_length {
        return Err(error::ErrorBadRequest(format!(
            "Post is too long. Maximum length is {} characters, but your post is {} characters.",
            max_length,
            form.content.len()
        )));
    }

    // Validate and normalize tags
    let tags: Vec<String> = form
        .tags
        .iter()
        .flat_map(|t| t.split(','))
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty() && t.len() <= 50)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .take(10) // Max 10 tags per thread
        .collect();

    // Validate poll if question is provided
    let validated_poll = if let Some(ref question) = form.poll_question {
        let question = question.trim();
        if !question.is_empty() {
            // Validate question length
            if question.len() > 500 {
                return Err(error::ErrorBadRequest(
                    "Poll question must be 500 characters or less.",
                ));
            }

            // Filter out empty options and validate
            let options: Vec<String> = form
                .poll_options
                .iter()
                .map(|o| o.trim().to_owned())
                .filter(|o| !o.is_empty())
                .collect();

            if options.len() < 2 {
                return Err(error::ErrorBadRequest("Poll must have at least 2 options."));
            }

            if options.len() > 20 {
                return Err(error::ErrorBadRequest(
                    "Poll cannot have more than 20 options.",
                ));
            }

            // Validate each option length
            for opt in &options {
                if opt.len() > 200 {
                    return Err(error::ErrorBadRequest(
                        "Each poll option must be 200 characters or less.",
                    ));
                }
            }

            // Validate max_choices
            let max_choices = form.poll_max_choices.clamp(1, options.len() as i32);

            // Parse closes_at if provided
            let closes_at = if let Some(ref closes_str) = form.poll_closes_at {
                let closes_str = closes_str.trim();
                if !closes_str.is_empty() {
                    Some(
                        chrono::NaiveDateTime::parse_from_str(closes_str, "%Y-%m-%dT%H:%M")
                            .map_err(|_| {
                                error::ErrorBadRequest("Invalid poll closing date format.")
                            })?,
                    )
                } else {
                    None
                }
            } else {
                None
            };

            Some(ValidatedPoll {
                question: question.to_owned(),
                options,
                max_choices,
                allow_change_vote: form.poll_allow_change_vote,
                show_results_before_vote: form.poll_show_results_before_vote,
                closes_at,
            })
        } else {
            None
        }
    } else {
        None
    };

    Ok((
        NewThreadFormData {
            title,
            subtitle,
            content: form.content.to_owned(),
            csrf_token: form.csrf_token.to_owned(),
            tags,
            poll_question: form.poll_question.clone(),
            poll_options: form.poll_options.clone(),
            poll_max_choices: form.poll_max_choices,
            poll_allow_change_vote: form.poll_allow_change_vote,
            poll_show_results_before_vote: form.poll_show_results_before_vote,
            poll_closes_at: form.poll_closes_at.clone(),
        },
        validated_poll,
    ))
}

/// Update the thread_read timestamp for a user viewing a thread
async fn update_thread_read(
    user_id: i32,
    thread_id: i32,
    db: &sea_orm::DatabaseConnection,
) -> Result<(), DbErr> {
    let now = chrono::Utc::now().naive_utc();

    // Delete existing record if any
    thread_read::Entity::delete_many()
        .filter(thread_read::Column::UserId.eq(user_id))
        .filter(thread_read::Column::ThreadId.eq(thread_id))
        .exec(db)
        .await?;

    // Insert new record
    let record = thread_read::ActiveModel {
        user_id: Set(user_id),
        thread_id: Set(thread_id),
        read_at: Set(now),
    };
    thread_read::Entity::insert(record).exec(db).await?;

    Ok(())
}

/// Get the first unread post ID in a thread for a user
/// Returns None if all posts are read or user hasn't viewed thread before
pub async fn get_first_unread_post_id(user_id: i32, thread_id: i32) -> Result<Option<i32>, DbErr> {
    use sea_orm::{DbBackend, Statement};

    let db = get_db_pool();

    // First check if user has a read record for this thread
    let read_record = thread_read::Entity::find()
        .filter(thread_read::Column::UserId.eq(user_id))
        .filter(thread_read::Column::ThreadId.eq(thread_id))
        .one(db)
        .await?;

    let read_at = match read_record {
        Some(r) => r.read_at,
        None => return Ok(None), // Never read this thread, go to beginning
    };

    // Find the first post created after the read timestamp
    let sql = r#"
        SELECT p.id
        FROM posts p
        WHERE p.thread_id = $1
          AND p.created_at > $2
          AND p.deleted_at IS NULL
        ORDER BY p.position ASC
        LIMIT 1
    "#;

    #[derive(Debug, FromQueryResult)]
    struct PostId {
        id: i32,
    }

    let result = PostId::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [thread_id.into(), read_at.into()],
    ))
    .one(db)
    .await?;

    Ok(result.map(|r| r.id))
}

/// Check if a thread has unread posts for a user
pub async fn has_unread_posts(
    user_id: i32,
    thread_id: i32,
    last_post_at: Option<chrono::NaiveDateTime>,
) -> bool {
    let last_post = match last_post_at {
        Some(t) => t,
        None => return false, // No posts in thread
    };

    let db = get_db_pool();

    let read_record = thread_read::Entity::find()
        .filter(thread_read::Column::UserId.eq(user_id))
        .filter(thread_read::Column::ThreadId.eq(thread_id))
        .one(db)
        .await
        .ok()
        .flatten();

    match read_record {
        Some(r) => last_post > r.read_at, // Unread if last post is newer than read time
        None => true,                     // Never read this thread
    }
}

/// Form data for thread moderation actions
#[derive(Deserialize)]
pub struct ThreadModActionFormData {
    pub csrf_token: String,
    #[serde(default)]
    pub reason: Option<String>,
    /// Deletion type: "normal" or "permanent"
    #[serde(default)]
    pub deletion_type: Option<String>,
}

/// Delete a thread (moderators only)
#[post("/threads/{thread_id}/delete")]
pub async fn delete_thread(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<ThreadModActionFormData>,
) -> Result<impl Responder, Error> {
    use crate::orm::ugc_deletions::DeletionType;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    let db = get_db_pool();
    let thread_id = path.into_inner();

    let thread = Thread::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Thread not found."))?;

    // Determine deletion type and check permissions
    let deletion_type = match form.deletion_type.as_deref() {
        Some("permanent") => {
            if !client.can("moderate.thread.delete_permanent") {
                return Err(error::ErrorForbidden(
                    "You do not have permission to permanently delete threads.",
                ));
            }
            DeletionType::Permanent
        }
        _ => {
            if !client.can("moderate.thread.delete_any") {
                return Err(error::ErrorForbidden(
                    "You do not have permission to delete threads.",
                ));
            }
            DeletionType::Normal
        }
    };

    // Check if thread is under legal hold
    if thread.deletion_type == Some(DeletionType::LegalHold) {
        return Err(error::ErrorForbidden(
            "This thread is under legal hold and cannot be deleted.",
        ));
    }

    let now = chrono::Utc::now().naive_utc();

    // Update thread with deletion info
    Thread::update_many()
        .col_expr(threads::Column::DeletedAt, Expr::value(now))
        .col_expr(threads::Column::DeletedBy, Expr::value(client.get_id()))
        .col_expr(threads::Column::DeletionType, Expr::value(deletion_type.clone()))
        .col_expr(threads::Column::DeletionReason, Expr::value(form.reason.clone()))
        .filter(threads::Column::Id.eq(thread_id))
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    // For permanent deletion, also purge all post content
    if deletion_type == DeletionType::Permanent {
        use crate::orm::ugc_revisions;

        // Get all UGC IDs for posts in this thread
        let post_ugc_ids: Vec<i32> = posts::Entity::find()
            .filter(posts::Column::ThreadId.eq(thread_id))
            .all(db)
            .await
            .map_err(error::ErrorInternalServerError)?
            .into_iter()
            .map(|p| p.ugc_id)
            .collect();

        // Clear content from all revisions
        if !post_ugc_ids.is_empty() {
            ugc_revisions::Entity::update_many()
                .col_expr(ugc_revisions::Column::Content, Expr::value("[Content permanently removed]".to_string()))
                .filter(ugc_revisions::Column::UgcId.is_in(post_ugc_ids))
                .exec(db)
                .await
                .map_err(error::ErrorInternalServerError)?;
        }
    }

    Ok(HttpResponse::Found()
        .append_header(("Location", format!("/forums/{}/", thread.forum_id)))
        .finish())
}

/// Restore a deleted thread (moderators only)
#[post("/threads/{thread_id}/restore")]
pub async fn restore_thread(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<ThreadModActionFormData>,
) -> Result<impl Responder, Error> {
    use crate::orm::ugc_deletions::DeletionType;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    if !client.can("moderate.thread.restore") {
        return Err(error::ErrorForbidden(
            "You do not have permission to restore threads.",
        ));
    }

    let db = get_db_pool();
    let thread_id = path.into_inner();

    let thread = Thread::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Thread not found."))?;

    // Check if thread is deleted
    if thread.deleted_at.is_none() {
        return Err(error::ErrorBadRequest("Thread is not deleted."));
    }

    // Cannot restore permanently deleted threads
    if thread.deletion_type == Some(DeletionType::Permanent) {
        return Err(error::ErrorForbidden(
            "Permanently deleted threads cannot be restored.",
        ));
    }

    // Cannot restore threads under legal hold
    if thread.deletion_type == Some(DeletionType::LegalHold) {
        return Err(error::ErrorForbidden(
            "Threads under legal hold cannot be restored. Contact an administrator.",
        ));
    }

    // Clear deletion fields
    Thread::update_many()
        .col_expr(threads::Column::DeletedAt, Expr::value(Option::<chrono::NaiveDateTime>::None))
        .col_expr(threads::Column::DeletedBy, Expr::value(Option::<i32>::None))
        .col_expr(threads::Column::DeletionType, Expr::value(Option::<DeletionType>::None))
        .col_expr(threads::Column::DeletionReason, Expr::value(Option::<String>::None))
        .filter(threads::Column::Id.eq(thread_id))
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", format!("/threads/{}/", thread_id)))
        .finish())
}

/// Place a legal hold on a thread (admin only)
#[post("/threads/{thread_id}/legal-hold")]
pub async fn legal_hold_thread(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<ThreadModActionFormData>,
) -> Result<impl Responder, Error> {
    use crate::orm::ugc_deletions::DeletionType;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    if !client.can("admin.content.legal_hold") {
        return Err(error::ErrorForbidden(
            "You do not have permission to place legal holds.",
        ));
    }

    let db = get_db_pool();
    let thread_id = path.into_inner();

    let _thread = Thread::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Thread not found."))?;

    let now = chrono::Utc::now().naive_utc();

    // Update thread with legal hold
    Thread::update_many()
        .col_expr(threads::Column::DeletedAt, Expr::value(now))
        .col_expr(threads::Column::DeletedBy, Expr::value(client.get_id()))
        .col_expr(threads::Column::DeletionType, Expr::value(DeletionType::LegalHold))
        .col_expr(threads::Column::DeletionReason, Expr::value(Some("Legal hold".to_string())))
        .col_expr(threads::Column::LegalHoldAt, Expr::value(now))
        .col_expr(threads::Column::LegalHoldBy, Expr::value(client.get_id()))
        .col_expr(threads::Column::LegalHoldReason, Expr::value(form.reason.clone()))
        .filter(threads::Column::Id.eq(thread_id))
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", format!("/threads/{}/", thread_id)))
        .finish())
}

/// Remove a legal hold from a thread (admin only)
#[post("/threads/{thread_id}/remove-legal-hold")]
pub async fn remove_legal_hold_thread(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<ThreadModActionFormData>,
) -> Result<impl Responder, Error> {
    use crate::orm::ugc_deletions::DeletionType;

    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    if !client.can("admin.content.remove_legal_hold") {
        return Err(error::ErrorForbidden(
            "You do not have permission to remove legal holds.",
        ));
    }

    let db = get_db_pool();
    let thread_id = path.into_inner();

    let thread = Thread::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Thread not found."))?;

    // Check if thread is under legal hold
    if thread.deletion_type != Some(DeletionType::LegalHold) {
        return Err(error::ErrorBadRequest("Thread is not under legal hold."));
    }

    // Change to normal deletion (still deleted, but can now be restored)
    Thread::update_many()
        .col_expr(threads::Column::DeletionType, Expr::value(DeletionType::Normal))
        .col_expr(threads::Column::LegalHoldAt, Expr::value(Option::<chrono::NaiveDateTime>::None))
        .col_expr(threads::Column::LegalHoldBy, Expr::value(Option::<i32>::None))
        .col_expr(threads::Column::LegalHoldReason, Expr::value(Option::<String>::None))
        .filter(threads::Column::Id.eq(thread_id))
        .exec(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", format!("/threads/{}/", thread_id)))
        .finish())
}
