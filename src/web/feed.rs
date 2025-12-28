use actix_web::{get, web, HttpResponse, Responder};
use atom_syndication::{
    ContentBuilder, EntryBuilder, FeedBuilder as AtomFeedBuilder, LinkBuilder, TextBuilder,
};
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use rss::{ChannelBuilder, GuidBuilder, ItemBuilder};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::time::{Duration, Instant};

use crate::orm::{forums, posts, threads, ugc, ugc_revisions, user_names};

const FEED_ITEM_LIMIT: u64 = 25;
const FEED_CACHE_TTL_SECS: u64 = 300; // 5 minutes

/// Cached feed entry with content and timestamp
struct CachedFeed {
    content: String,
    cached_at: Instant,
}

/// Global feed cache
static FEED_CACHE: Lazy<DashMap<String, CachedFeed>> = Lazy::new(DashMap::new);

/// Get cached feed if valid, otherwise return None
fn get_cached_feed(key: &str) -> Option<String> {
    if let Some(entry) = FEED_CACHE.get(key) {
        if entry.cached_at.elapsed() < Duration::from_secs(FEED_CACHE_TTL_SECS) {
            return Some(entry.content.clone());
        }
        // Cache expired, will be replaced
    }
    None
}

/// Store feed in cache
fn cache_feed(key: String, content: String) {
    FEED_CACHE.insert(
        key,
        CachedFeed {
            content,
            cached_at: Instant::now(),
        },
    );
}

/// Clear all cached feeds (useful for testing)
#[allow(dead_code)]
pub fn clear_feed_cache() {
    FEED_CACHE.clear();
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(latest_threads_feed)
        .service(forum_feed)
        .service(latest_threads_atom_feed)
        .service(forum_atom_feed)
        .service(thread_feed)
        .service(thread_atom_feed);
}

/// RSS feed for latest threads across all forums
#[get("/feed.rss")]
pub async fn latest_threads_feed(db: web::Data<DatabaseConnection>) -> impl Responder {
    let cache_key = "rss:latest".to_string();

    // Check cache first
    if let Some(cached) = get_cached_feed(&cache_key) {
        return HttpResponse::Ok()
            .content_type("application/rss+xml; charset=utf-8")
            .body(cached);
    }

    let db = db.get_ref();

    // Get latest threads with their first post content
    let threads = match threads::Entity::find()
        .order_by_desc(threads::Column::CreatedAt)
        .limit(FEED_ITEM_LIMIT)
        .all(db)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to fetch threads for feed: {}", e);
            return HttpResponse::InternalServerError().body("Failed to generate feed");
        }
    };

    let site_url =
        std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    let mut items = Vec::new();
    for thread in threads {
        // Get the first post content if available
        let description = if let Some(post_id) = thread.first_post_id {
            get_post_content(db, post_id).await.unwrap_or_default()
        } else {
            String::new()
        };

        let link = format!("{}/threads/{}", site_url, thread.id);
        let guid = GuidBuilder::default()
            .value(link.clone())
            .permalink(true)
            .build();

        let item = ItemBuilder::default()
            .title(Some(thread.title))
            .link(Some(link))
            .description(Some(truncate_content(&description, 500)))
            .pub_date(Some(
                thread
                    .created_at
                    .format("%a, %d %b %Y %H:%M:%S GMT")
                    .to_string(),
            ))
            .guid(Some(guid))
            .build();

        items.push(item);
    }

    let channel = ChannelBuilder::default()
        .title("Forum - Latest Threads")
        .link(site_url.clone())
        .description("Latest threads from the forum")
        .items(items)
        .build();

    let content = channel.to_string();
    cache_feed(cache_key, content.clone());

    HttpResponse::Ok()
        .content_type("application/rss+xml; charset=utf-8")
        .body(content)
}

