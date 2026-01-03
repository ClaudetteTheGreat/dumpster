//! URL unfurling - fetches and caches metadata for URLs
//!
//! Provides an API endpoint for getting URL metadata (title, description, image)
//! which is cached in the database for performance.

use crate::db::get_db_pool;
use crate::orm::unfurl_cache;
use actix_web::{error, get, web, Error, HttpRequest, HttpResponse};
use chrono::Utc;
use sea_orm::{entity::*, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(get_unfurl);
}

/// Unfurl metadata response
#[derive(Serialize, Clone)]
pub struct UnfurlResponse {
    pub success: bool,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Site type for special rendering (youtube, twitter, github, or null for generic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_type: Option<String>,
    /// Site-specific embed data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_data: Option<EmbedData>,
}

/// Site-specific embed data for special rendering
#[derive(Serialize, Clone, Default)]
pub struct EmbedData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tweet_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_name: Option<String>,
}

#[derive(Deserialize)]
pub struct UnfurlQuery {
    url: String,
}

/// Cache duration in hours - refetch after this time
const CACHE_DURATION_HOURS: i64 = 24;

/// Maximum time to wait for URL fetch
const FETCH_TIMEOUT_SECS: u64 = 10;

/// Maximum response body size (1MB)
const MAX_BODY_SIZE: usize = 1024 * 1024;

/// Get unfurl metadata for a URL
#[get("/api/unfurl")]
async fn get_unfurl(
    req: HttpRequest,
    query: web::Query<UnfurlQuery>,
) -> Result<HttpResponse, Error> {
    // Get client IP for rate limiting
    let ip = crate::ip::extract_client_ip(&req)
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Rate limiting - uses API rate limit
    if let Err(e) = crate::rate_limit::check_api_rate_limit(&ip) {
        log::warn!("Unfurl rate limit exceeded for IP: {}", ip);
        return Err(error::ErrorTooManyRequests(format!(
            "Too many requests. Please try again in {} seconds.",
            e.retry_after_seconds
        )));
    }

    let url = &query.url;

    // Validate URL
    let parsed_url = url::Url::parse(url).map_err(|_| error::ErrorBadRequest("Invalid URL"))?;

    // Only allow http/https
    match parsed_url.scheme() {
        "http" | "https" => {}
        _ => return Err(error::ErrorBadRequest("Only HTTP/HTTPS URLs are supported")),
    }

    // Compute URL hash for cache lookup
    let url_hash = compute_url_hash(url);

    let db = get_db_pool();

    // Check cache first
    if let Some(cached) = unfurl_cache::Entity::find()
        .filter(unfurl_cache::Column::UrlHash.eq(url_hash.clone()))
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
    {
        // Check if cache is still fresh
        let age = Utc::now().naive_utc() - cached.fetched_at;
        if age.num_hours() < CACHE_DURATION_HOURS {
            // Detect site type from URL (not cached, but fast)
            let (site_type, embed_data) = detect_site_type(&parsed_url);

            // Return cached data
            return Ok(HttpResponse::Ok().json(UnfurlResponse {
                success: cached.error_message.is_none(),
                url: cached.url,
                title: cached.title,
                description: cached.description,
                image_url: cached.image_url,
                site_name: cached.site_name,
                favicon_url: cached.favicon_url,
                error: cached.error_message,
                site_type,
                embed_data,
            }));
        }

        // Cache expired, delete old entry
        unfurl_cache::Entity::delete_by_id(cached.id)
            .exec(db)
            .await
            .map_err(error::ErrorInternalServerError)?;
    }

    // Fetch fresh data
    let result = fetch_url_metadata(url, &parsed_url).await;

    // Store in cache
    let now = Utc::now().naive_utc();
    let cache_entry = unfurl_cache::ActiveModel {
        url_hash: Set(url_hash),
        url: Set(url.clone()),
        title: Set(result.title.clone()),
        description: Set(result.description.clone()),
        image_url: Set(result.image_url.clone()),
        site_name: Set(result.site_name.clone()),
        favicon_url: Set(result.favicon_url.clone()),
        fetched_at: Set(now),
        error_message: Set(result.error.clone()),
        created_at: Set(now),
        ..Default::default()
    };

    cache_entry
        .insert(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(result))
}

