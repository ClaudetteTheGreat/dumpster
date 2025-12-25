//! SeaORM Entity for poll_options table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "poll_options")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub poll_id: i32,
    pub option_text: String,
    pub display_order: i32,
    pub vote_count: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::polls::Entity",
        from = "Column::PollId",
        to = "super::polls::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Poll,
    #[sea_orm(has_many = "super::poll_votes::Entity")]
    PollVotes,
}

impl Related<super::polls::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Poll.def()
    }
}

impl Related<super::poll_votes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PollVotes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