/// RSS feed for threads in a specific forum
#[get("/forums/{id}/feed.rss")]
pub async fn forum_feed(db: web::Data<DatabaseConnection>, path: web::Path<i32>) -> impl Responder {
    let forum_id = path.into_inner();
    let cache_key = format!("rss:forum:{}", forum_id);

    // Check cache first
    if let Some(cached) = get_cached_feed(&cache_key) {
        return HttpResponse::Ok()
            .content_type("application/rss+xml; charset=utf-8")
            .body(cached);
    }

    let db = db.get_ref();

    // Get forum info
    let forum = match forums::Entity::find_by_id(forum_id).one(db).await {
        Ok(Some(f)) => f,
        Ok(None) => return HttpResponse::NotFound().body("Forum not found"),
        Err(e) => {
            log::error!("Failed to fetch forum: {}", e);
            return HttpResponse::InternalServerError().body("Failed to generate feed");
        }
    };

    // Get latest threads in this forum
    let threads = match threads::Entity::find()
        .filter(threads::Column::ForumId.eq(forum_id))
        .order_by_desc(threads::Column::CreatedAt)
        .limit(FEED_ITEM_LIMIT)
        .all(db)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to fetch threads for forum feed: {}", e);
            return HttpResponse::InternalServerError().body("Failed to generate feed");
        }
    };

    let site_url =
        std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    let mut items = Vec::new();
    for thread in threads {
        let description = if let Some(post_id) = thread.first_post_id {
            get_post_content(db, post_id).await.unwrap_or_default()
        } else {
            String::new()
        };

        let link = format!("{}/threads/{}", site_url, thread.id);
        let guid = GuidBuilder::default()
            .value(link.clone())
            .permalink(true)
            .build();

        let item = ItemBuilder::default()
            .title(Some(thread.title))
            .link(Some(link))
            .description(Some(truncate_content(&description, 500)))
            .pub_date(Some(
                thread
                    .created_at
                    .format("%a, %d %b %Y %H:%M:%S GMT")
                    .to_string(),
            ))
            .guid(Some(guid))
            .build();

        items.push(item);
    }

    let channel = ChannelBuilder::default()
        .title(format!("{} - Latest Threads", forum.label))
        .link(format!("{}/forums/{}", site_url, forum_id))
        .description(
            forum
                .description
                .unwrap_or_else(|| format!("Latest threads from {}", forum.label)),
        )
        .items(items)
        .build();

    let content = channel.to_string();
    cache_feed(cache_key, content.clone());

    HttpResponse::Ok()
        .content_type("application/rss+xml; charset=utf-8")
        .body(content)
}

/// Atom feed for latest threads across all forums
#[get("/feed.atom")]
pub async fn latest_threads_atom_feed(db: web::Data<DatabaseConnection>) -> impl Responder {
    let cache_key = "atom:latest".to_string();

    // Check cache first
    if let Some(cached) = get_cached_feed(&cache_key) {
        return HttpResponse::Ok()
            .content_type("application/atom+xml; charset=utf-8")
            .body(cached);
    }

    let db = db.get_ref();

    let threads = match threads::Entity::find()
        .order_by_desc(threads::Column::CreatedAt)
        .limit(FEED_ITEM_LIMIT)
        .all(db)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to fetch threads for Atom feed: {}", e);
            return HttpResponse::InternalServerError().body("Failed to generate feed");
        }
    };

    let site_url =
        std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    let mut entries = Vec::new();
    let mut latest_updated: Option<DateTime<FixedOffset>> = None;

    for thread in threads {
        let description = if let Some(post_id) = thread.first_post_id {
            get_post_content(db, post_id).await.unwrap_or_default()
        } else {
            String::new()
        };

        let link = format!("{}/threads/{}", site_url, thread.id);
        let updated = naive_to_fixed_offset(thread.created_at);

        if latest_updated.is_none() || Some(updated) > latest_updated {
            latest_updated = Some(updated);
        }

        let entry = EntryBuilder::default()
            .id(link.clone())
            .title(TextBuilder::default().value(thread.title).build())
            .link(
                LinkBuilder::default()
                    .href(link)
                    .rel("alternate".to_string())
                    .build(),
            )
            .summary(Some(
                TextBuilder::default()
                    .value(truncate_content(&description, 500))
                    .build(),
            ))
            .content(Some(
                ContentBuilder::default()
                    .content_type(Some("html".to_string()))
                    .value(Some(description))
                    .build(),
            ))
            .updated(updated)
            .published(Some(updated))
            .build();

        entries.push(entry);
    }

    let feed = AtomFeedBuilder::default()
        .id(site_url.clone())
        .title(
            TextBuilder::default()
                .value("Forum - Latest Threads")
                .build(),
        )
        .link(
            LinkBuilder::default()
                .href(site_url.clone())
                .rel("alternate".to_string())
                .build(),
        )
        .link(
            LinkBuilder::default()
                .href(format!("{}/feed.atom", site_url))
                .rel("self".to_string())
                .mime_type(Some("application/atom+xml".to_string()))
                .build(),
        )
        .updated(latest_updated.unwrap_or_else(|| Utc::now().fixed_offset()))
        .entries(entries)
        .build();

    let content = feed.to_string();
    cache_feed(cache_key, content.clone());

    HttpResponse::Ok()
        .content_type("application/atom+xml; charset=utf-8")
        .body(content)
}

