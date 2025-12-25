//! Poll voting endpoints

use crate::db::get_db_pool;
use crate::middleware::ClientCtx;
use crate::orm::{poll_options, poll_votes, polls};
use actix_web::{error, post, web, Error, HttpResponse, Responder};
use sea_orm::{entity::*, query::*, sea_query::Expr, ColumnTrait, EntityTrait};
use serde::Deserialize;

pub(super) fn configure(conf: &mut actix_web::web::ServiceConfig) {
    conf.service(vote_on_poll);
}

#[derive(Deserialize)]
pub struct VoteFormData {
    pub csrf_token: String,
    #[serde(default)]
    pub option_ids: Vec<i32>,
}

#[post("/polls/{poll_id}/vote")]
pub async fn vote_on_poll(
    client: ClientCtx,
    cookies: actix_session::Session,
    path: web::Path<i32>,
    form: web::Form<VoteFormData>,
) -> Result<impl Responder, Error> {
    // Validate CSRF token
    crate::middleware::csrf::validate_csrf_token(&cookies, &form.csrf_token)?;

    // Require authentication
    let user_id = client.require_login()?;

    let poll_id = path.into_inner();
    let db = get_db_pool();

    // Fetch the poll
    let poll = polls::Entity::find_by_id(poll_id)
        .one(db)
        .await
        .map_err(error::ErrorInternalServerError)?
        .ok_or_else(|| error::ErrorNotFound("Poll not found."))?;

    // Check if poll is closed
    if let Some(closes_at) = poll.closes_at {
        if closes_at < chrono::Utc::now().naive_utc() {
            return Err(error::ErrorForbidden("This poll is closed."));
        }
    }

    // Validate option_ids
    if form.option_ids.is_empty() {
        return Err(error::ErrorBadRequest("Please select at least one option."));
    }

    if form.option_ids.len() > poll.max_choices as usize {
        return Err(error::ErrorBadRequest(format!(
            "You can only select up to {} option(s).",
            poll.max_choices
        )));
    }

    // Verify all selected options belong to this poll
    let valid_options = poll_options::Entity::find()
        .filter(poll_options::Column::PollId.eq(poll_id))
        .filter(poll_options::Column::Id.is_in(form.option_ids.clone()))
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    if valid_options.len() != form.option_ids.len() {
        return Err(error::ErrorBadRequest("Invalid poll option(s) selected."));
    }

    // Check if user has already voted
    let existing_votes = poll_votes::Entity::find()
        .filter(poll_votes::Column::PollId.eq(poll_id))
        .filter(poll_votes::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let has_voted = !existing_votes.is_empty();

    // If user has voted and poll doesn't allow changing vote, reject
    if has_voted && !poll.allow_change_vote {
        return Err(error::ErrorForbidden(
            "You have already voted and this poll does not allow changing your vote.",
        ));
    }

    // Begin transaction
    let txn = db.begin().await.map_err(error::ErrorInternalServerError)?;

    // If changing vote, delete old votes
    if has_voted {
        // Decrement vote counts for old options
        for old_vote in &existing_votes {
            poll_options::Entity::update_many()
                .col_expr(
                    poll_options::Column::VoteCount,
                    Expr::col(poll_options::Column::VoteCount).sub(1),
                )
                .filter(poll_options::Column::Id.eq(old_vote.option_id))
                .exec(&txn)
                .await
                .map_err(error::ErrorInternalServerError)?;
        }

        // Delete old votes
        poll_votes::Entity::delete_many()
            .filter(poll_votes::Column::PollId.eq(poll_id))
            .filter(poll_votes::Column::UserId.eq(user_id))
            .exec(&txn)
            .await
            .map_err(error::ErrorInternalServerError)?;
    }

    // Insert new votes
    let now = chrono::Utc::now().naive_utc();
    for option_id in &form.option_ids {
        let vote = poll_votes::ActiveModel {
            poll_id: Set(poll_id),
            option_id: Set(*option_id),
            user_id: Set(user_id),
            created_at: Set(now),
            ..Default::default()
        };
        poll_votes::Entity::insert(vote)
            .exec(&txn)
            .await
            .map_err(error::ErrorInternalServerError)?;

        // Increment vote count for this option
        poll_options::Entity::update_many()
            .col_expr(
                poll_options::Column::VoteCount,
                Expr::col(poll_options::Column::VoteCount).add(1),
            )
            .filter(poll_options::Column::Id.eq(*option_id))
            .exec(&txn)
            .await
            .map_err(error::ErrorInternalServerError)?;
    }

    // Commit transaction
    txn.commit()
        .await
        .map_err(error::ErrorInternalServerError)?;

    // Redirect back to the thread
    Ok(HttpResponse::Found()
        .append_header(("Location", format!("/threads/{}/", poll.thread_id)))
        .finish())
}
