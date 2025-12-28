//! SeaORM Entity for UGC (User Generated Content) deletions
//!
//! Supports three deletion types:
//! - Normal: Soft delete, visible to moderators, can be restored
//! - Permanent: Hard reference kept for audit, content purged (for spam)
//! - LegalHold: Cannot be modified/restored except by admin

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Deletion type enum matching PostgreSQL deletion_type
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "deletion_type")]
#[derive(Default)]
pub enum DeletionType {
    /// Soft delete - visible to moderators, can be restored
    #[sea_orm(string_value = "normal")]
    #[default]
    Normal,
    /// Permanent delete - content purged, audit trail kept (for spam)
    #[sea_orm(string_value = "permanent")]
    Permanent,
    /// Legal hold - cannot be modified except by admin
    #[sea_orm(string_value = "legal_hold")]
    LegalHold,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ugc_deletions")]
pub struct Model {
    /// The UGC ID (matches ugc.id)
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i32,
    /// Original content author ID (for reference)
    pub user_id: Option<i32>,
    /// When the content was deleted
    pub deleted_at: DateTime,
    /// Reason for deletion
    #[sea_orm(column_type = "Text", nullable)]
    pub reason: Option<String>,
    /// Type of deletion (normal, permanent, legal_hold)
    pub deletion_type: DeletionType,
    /// Who performed the deletion (moderator/admin)
    pub deleted_by_id: Option<i32>,
    /// When legal hold was placed (if applicable)
    pub legal_hold_at: Option<DateTime>,
    /// Who placed the legal hold
    pub legal_hold_by: Option<i32>,
    /// Reason for legal hold
    #[sea_orm(column_type = "Text", nullable)]
    pub legal_hold_reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::ugc::Entity",
        from = "Column::Id",
        to = "super::ugc::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Ugc,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Users,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::DeletedById",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    DeletedBy,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::LegalHoldBy",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    LegalHoldByUser,
}

impl Related<super::ugc::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ugc.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
