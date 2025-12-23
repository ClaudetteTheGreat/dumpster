//! SeaORM Entity for reaction_types table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "reaction_types")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub emoji: String,
    pub display_order: i32,
    pub is_positive: bool,
    pub is_active: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::ugc_reactions::Entity")]
    UgcReactions,
}

impl Related<super::ugc_reactions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UgcReactions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
