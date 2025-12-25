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
    pub forum_id: Option<i32>,
    pub use_count: i32,
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
    #[sea_orm(has_many = "super::thread_tags::Entity")]
    ThreadTags,
}

impl Related<super::forums::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Forum.def()
    }
}

impl Related<super::thread_tags::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ThreadTags.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
