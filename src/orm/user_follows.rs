//! SeaORM Entity for user_follows table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_follows")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub follower_id: i32,
    pub following_id: i32,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::FollowerId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Follower,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::FollowingId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Following,
    #[sea_orm(
        belongs_to = "super::user_names::Entity",
        from = "Column::FollowerId",
        to = "super::user_names::Column::UserId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    FollowerName,
    #[sea_orm(
        belongs_to = "super::user_names::Entity",
        from = "Column::FollowingId",
        to = "super::user_names::Column::UserId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    FollowingName,
    #[sea_orm(
        belongs_to = "super::user_avatars::Entity",
        from = "Column::FollowerId",
        to = "super::user_avatars::Column::UserId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    FollowerAvatar,
    #[sea_orm(
        belongs_to = "super::user_avatars::Entity",
        from = "Column::FollowingId",
        to = "super::user_avatars::Column::UserId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    FollowingAvatar,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Follower.def()
    }
}

impl Related<super::user_names::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FollowerName.def()
    }
}

impl Related<super::user_avatars::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FollowerAvatar.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// Special relation to Follower Avatar Attachment
pub struct FollowToFollowerAvatarAttachment;

impl Linked for FollowToFollowerAvatarAttachment {
    type FromEntity = super::user_follows::Entity;
    type ToEntity = super::attachments::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            super::user_follows::Relation::FollowerAvatar.def(),
            super::user_avatars::Relation::Attachments.def(),
        ]
    }
}

// Special relation to Following Avatar Attachment
pub struct FollowToFollowingAvatarAttachment;

impl Linked for FollowToFollowingAvatarAttachment {
    type FromEntity = super::user_follows::Entity;
    type ToEntity = super::attachments::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            super::user_follows::Relation::FollowingAvatar.def(),
            super::user_avatars::Relation::Attachments.def(),
        ]
    }
}
