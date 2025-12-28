//! SeaORM Entity for badges table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "badges")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub slug: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    pub icon: String,
    #[sea_orm(nullable)]
    pub color: Option<String>,
    pub condition_type: BadgeConditionType,
    #[sea_orm(nullable)]
    pub condition_value: Option<i32>,
    pub display_order: i32,
    pub is_active: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "badge_condition_type"
)]
pub enum BadgeConditionType {
    #[sea_orm(string_value = "manual")]
    Manual,
    #[sea_orm(string_value = "post_count")]
    PostCount,
    #[sea_orm(string_value = "thread_count")]
    ThreadCount,
    #[sea_orm(string_value = "time_member")]
    TimeMember,
    #[sea_orm(string_value = "reputation")]
    Reputation,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_badges::Entity")]
    UserBadges,
}

impl Related<super::user_badges::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserBadges.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_badges::Relation::User.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::user_badges::Relation::Badge.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
