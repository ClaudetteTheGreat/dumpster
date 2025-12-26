//! SeaORM Entity for ip_bans table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ip_bans")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_type = "Custom(\"inet\".to_owned())", unique)]
    pub ip_address: String,
    pub banned_by: Option<i32>,
    pub reason: String,
    pub expires_at: Option<DateTime>,
    pub created_at: DateTime,
    pub is_permanent: bool,
    pub is_range_ban: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::BannedBy",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    Moderator,
}

impl ActiveModelBehavior for ActiveModel {}
