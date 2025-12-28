//! Word filter entity for content moderation
//!
//! Supports three action types:
//! - `replace`: Replace matched text with replacement (word exchange)
//! - `block`: Reject the content entirely
//! - `flag`: Allow but flag for moderator review

use sea_orm::entity::prelude::*;

/// Word filter action type
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(20))")]
#[derive(Default)]
pub enum FilterAction {
    #[sea_orm(string_value = "replace")]
    #[default]
    Replace,
    #[sea_orm(string_value = "block")]
    Block,
    #[sea_orm(string_value = "flag")]
    Flag,
}


impl FilterAction {
    /// Returns true if this is the Replace action
    pub fn is_replace(&self) -> bool {
        matches!(self, FilterAction::Replace)
    }

    /// Returns true if this is the Block action
    pub fn is_block(&self) -> bool {
        matches!(self, FilterAction::Block)
    }

    /// Returns true if this is the Flag action
    pub fn is_flag(&self) -> bool {
        matches!(self, FilterAction::Flag)
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "word_filters")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub pattern: String,
    pub replacement: Option<String>,
    pub is_regex: bool,
    pub is_case_sensitive: bool,
    pub is_whole_word: bool,
    pub action: FilterAction,
    pub is_enabled: bool,
    pub created_by: Option<i32>,
    pub created_at: DateTime,
    #[sea_orm(column_type = "Text", nullable)]
    pub notes: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::CreatedBy",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    CreatedByUser,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedByUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
