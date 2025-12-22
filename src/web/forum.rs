use super::thread::{validate_thread_form, NewThreadFormData, ThreadForTemplate};
use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{posts, threads, user_names};
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use askama_actix::{Template, TemplateToResponse};
use sea_orm::{entity::*, query::*, sea_query::Expr, FromQueryResult};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(create_thread)
        .service(view_forums)
        .service(view_forum);
}

#[derive(Template)]
#[template(path = "forum.html")]
pub struct ForumTemplate<'a> {
    pub client: ClientCtx,
    pub forum: &'a crate::orm::forums::Model,
    pub threads: &'a Vec<ThreadForTemplate>,
    pub breadcrumbs: Vec<super::thread::Breadcrumb>,
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
    let form = validate_thread_form(form)?;

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
pub async fn view_forum(client: ClientCtx, path: web::Path<i32>) -> Result<impl Responder, Error> {
    use crate::orm::forums;

    let forum_id = path.into_inner();
    let forum = forums::Entity::find_by_id(forum_id)
        .one(get_db_pool())
        .await
        .map_err(|_| error::ErrorInternalServerError("Could not look up forum."))?
        .ok_or_else(|| error::ErrorNotFound("Forum not found."))?;

    let threads: Vec<ThreadForTemplate> = threads::Entity::find()
        // Authoring User
        .left_join(user_names::Entity)
        .column_as(user_names::Column::Name, "username")
        // Last Post
        // TODO: This is an actual nightmare.
        //.join_join(JoinType::LeftJoin, threads::Relations::::to(), threads::Relation::LastPost<posts::Entity>::via())
        //.column_as(users::Column::Name, "username")
        // Execute
        .filter(threads::Column::ForumId.eq(forum_id))
        .order_by_desc(threads::Column::IsPinned)
        .order_by_desc(threads::Column::LastPostAt)
        .into_model::<ThreadForTemplate>()
        .all(get_db_pool())
        .await
        .unwrap_or_default();

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

    Ok(ForumTemplate {
        client: client.to_owned(),
        forum: &forum,
        threads: &threads,
        breadcrumbs,
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
