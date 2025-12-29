//! SeaORM Entity for tags table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub color: Option<String>,
    pub is_global: bool,
    pub use_count: i32,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::thread_tags::Entity")]
    ThreadTags,
    #[sea_orm(has_many = "super::tag_forums::Entity")]
    TagForums,
}

impl Related<super::thread_tags::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ThreadTags.def()
    }
}

impl Related<super::tag_forums::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TagForums.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
