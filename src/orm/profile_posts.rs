//! SeaORM Entity for profile_posts table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "profile_posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub profile_user_id: i32,
    pub author_id: Option<i32>,
    pub ugc_id: i32,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::ProfileUserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    ProfileUser,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::AuthorId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    Author,
    #[sea_orm(
        belongs_to = "super::ugc::Entity",
        from = "Column::UgcId",
        to = "super::ugc::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Ugc,
    #[sea_orm(
        belongs_to = "super::user_names::Entity",
        from = "Column::AuthorId",
        to = "super::user_names::Column::UserId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    AuthorName,
    #[sea_orm(
        belongs_to = "super::user_avatars::Entity",
        from = "Column::AuthorId",
        to = "super::user_avatars::Column::UserId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    AuthorAvatar,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProfileUser.def()
    }
}

impl Related<super::ugc::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ugc.def()
    }
}

impl Related<super::user_names::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AuthorName.def()
    }
}

impl Related<super::user_avatars::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AuthorAvatar.def()
    }
}

impl Related<super::ugc_revisions::Entity> for Entity {
    fn to() -> RelationDef {
        super::ugc::Relation::UgcRevisions.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::ugc::Relation::Posts.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

// Special relation to UGC Revision
pub struct ProfilePostToUgcRevision;

impl Linked for ProfilePostToUgcRevision {
    type FromEntity = super::profile_posts::Entity;
    type ToEntity = super::ugc_revisions::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            super::profile_posts::Relation::Ugc.def(),
            super::ugc::Relation::UgcRevisions.def(),
        ]
    }
}

// Special relation to Author Avatar Attachment
pub struct ProfilePostToAvatarAttachment;

impl Linked for ProfilePostToAvatarAttachment {
    type FromEntity = super::profile_posts::Entity;
    type ToEntity = super::attachments::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            super::profile_posts::Relation::AuthorAvatar.def(),
            super::user_avatars::Relation::Attachments.def(),
        ]
    }
}
