//! SeaORM Entity for mod_log table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "mod_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub moderator_id: Option<i32>,
    pub action: String,
    pub target_type: String,
    pub target_id: i32,
    pub reason: Option<String>,
    pub metadata: Option<Json>,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::ModeratorId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    Moderator,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Moderator.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
