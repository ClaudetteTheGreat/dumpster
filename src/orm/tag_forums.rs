//! SeaORM Entity for tag_forums junction table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tag_forums")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub tag_id: i32,
    pub forum_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tags::Entity",
        from = "Column::TagId",
        to = "super::tags::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Tag,
    #[sea_orm(
        belongs_to = "super::forums::Entity",
        from = "Column::ForumId",
        to = "super::forums::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Forum,
}

impl Related<super::tags::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tag.def()
    }
}

impl Related<super::forums::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Forum.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