/// Atom feed for threads in a specific forum
#[get("/forums/{id}/feed.atom")]
pub async fn forum_atom_feed(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    let forum_id = path.into_inner();
    let cache_key = format!("atom:forum:{}", forum_id);

    // Check cache first
    if let Some(cached) = get_cached_feed(&cache_key) {
        return HttpResponse::Ok()
            .content_type("application/atom+xml; charset=utf-8")
            .body(cached);
    }

    let db = db.get_ref();

    let forum = match forums::Entity::find_by_id(forum_id).one(db).await {
        Ok(Some(f)) => f,
        Ok(None) => return HttpResponse::NotFound().body("Forum not found"),
        Err(e) => {
            log::error!("Failed to fetch forum: {}", e);
            return HttpResponse::InternalServerError().body("Failed to generate feed");
        }
    };

    let threads = match threads::Entity::find()
        .filter(threads::Column::ForumId.eq(forum_id))
        .order_by_desc(threads::Column::CreatedAt)
        .limit(FEED_ITEM_LIMIT)
        .all(db)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to fetch threads for forum Atom feed: {}", e);
            return HttpResponse::InternalServerError().body("Failed to generate feed");
        }
    };

    let site_url =
        std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    let mut entries = Vec::new();
    let mut latest_updated: Option<DateTime<FixedOffset>> = None;

    for thread in threads {
        let description = if let Some(post_id) = thread.first_post_id {
            get_post_content(db, post_id).await.unwrap_or_default()
        } else {
            String::new()
        };

        let link = format!("{}/threads/{}", site_url, thread.id);
        let updated = naive_to_fixed_offset(thread.created_at);

        if latest_updated.is_none() || Some(updated) > latest_updated {
            latest_updated = Some(updated);
        }

        let entry = EntryBuilder::default()
            .id(link.clone())
            .title(TextBuilder::default().value(thread.title).build())
            .link(
                LinkBuilder::default()
                    .href(link)
                    .rel("alternate".to_string())
                    .build(),
            )
            .summary(Some(
                TextBuilder::default()
                    .value(truncate_content(&description, 500))
                    .build(),
            ))
            .content(Some(
                ContentBuilder::default()
                    .content_type(Some("html".to_string()))
                    .value(Some(description))
                    .build(),
            ))
            .updated(updated)
            .published(Some(updated))
            .build();

        entries.push(entry);
    }

    let forum_url = format!("{}/forums/{}", site_url, forum_id);
    let feed = AtomFeedBuilder::default()
        .id(forum_url.clone())
        .title(
            TextBuilder::default()
                .value(format!("{} - Latest Threads", forum.label))
                .build(),
        )
        .subtitle(Some(
            TextBuilder::default()
                .value(
                    forum
                        .description
                        .unwrap_or_else(|| format!("Latest threads from {}", forum.label)),
                )
                .build(),
        ))
        .link(
            LinkBuilder::default()
                .href(forum_url)
                .rel("alternate".to_string())
                .build(),
        )
        .link(
            LinkBuilder::default()
                .href(format!("{}/forums/{}/feed.atom", site_url, forum_id))
                .rel("self".to_string())
                .mime_type(Some("application/atom+xml".to_string()))
                .build(),
        )
        .updated(latest_updated.unwrap_or_else(|| Utc::now().fixed_offset()))
        .entries(entries)
        .build();

    let content = feed.to_string();
    cache_feed(cache_key, content.clone());

    HttpResponse::Ok()
        .content_type("application/atom+xml; charset=utf-8")
        .body(content)
}

