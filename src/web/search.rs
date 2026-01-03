/// Search functionality using PostgreSQL full-text search
///
/// This module provides search capabilities for threads and posts.
use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use actix_web::{error, get, web, Error, HttpRequest, Responder};
use askama_actix::{Template, TemplateToResponse};
use sea_orm::{DatabaseConnection, FromQueryResult};
use serde::Deserialize;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(search_form).service(search_results);
}

/// Template for search form and results
#[derive(Template)]
#[template(path = "search.html")]
struct SearchTemplate {
    client: ClientCtx,
    query: Option<String>,
    results: Option<SearchResults>,
}

#[derive(Debug)]
struct SearchResults {
    threads: Vec<ThreadSearchResult>,
    posts: Vec<PostSearchResult>,
    total_count: usize,
}

#[derive(Debug, FromQueryResult)]
#[allow(dead_code)]
struct ThreadSearchResult {
    id: i32,
    title: String,
    forum_id: i32,
    user_id: Option<i32>,
    created_at: chrono::NaiveDateTime,
    rank: f32,
}

#[derive(Debug, FromQueryResult)]
#[allow(dead_code)]
struct PostSearchResult {
    id: i32,
    thread_id: i32,
    content: String,
    user_id: Option<i32>,
    created_at: chrono::NaiveDateTime,
    rank: f32,
}

/// Form data for search query
#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

/// GET /search - Show search form
#[get("/search")]
pub async fn search_form(client: ClientCtx) -> impl Responder {
    SearchTemplate {
        client,
        query: None,
        results: None,
    }
    .to_response()
}

/// GET /search?q=query - Perform search and show results
#[get("/search/results")]
pub async fn search_results(
    req: HttpRequest,
    client: ClientCtx,
    query: web::Query<SearchQuery>,
) -> Result<impl Responder, Error> {
    // Get identifier for rate limiting (prefer user_id, fallback to IP)
    let rate_limit_id = client
        .get_id()
        .map(|id: i32| id.to_string())
        .unwrap_or_else(|| {
            crate::ip::extract_client_ip(&req)
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

    // Rate limiting - prevent search abuse
    if let Err(e) = crate::rate_limit::check_search_rate_limit(&rate_limit_id) {
        log::warn!("Search rate limit exceeded for: {}", rate_limit_id);
        return Err(error::ErrorTooManyRequests(format!(
            "Too many search requests. Please try again in {} seconds.",
            e.retry_after_seconds
        )));
    }

    let search_query = match &query.q {
        Some(q) if !q.trim().is_empty() => q.trim(),
        _ => {
            return Ok(SearchTemplate {
                client,
                query: None,
                results: None,
            }
            .to_response());
        }
    };

    let db = get_db_pool();

    // Search threads
    let threads = search_threads(db, search_query).await?;

    // Search posts
    let posts = search_posts(db, search_query).await?;

    let total_count = threads.len() + posts.len();

    Ok(SearchTemplate {
        client,
        query: Some(search_query.to_string()),
        results: Some(SearchResults {
            threads,
            posts,
            total_count,
        }),
    }
    .to_response())
}

/// Search threads by title using full-text search
async fn search_threads(
    db: &DatabaseConnection,
    query: &str,
) -> Result<Vec<ThreadSearchResult>, Error> {
    use sea_orm::Statement;

    // Use PostgreSQL's to_tsquery for search
    // ts_rank calculates relevance score
    let sql = r#"
        SELECT
            t.id,
            t.title,
            t.forum_id,
            t.user_id,
            t.created_at,
            ts_rank(t.title_tsv, to_tsquery('english', $1)) as rank
        FROM threads t
        WHERE t.title_tsv @@ to_tsquery('english', $1)
        ORDER BY rank DESC, t.created_at DESC
        LIMIT 50
    "#;

    // Convert search query to tsquery format (replace spaces with &)
    let tsquery = query.split_whitespace().collect::<Vec<&str>>().join(" & ");

    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![tsquery.into()],
    );

    ThreadSearchResult::find_by_statement(stmt)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Thread search error: {}", e);
            actix_web::error::ErrorInternalServerError("Search failed")
        })
}

/// Search posts by content using full-text search
async fn search_posts(
    db: &DatabaseConnection,
    query: &str,
) -> Result<Vec<PostSearchResult>, Error> {
    use sea_orm::Statement;

    // Join ugc_revisions with posts to get thread_id
    let sql = r#"
        SELECT
            p.id,
            p.thread_id,
            SUBSTRING(ur.content, 1, 200) as content,
            ur.user_id,
            ur.created_at,
            ts_rank(ur.content_tsv, to_tsquery('english', $1)) as rank
        FROM posts p
        JOIN ugc u ON p.ugc_id = u.id
        JOIN ugc_revisions ur ON u.ugc_revision_id = ur.id
        WHERE ur.content_tsv @@ to_tsquery('english', $1)
        ORDER BY rank DESC, ur.created_at DESC
        LIMIT 50
    "#;

    // Convert search query to tsquery format
    let tsquery = query.split_whitespace().collect::<Vec<&str>>().join(" & ");

    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![tsquery.into()],
    );

    PostSearchResult::find_by_statement(stmt)
        .all(db)
        .await
        .map_err(|e| {
            log::error!("Post search error: {}", e);
            actix_web::error::ErrorInternalServerError("Search failed")
        })
}
