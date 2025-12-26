//! Setting history entity for audit trail

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "setting_history")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub setting_key: String,
    pub old_value: Option<String>,
    pub new_value: String,
    pub changed_by: Option<i32>,
    pub changed_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::ChangedBy",
        to = "super::users::Column::Id"
    )]
    User,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