/// RSS feed for replies in a specific thread
#[get("/threads/{id}/feed.rss")]
pub async fn thread_feed(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    let thread_id = path.into_inner();
    let cache_key = format!("rss:thread:{}", thread_id);

    // Check cache first
    if let Some(cached) = get_cached_feed(&cache_key) {
        return HttpResponse::Ok()
            .content_type("application/rss+xml; charset=utf-8")
            .body(cached);
    }

    let db = db.get_ref();

    // Get thread info
    let thread = match threads::Entity::find_by_id(thread_id).one(db).await {
        Ok(Some(t)) => t,
        Ok(None) => return HttpResponse::NotFound().body("Thread not found"),
        Err(e) => {
            log::error!("Failed to fetch thread: {}", e);
            return HttpResponse::InternalServerError().body("Failed to generate feed");
        }
    };

    // Get latest posts in this thread (excluding the first post which is the OP)
    let thread_posts = match posts::Entity::find()
        .filter(posts::Column::ThreadId.eq(thread_id))
        .filter(posts::Column::Position.gt(1)) // Exclude OP (position 1)
        .order_by_desc(posts::Column::CreatedAt)
        .limit(FEED_ITEM_LIMIT)
        .all(db)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to fetch posts for thread feed: {}", e);
            return HttpResponse::InternalServerError().body("Failed to generate feed");
        }
    };

    let site_url =
        std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    let mut items = Vec::new();
    for post in thread_posts {
        let content = get_post_content(db, post.id).await.unwrap_or_default();
        let author = get_post_author(db, post.user_id).await;

        let link = format!("{}/threads/{}#post-{}", site_url, thread_id, post.id);
        let guid = GuidBuilder::default()
            .value(format!("post-{}", post.id))
            .permalink(false)
            .build();

        let mut item_builder = ItemBuilder::default();
        item_builder
            .title(Some(format!("Reply #{}", post.position)))
            .link(Some(link))
            .description(Some(truncate_content(&content, 500)))
            .pub_date(Some(
                post.created_at
                    .format("%a, %d %b %Y %H:%M:%S GMT")
                    .to_string(),
            ))
            .guid(Some(guid));

        if let Some(author_name) = author {
            item_builder.author(Some(author_name));
        }

        items.push(item_builder.build());
    }

    let channel = ChannelBuilder::default()
        .title(format!("{} - Replies", thread.title))
        .link(format!("{}/threads/{}", site_url, thread_id))
        .description(format!("Latest replies to: {}", thread.title))
        .items(items)
        .build();

    let content = channel.to_string();
    cache_feed(cache_key, content.clone());

    HttpResponse::Ok()
        .content_type("application/rss+xml; charset=utf-8")
        .body(content)
}

