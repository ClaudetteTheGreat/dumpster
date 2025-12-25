//! SeaORM Entity for forum_moderators

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "forum_moderators")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub forum_id: i32,
    pub user_id: i32,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::forums::Entity",
        from = "Column::ForumId",
        to = "super::forums::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Forum,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::forums::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Forum.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
