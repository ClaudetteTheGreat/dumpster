use actix_web::{get, web, HttpResponse, Responder};
use rss::{ChannelBuilder, GuidBuilder, ItemBuilder};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::orm::{forums, threads, ugc, ugc_revisions};

const FEED_ITEM_LIMIT: u64 = 25;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(latest_threads_feed)
        .service(forum_feed);
}

/// RSS feed for latest threads across all forums
#[get("/feed.rss")]
pub async fn latest_threads_feed(db: web::Data<DatabaseConnection>) -> impl Responder {
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

    let site_url = std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

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
            .pub_date(Some(thread.created_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string()))
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

    HttpResponse::Ok()
        .content_type("application/rss+xml; charset=utf-8")
        .body(channel.to_string())
}

/// RSS feed for threads in a specific forum
#[get("/forums/{id}/feed.rss")]
pub async fn forum_feed(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    let db = db.get_ref();
    let forum_id = path.into_inner();

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

    let site_url = std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

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
            .pub_date(Some(thread.created_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string()))
            .guid(Some(guid))
            .build();

        items.push(item);
    }

    let channel = ChannelBuilder::default()
        .title(format!("{} - Latest Threads", forum.label))
        .link(format!("{}/forums/{}", site_url, forum_id))
        .description(forum.description.unwrap_or_else(|| format!("Latest threads from {}", forum.label)))
        .items(items)
        .build();

    HttpResponse::Ok()
        .content_type("application/rss+xml; charset=utf-8")
        .body(channel.to_string())
}

/// Get post content from UGC system
async fn get_post_content(db: &DatabaseConnection, post_id: i32) -> Option<String> {
    use crate::orm::posts;

    // Get post -> ugc -> ugc_revision
    let post = posts::Entity::find_by_id(post_id).one(db).await.ok()??;
    let ugc = ugc::Entity::find_by_id(post.ugc_id).one(db).await.ok()??;
    let revision_id = ugc.ugc_revision_id?;
    let revision = ugc_revisions::Entity::find_by_id(revision_id).one(db).await.ok()??;

    Some(revision.content)
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
