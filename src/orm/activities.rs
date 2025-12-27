//! SeaORM Entity for activities table

use sea_orm::entity::prelude::*;

/// Activity types for the activity feed
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "activity_type")]
pub enum ActivityType {
    #[sea_orm(string_value = "post_created")]
    PostCreated,
    #[sea_orm(string_value = "thread_created")]
    ThreadCreated,
    #[sea_orm(string_value = "profile_post_created")]
    ProfilePostCreated,
    #[sea_orm(string_value = "user_followed")]
    UserFollowed,
    #[sea_orm(string_value = "reaction_given")]
    ReactionGiven,
}

impl ActivityType {
    /// Get a human-readable description of the activity
    pub fn description(&self) -> &'static str {
        match self {
            Self::PostCreated => "posted a reply",
            Self::ThreadCreated => "started a new thread",
            Self::ProfilePostCreated => "posted on a profile",
            Self::UserFollowed => "followed",
            Self::ReactionGiven => "reacted to a post",
        }
    }

    /// Get an icon/emoji for the activity type
    pub fn icon(&self) -> &'static str {
        match self {
            Self::PostCreated => "💬",
            Self::ThreadCreated => "📝",
            Self::ProfilePostCreated => "📋",
            Self::UserFollowed => "👤",
            Self::ReactionGiven => "👍",
        }
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "activities")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub activity_type: ActivityType,
    pub user_id: i32,
    pub created_at: DateTimeWithTimeZone,

    // Polymorphic target references
    pub target_user_id: Option<i32>,
    pub target_thread_id: Option<i32>,
    pub target_post_id: Option<i32>,
    pub target_forum_id: Option<i32>,

    // Denormalized data for display
    pub title: Option<String>,
    pub content_preview: Option<String>,
    pub reaction_emoji: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::TargetUserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    TargetUser,
    #[sea_orm(
        belongs_to = "super::threads::Entity",
        from = "Column::TargetThreadId",
        to = "super::threads::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    TargetThread,
    #[sea_orm(
        belongs_to = "super::posts::Entity",
        from = "Column::TargetPostId",
        to = "super::posts::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    TargetPost,
    #[sea_orm(
        belongs_to = "super::forums::Entity",
        from = "Column::TargetForumId",
        to = "super::forums::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    TargetForum,
    #[sea_orm(
        belongs_to = "super::user_names::Entity",
        from = "Column::UserId",
        to = "super::user_names::Column::UserId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    UserName,
    #[sea_orm(
        belongs_to = "super::user_avatars::Entity",
        from = "Column::UserId",
        to = "super::user_avatars::Column::UserId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    UserAvatar,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::threads::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TargetThread.def()
    }
}

impl Related<super::posts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TargetPost.def()
    }
}

impl Related<super::forums::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TargetForum.def()
    }
}

impl Related<super::user_names::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserName.def()
    }
}

impl Related<super::user_avatars::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserAvatar.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// Linked relation to get user's avatar attachment
pub struct ActivityToUserAvatarAttachment;

impl Linked for ActivityToUserAvatarAttachment {
    type FromEntity = super::activities::Entity;
    type ToEntity = super::attachments::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            super::activities::Relation::UserAvatar.def(),
            super::user_avatars::Relation::Attachments.def(),
        ]
    }
}