/// Atom feed for replies in a specific thread
#[get("/threads/{id}/feed.atom")]
pub async fn thread_atom_feed(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    let thread_id = path.into_inner();
    let cache_key = format!("atom:thread:{}", thread_id);

    // Check cache first
    if let Some(cached) = get_cached_feed(&cache_key) {
        return HttpResponse::Ok()
            .content_type("application/atom+xml; charset=utf-8")
            .body(cached);
    }

    let db = db.get_ref();

    // Get thread info
    let thread = match threads::Entity::find_by_id(thread_id).one(db).await {
        Ok(Some(t)) => t,
        Ok(None) => return HttpResponse::NotFound().body("Thread not found"),
        Err(e) => {
            log::error!("Failed to fetch thread: {}", e);
            return HttpResponse::InternalServerError().body("Failed to generate feed");
        }
    };

    // Get latest posts in this thread (excluding the first post which is the OP)
    let thread_posts = match posts::Entity::find()
        .filter(posts::Column::ThreadId.eq(thread_id))
        .filter(posts::Column::Position.gt(1)) // Exclude OP (position 1)
        .order_by_desc(posts::Column::CreatedAt)
        .limit(FEED_ITEM_LIMIT)
        .all(db)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to fetch posts for thread Atom feed: {}", e);
            return HttpResponse::InternalServerError().body("Failed to generate feed");
        }
    };

    let site_url =
        std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    let mut entries = Vec::new();
    let mut latest_updated: Option<DateTime<FixedOffset>> = None;

    for post in thread_posts {
        let content = get_post_content(db, post.id).await.unwrap_or_default();
        let author = get_post_author(db, post.user_id).await;

        let link = format!("{}/threads/{}#post-{}", site_url, thread_id, post.id);
        let updated = naive_to_fixed_offset(post.created_at);

        if latest_updated.is_none() || Some(updated) > latest_updated {
            latest_updated = Some(updated);
        }

        let mut entry_builder = EntryBuilder::default();
        entry_builder
            .id(format!("post-{}", post.id))
            .title(TextBuilder::default().value(format!("Reply #{}", post.position)).build())
            .link(
                LinkBuilder::default()
                    .href(link)
                    .rel("alternate".to_string())
                    .build(),
            )
            .summary(Some(
                TextBuilder::default()
                    .value(truncate_content(&content, 500))
                    .build(),
            ))
            .content(Some(
                ContentBuilder::default()
                    .content_type(Some("html".to_string()))
                    .value(Some(content))
                    .build(),
            ))
            .updated(updated)
            .published(Some(updated));

        if let Some(author_name) = author {
            entry_builder.authors(vec![atom_syndication::PersonBuilder::default()
                .name(author_name)
                .build()]);
        }

        entries.push(entry_builder.build());
    }

    let thread_url = format!("{}/threads/{}", site_url, thread_id);
    let feed = AtomFeedBuilder::default()
        .id(thread_url.clone())
        .title(
            TextBuilder::default()
                .value(format!("{} - Replies", thread.title))
                .build(),
        )
        .subtitle(Some(
            TextBuilder::default()
                .value(format!("Latest replies to: {}", thread.title))
                .build(),
        ))
        .link(
            LinkBuilder::default()
                .href(thread_url)
                .rel("alternate".to_string())
                .build(),
        )
        .link(
            LinkBuilder::default()
                .href(format!("{}/threads/{}/feed.atom", site_url, thread_id))
                .rel("self".to_string())
                .mime_type(Some("application/atom+xml".to_string()))
                .build(),
        )
        .updated(latest_updated.unwrap_or_else(|| Utc::now().fixed_offset()))
        .entries(entries)
        .build();

    let content = feed.to_string();
    cache_feed(cache_key, content.clone());

    HttpResponse::Ok()
        .content_type("application/atom+xml; charset=utf-8")
        .body(content)
}

/// Convert NaiveDateTime to DateTime<FixedOffset> (assuming UTC)
fn naive_to_fixed_offset(dt: NaiveDateTime) -> DateTime<FixedOffset> {
    DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc).fixed_offset()
}

/// Get post content from UGC system
async fn get_post_content(db: &DatabaseConnection, post_id: i32) -> Option<String> {
    use crate::orm::posts;

    // Get post -> ugc -> ugc_revision
    let post = posts::Entity::find_by_id(post_id).one(db).await.ok()??;
    let ugc = ugc::Entity::find_by_id(post.ugc_id).one(db).await.ok()??;
    let revision_id = ugc.ugc_revision_id?;
    let revision = ugc_revisions::Entity::find_by_id(revision_id)
        .one(db)
        .await
        .ok()??;

    Some(revision.content)
}

/// Get post author name from user_names table
async fn get_post_author(db: &DatabaseConnection, user_id: Option<i32>) -> Option<String> {
    let user_id = user_id?;

    // Get the username for this user
    let user_name = user_names::Entity::find_by_id(user_id)
        .one(db)
        .await
        .ok()??;

    Some(user_name.name)
}

/// Truncate content to a maximum length, adding ellipsis if truncated
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let truncated: String = content.chars().take(max_len).collect();
        format!("{}...", truncated.trim_end())
    }
}
