//! User warnings entity
//!
//! Warnings issued to users by moderators, with points that accumulate

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_warnings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub issued_by: Option<i32>,
    #[sea_orm(column_type = "Text")]
    pub reason: String,
    pub points: i32,
    pub expires_at: Option<DateTime>,
    pub acknowledged_at: Option<DateTime>,
    pub created_at: DateTime,
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
        belongs_to = "super::users::Entity",
        from = "Column::IssuedBy",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    IssuedByUser,
}

impl ActiveModelBehavior for ActiveModel {}