/// Compute SHA256 hash of URL for cache key
fn compute_url_hash(url: &str) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(url.as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Fetch URL and extract metadata
async fn fetch_url_metadata(url: &str, parsed_url: &url::Url) -> UnfurlResponse {
    // Detect site type upfront (used for all responses, including errors)
    let (site_type, embed_data) = detect_site_type(parsed_url);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (compatible; DumpsterBot/1.0)")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return UnfurlResponse {
                success: false,
                url: url.to_string(),
                title: None,
                description: None,
                image_url: None,
                site_name: None,
                favicon_url: None,
                error: Some(format!("Failed to create HTTP client: {}", e)),
                site_type,
                embed_data,
            };
        }
    };

    // Fetch the URL
    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            return UnfurlResponse {
                success: false,
                url: url.to_string(),
                title: None,
                description: None,
                image_url: None,
                site_name: None,
                favicon_url: None,
                error: Some(format!("Failed to fetch URL: {}", e)),
                site_type,
                embed_data,
            };
        }
    };

    // Check content type
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.contains("text/html") && !content_type.contains("application/xhtml") {
        return UnfurlResponse {
            success: false,
            url: url.to_string(),
            title: None,
            description: None,
            image_url: None,
            site_name: None,
            favicon_url: None,
            error: Some("URL does not return HTML content".to_string()),
            site_type,
            embed_data,
        };
    }

    // Get body with size limit
    let body = match response.bytes().await {
        Ok(b) => {
            if b.len() > MAX_BODY_SIZE {
                return UnfurlResponse {
                    success: false,
                    url: url.to_string(),
                    title: None,
                    description: None,
                    image_url: None,
                    site_name: None,
                    favicon_url: None,
                    error: Some("Response too large".to_string()),
                    site_type,
                    embed_data,
                };
            }
            b
        }
        Err(e) => {
            return UnfurlResponse {
                success: false,
                url: url.to_string(),
                title: None,
                description: None,
                image_url: None,
                site_name: None,
                favicon_url: None,
                error: Some(format!("Failed to read response: {}", e)),
                site_type,
                embed_data,
            };
        }
    };

    let html = String::from_utf8_lossy(&body);

    // Parse HTML and extract metadata
    extract_metadata(&html, url, parsed_url, site_type, embed_data)
}

/// Extract Open Graph and meta tags from HTML
fn extract_metadata(
    html: &str,
    url: &str,
    parsed_url: &url::Url,
    site_type: Option<String>,
    embed_data: Option<EmbedData>,
) -> UnfurlResponse {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // Selectors for metadata
    let og_title = Selector::parse("meta[property='og:title']").ok();
    let og_description = Selector::parse("meta[property='og:description']").ok();
    let og_image = Selector::parse("meta[property='og:image']").ok();
    let og_site_name = Selector::parse("meta[property='og:site_name']").ok();
    let meta_description = Selector::parse("meta[name='description']").ok();
    let title_tag = Selector::parse("title").ok();
    let favicon_link = Selector::parse("link[rel='icon'], link[rel='shortcut icon']").ok();

    // Extract title (prefer og:title, fallback to <title>)
    let title = og_title
        .as_ref()
        .and_then(|s| document.select(s).next())
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.to_string())
        .or_else(|| {
            title_tag
                .as_ref()
                .and_then(|s| document.select(s).next())
                .map(|el| el.text().collect::<String>())
        })
        .map(|s| truncate_string(&s, 200));

    // Extract description (prefer og:description, fallback to meta description)
    let description = og_description
        .as_ref()
        .and_then(|s| document.select(s).next())
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.to_string())
        .or_else(|| {
            meta_description
                .as_ref()
                .and_then(|s| document.select(s).next())
                .and_then(|el| el.value().attr("content"))
                .map(|s| s.to_string())
        })
        .map(|s| truncate_string(&s, 500));

    // Extract image
    let image_url = og_image
        .as_ref()
        .and_then(|s| document.select(s).next())
        .and_then(|el| el.value().attr("content"))
        .and_then(|img_url| resolve_url(parsed_url, img_url));

    // Extract site name
    let site_name = og_site_name
        .as_ref()
        .and_then(|s| document.select(s).next())
        .and_then(|el| el.value().attr("content"))
        .map(|s| truncate_string(s, 100));

    // Extract favicon
    let favicon_url = favicon_link
        .as_ref()
        .and_then(|s| document.select(s).next())
        .and_then(|el| el.value().attr("href"))
        .and_then(|href| resolve_url(parsed_url, href))
        .or_else(|| {
            // Default to /favicon.ico
            let mut favicon = parsed_url.clone();
            favicon.set_path("/favicon.ico");
            favicon.set_query(None);
            Some(favicon.to_string())
        });

    UnfurlResponse {
        success: title.is_some() || description.is_some(),
        url: url.to_string(),
        title,
        description,
        image_url,
        site_name,
        favicon_url,
        error: None,
        site_type,
        embed_data,
    }
}

/// Resolve a potentially relative URL against a base URL
fn resolve_url(base: &url::Url, url_str: &str) -> Option<String> {
    base.join(url_str).ok().map(|u| u.to_string())
}

