//! SeaORM Entity for polls table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "polls")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub thread_id: i32,
    pub question: String,
    pub max_choices: i32,
    pub allow_change_vote: bool,
    pub show_results_before_vote: bool,
    pub closes_at: Option<DateTime>,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::threads::Entity",
        from = "Column::ThreadId",
        to = "super::threads::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Thread,
    #[sea_orm(has_many = "super::poll_options::Entity")]
    PollOptions,
    #[sea_orm(has_many = "super::poll_votes::Entity")]
    PollVotes,
}

impl Related<super::threads::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Thread.def()
    }
}

impl Related<super::poll_options::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PollOptions.def()
    }
}

impl Related<super::poll_votes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PollVotes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
