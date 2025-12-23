//! SeaORM Entity for ugc_reactions table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ugc_reactions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub ugc_id: i32,
    pub user_id: i32,
    pub reaction_type_id: i32,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::ugc::Entity",
        from = "Column::UgcId",
        to = "super::ugc::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Ugc,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::reaction_types::Entity",
        from = "Column::ReactionTypeId",
        to = "super::reaction_types::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    ReactionType,
}

impl Related<super::ugc::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ugc.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::reaction_types::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ReactionType.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
