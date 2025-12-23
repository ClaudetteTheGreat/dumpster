//! SeaORM Entity for reports table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "reports")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub reporter_id: i32,
    pub content_type: String,
    pub content_id: i32,
    pub reason: String,
    pub details: Option<String>,
    pub status: String,
    pub moderator_id: Option<i32>,
    pub moderator_notes: Option<String>,
    pub resolved_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::ReporterId",
        to = "super::users::Column::Id"
    )]
    Reporter,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::ModeratorId",
        to = "super::users::Column::Id"
    )]
    Moderator,
}

impl ActiveModelBehavior for ActiveModel {}
