//! Recent activity page - shows latest threads and posts across the forum

use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{forums, threads, user_names, users};
use crate::url::UrlToken;
use actix_web::{get, Responder};
use askama_actix::{Template, TemplateToResponse};
use chrono::NaiveDateTime;
use sea_orm::{entity::*, query::*, FromQueryResult};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(view_recent_threads).service(view_recent_posts);
}

/// Display model for a recent thread
#[derive(Debug, FromQueryResult)]
pub struct RecentThread {
    pub id: i32,
    pub title: String,
    pub forum_id: i32,
    pub forum_label: String,
    pub created_at: NaiveDateTime,
    pub post_count: i32,
    pub view_count: i32,
    pub user_id: Option<i32>,
    pub username: Option<String>,
}

impl RecentThread {
    pub fn get_url_token(&self) -> UrlToken<'static> {
        UrlToken {
            id: Some(self.id),
            name: self.title.to_owned(),
            base_url: "threads",
            class: "thread",
        }
    }

    pub fn get_forum_url_token(&self) -> UrlToken<'static> {
        UrlToken {
            id: Some(self.forum_id),
            name: self.forum_label.to_owned(),
            base_url: "forums",
            class: "forum",
        }
    }

    pub fn get_author_url_token(&self) -> Option<UrlToken<'static>> {
        if let (Some(user_id), Some(username)) = (self.user_id, &self.username) {
            Some(UrlToken {
                id: Some(user_id),
                name: username.to_owned(),
                base_url: "members",
                class: "username",
            })
        } else {
            None
        }
    }
}

/// Display model for a recent post
#[derive(Debug, FromQueryResult)]
pub struct RecentPost {
    pub id: i32,
    pub thread_id: i32,
    pub thread_title: String,
    pub forum_id: i32,
    pub forum_label: String,
    pub content_preview: String,
    pub created_at: NaiveDateTime,
    pub user_id: Option<i32>,
    pub username: Option<String>,
}

impl RecentPost {
    pub fn get_url(&self) -> String {
        format!("/threads/{}/post-{}", self.thread_id, self.id)
    }

    pub fn get_thread_url_token(&self) -> UrlToken<'static> {
        UrlToken {
            id: Some(self.thread_id),
            name: self.thread_title.to_owned(),
            base_url: "threads",
            class: "thread",
        }
    }

    pub fn get_forum_url_token(&self) -> UrlToken<'static> {
        UrlToken {
            id: Some(self.forum_id),
            name: self.forum_label.to_owned(),
            base_url: "forums",
            class: "forum",
        }
    }

    pub fn get_author_url_token(&self) -> Option<UrlToken<'static>> {
        if let (Some(user_id), Some(username)) = (self.user_id, &self.username) {
            Some(UrlToken {
                id: Some(user_id),
                name: username.to_owned(),
                base_url: "members",
                class: "username",
            })
        } else {
            None
        }
    }

    /// Get a short preview of the post content (first 200 characters)
    pub fn get_preview(&self) -> String {
        if self.content_preview.len() > 200 {
            format!("{}...", &self.content_preview[..200])
        } else {
            self.content_preview.clone()
        }
    }
}

#[derive(Template)]
#[template(path = "recent_threads.html")]
pub struct RecentThreadsTemplate {
    pub client: ClientCtx,
    pub threads: Vec<RecentThread>,
}

#[derive(Template)]
#[template(path = "recent_posts.html")]
pub struct RecentPostsTemplate {
    pub client: ClientCtx,
    pub posts: Vec<RecentPost>,
}

/// View recent threads across all forums
#[get("/recent/threads")]
async fn view_recent_threads(client: ClientCtx) -> impl Responder {
    let db = get_db_pool();

    // Get the 50 most recent threads across all forums
    let threads = threads::Entity::find()
        .inner_join(forums::Entity)
        .left_join(users::Entity)
        .left_join(user_names::Entity)
        .select_only()
        .column_as(threads::Column::Id, "id")
        .column_as(threads::Column::Title, "title")
        .column_as(threads::Column::ForumId, "forum_id")
        .column_as(forums::Column::Label, "forum_label")
        .column_as(threads::Column::CreatedAt, "created_at")
        .column_as(threads::Column::PostCount, "post_count")
        .column_as(threads::Column::ViewCount, "view_count")
        .column_as(threads::Column::UserId, "user_id")
        .column_as(user_names::Column::Name, "username")
        .order_by_desc(threads::Column::CreatedAt)
        .limit(50)
        .into_model::<RecentThread>()
        .all(db)
        .await
        .unwrap_or_default();

    RecentThreadsTemplate { client, threads }.to_response()
}

/// View recent posts across all forums
#[get("/recent/posts")]
async fn view_recent_posts(client: ClientCtx) -> impl Responder {
    use sea_orm::{DbBackend, FromQueryResult, Statement};

    let db = get_db_pool();

    // Use raw SQL to properly join through all tables including UGC
    let sql = r#"
        SELECT
            p.id,
            p.thread_id,
            t.title as thread_title,
            f.id as forum_id,
            f.label as forum_label,
            COALESCE(LEFT(ugc.content, 250), '[No content]') as content_preview,
            p.created_at,
            p.user_id,
            un.name as username
        FROM posts p
        INNER JOIN threads t ON p.thread_id = t.id
        INNER JOIN forums f ON t.forum_id = f.id
        LEFT JOIN users u ON p.user_id = u.id
        LEFT JOIN user_names un ON u.id = un.user_id
        LEFT JOIN ugc_revisions ugc ON p.id = ugc.ugc_id
        ORDER BY p.created_at DESC
        LIMIT 50
    "#;

    let posts =
        RecentPost::find_by_statement(Statement::from_string(DbBackend::Postgres, sql.to_owned()))
            .all(db)
            .await
            .unwrap_or_default();

    RecentPostsTemplate { client, posts }.to_response()
}
