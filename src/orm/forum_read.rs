//! SeaORM Entity for forum_read table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "forum_read")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub forum_id: i32,
    pub read_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::forums::Entity",
        from = "Column::ForumId",
        to = "super::forums::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Forum,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::forums::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Forum.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
