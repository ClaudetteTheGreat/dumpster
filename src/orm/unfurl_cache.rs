//! SeaORM Entity for unfurl_cache

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "unfurl_cache")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub url_hash: String,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub site_name: Option<String>,
    pub favicon_url: Option<String>,
    pub fetched_at: DateTime,
    pub error_message: Option<String>,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
