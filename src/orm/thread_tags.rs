//! SeaORM Entity for thread_tags junction table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "thread_tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub thread_id: i32,
    pub tag_id: i32,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::threads::Entity",
        from = "Column::ThreadId",
        to = "super::threads::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Thread,
    #[sea_orm(
        belongs_to = "super::tags::Entity",
        from = "Column::TagId",
        to = "super::tags::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Tag,
}

impl Related<super::threads::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Thread.def()
    }
}

impl Related<super::tags::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tag.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
