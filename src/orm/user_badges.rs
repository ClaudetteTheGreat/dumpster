//! SeaORM Entity for user_badges table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_badges")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub badge_id: i32,
    pub awarded_at: DateTimeWithTimeZone,
    #[sea_orm(nullable)]
    pub awarded_by: Option<i32>,
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
        belongs_to = "super::badges::Entity",
        from = "Column::BadgeId",
        to = "super::badges::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Badge,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::AwardedBy",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    AwardedByUser,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::badges::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Badge.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