/// Truncate string to max length, adding ellipsis if needed
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Detect site type and extract embed data from URL
fn detect_site_type(url: &url::Url) -> (Option<String>, Option<EmbedData>) {
    let host = url.host_str().unwrap_or("");

    // YouTube detection
    if host == "youtube.com"
        || host == "www.youtube.com"
        || host == "m.youtube.com"
        || host == "youtu.be"
    {
        if let Some(video_id) = extract_youtube_id(url) {
            return (
                Some("youtube".to_string()),
                Some(EmbedData {
                    video_id: Some(video_id),
                    ..Default::default()
                }),
            );
        }
    }

    // Twitter/X detection
    if host == "twitter.com"
        || host == "www.twitter.com"
        || host == "x.com"
        || host == "www.x.com"
        || host == "mobile.twitter.com"
        || host == "mobile.x.com"
    {
        if let Some(tweet_id) = extract_tweet_id(url) {
            return (
                Some("twitter".to_string()),
                Some(EmbedData {
                    tweet_id: Some(tweet_id),
                    ..Default::default()
                }),
            );
        }
    }

    // GitHub detection
    if host == "github.com" || host == "www.github.com" {
        if let Some((owner, repo)) = extract_github_repo(url) {
            return (
                Some("github".to_string()),
                Some(EmbedData {
                    repo_owner: Some(owner),
                    repo_name: Some(repo),
                    ..Default::default()
                }),
            );
        }
    }

    (None, None)
}

/// Extract YouTube video ID from various URL formats
fn extract_youtube_id(url: &url::Url) -> Option<String> {
    let host = url.host_str()?;

    // youtube.com/watch?v=ID
    if host == "youtube.com" || host == "www.youtube.com" || host == "m.youtube.com" {
        if url.path() == "/watch" {
            return url
                .query_pairs()
                .find(|(k, _)| k == "v")
                .map(|(_, v)| v.to_string());
        }
        // youtube.com/embed/ID
        if let Some(id) = url.path().strip_prefix("/embed/") {
            let id = id.split(['?', '/']).next().unwrap_or(id);
            if is_valid_youtube_id(id) {
                return Some(id.to_string());
            }
        }
        // youtube.com/v/ID
        if let Some(id) = url.path().strip_prefix("/v/") {
            let id = id.split(['?', '/']).next().unwrap_or(id);
            if is_valid_youtube_id(id) {
                return Some(id.to_string());
            }
        }
        // youtube.com/shorts/ID
        if let Some(id) = url.path().strip_prefix("/shorts/") {
            let id = id.split(['?', '/']).next().unwrap_or(id);
            if is_valid_youtube_id(id) {
                return Some(id.to_string());
            }
        }
    }

    // youtu.be/ID (short URLs)
    if host == "youtu.be" {
        let path = url.path().strip_prefix('/').unwrap_or(url.path());
        let id = path.split(['?', '/']).next().unwrap_or(path);
        if is_valid_youtube_id(id) {
            return Some(id.to_string());
        }
    }

    None
}

/// Validate YouTube video ID format
fn is_valid_youtube_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 11
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Extract tweet ID from Twitter/X URL
fn extract_tweet_id(url: &url::Url) -> Option<String> {
    // Pattern: twitter.com/user/status/1234567890
    // Pattern: x.com/user/status/1234567890
    let path_segments: Vec<&str> = url.path().split('/').filter(|s| !s.is_empty()).collect();

    // Looking for: [username, "status", tweet_id]
    if path_segments.len() >= 3 && path_segments[1] == "status" {
        let tweet_id = path_segments[2];
        // Tweet IDs are numeric
        if tweet_id.chars().all(|c| c.is_ascii_digit()) && !tweet_id.is_empty() {
            return Some(tweet_id.to_string());
        }
    }

    None
}

/// Extract GitHub repo owner and name from URL
fn extract_github_repo(url: &url::Url) -> Option<(String, String)> {
    // Pattern: github.com/owner/repo
    let path_segments: Vec<&str> = url.path().split('/').filter(|s| !s.is_empty()).collect();

    if path_segments.len() >= 2 {
        let owner = path_segments[0];
        let repo = path_segments[1];

        // Validate: not a reserved path and looks like a valid username/repo
        let reserved = [
            "features",
            "explore",
            "marketplace",
            "trending",
            "collections",
            "events",
            "sponsors",
            "settings",
            "notifications",
            "issues",
            "pulls",
            "discussions",
            "codespaces",
            "orgs",
            "users",
            "apps",
            "login",
            "signup",
            "join",
            "pricing",
            "enterprise",
            "about",
            "security",
            "site",
            "new",
        ];

        if !reserved.contains(&owner.to_lowercase().as_str())
            && !owner.is_empty()
            && !repo.is_empty()
            && owner
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Some((owner.to_string(), repo.to_string()));
        }
    }

    None
}
