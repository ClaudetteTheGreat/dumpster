use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{attachments, posts, threads, user_names, users};
use crate::user::Profile as UserProfile;
use actix_web::{error, get, web, Error, Responder};
use askama_actix::{Template, TemplateToResponse};
use sea_orm::{entity::*, query::*, DatabaseConnection};

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(view_member).service(view_members);
}

/// User statistics for profile display
#[derive(Debug, Clone)]
pub struct UserStatistics {
    pub post_count: i64,
    pub thread_count: i64,
    pub member_since: chrono::NaiveDateTime,
}

/// Get user statistics for profile display
async fn get_user_statistics(
    db: &DatabaseConnection,
    user_id: i32,
    created_at: chrono::NaiveDateTime,
) -> Result<UserStatistics, sea_orm::DbErr> {
    // Count posts by this user
    let post_count = posts::Entity::find()
        .filter(posts::Column::UserId.eq(user_id))
        .count(db)
        .await?;

    // Count threads created by this user
    let thread_count = threads::Entity::find()
        .filter(threads::Column::UserId.eq(user_id))
        .count(db)
        .await?;

    Ok(UserStatistics {
        post_count: post_count as i64,
        thread_count: thread_count as i64,
        member_since: created_at,
    })
}

#[get("/members/{user_id}/")]
pub async fn view_member(
    client: ClientCtx,
    path: web::Path<(i32,)>,
) -> Result<impl Responder, Error> {
    #[derive(Template)]
    #[template(path = "member.html")]
    pub struct MemberTemplate {
        pub client: ClientCtx,
        pub user: UserProfile,
        pub stats: UserStatistics,
    }

    let user_id = path.into_inner().0;
    let db = get_db_pool();

    let user = users::Entity::find_by_id(user_id)
        .left_join(user_names::Entity)
        .column_as(user_names::Column::Name, "name")
        .left_join(attachments::Entity)
        .column_as(attachments::Column::Filename, "avatar_filename")
        .column_as(attachments::Column::FileHeight, "avatar_height")
        .column_as(attachments::Column::FileWidth, "avatar_width")
        .into_model::<UserProfile>()
        .one(db)
        .await
        .map_err(|e| {
            log::error!("error {:?}", e);
            error::ErrorInternalServerError("Couldn't load user.")
        })?
        .ok_or_else(|| error::ErrorNotFound("User not found."))?;

    // Get user statistics
    let stats = get_user_statistics(db, user_id, user.created_at)
        .await
        .map_err(|e| {
            log::error!("error getting user stats: {:?}", e);
            error::ErrorInternalServerError("Couldn't load user statistics.")
        })?;

    Ok(MemberTemplate {
        client,
        user,
        stats,
    }
    .to_response())
}

#[get("/members")]
pub async fn view_members(client: ClientCtx) -> impl Responder {
    #[derive(Template)]
    #[template(path = "members.html")]
    pub struct MembersTemplate {
        pub client: ClientCtx,
        pub users: Vec<UserProfile>,
    }

    match users::Entity::find()
        .left_join(user_names::Entity)
        .column_as(user_names::Column::Name, "name")
        .left_join(attachments::Entity)
        .column_as(attachments::Column::Filename, "avatar_filename")
        .column_as(attachments::Column::FileHeight, "avatar_height")
        .column_as(attachments::Column::FileWidth, "avatar_width")
        .into_model::<UserProfile>()
        .all(get_db_pool())
        .await
    {
        Ok(users) => Ok(MembersTemplate { client, users }.to_response()),
        Err(e) => {
            log::error!("error {:?}", e);
            Err(error::ErrorInternalServerError("Couldn't load users"))
        }
    }
}
