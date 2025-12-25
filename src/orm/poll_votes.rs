//! SeaORM Entity for poll_votes table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "poll_votes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub poll_id: i32,
    pub option_id: i32,
    pub user_id: i32,
    pub created_at: DateTime,
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
    #[sea_orm(
        belongs_to = "super::poll_options::Entity",
        from = "Column::OptionId",
        to = "super::poll_options::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    PollOption,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::polls::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Poll.def()
    }
}

impl Related<super::poll_options::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PollOption.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
