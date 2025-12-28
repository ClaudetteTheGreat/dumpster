//! User social media links ORM model

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Supported social media platforms
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "social_platform")]
pub enum SocialPlatform {
    #[sea_orm(string_value = "twitter")]
    Twitter,
    #[sea_orm(string_value = "discord")]
    Discord,
    #[sea_orm(string_value = "github")]
    Github,
    #[sea_orm(string_value = "youtube")]
    Youtube,
    #[sea_orm(string_value = "twitch")]
    Twitch,
    #[sea_orm(string_value = "steam")]
    Steam,
    #[sea_orm(string_value = "telegram")]
    Telegram,
    #[sea_orm(string_value = "reddit")]
    Reddit,
    #[sea_orm(string_value = "instagram")]
    Instagram,
    #[sea_orm(string_value = "facebook")]
    Facebook,
    #[sea_orm(string_value = "linkedin")]
    Linkedin,
    #[sea_orm(string_value = "tiktok")]
    Tiktok,
    #[sea_orm(string_value = "website")]
    Website,
    #[sea_orm(string_value = "other")]
    Other,
}

impl SocialPlatform {
    /// Get display name for the platform
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Twitter => "Twitter/X",
            Self::Discord => "Discord",
            Self::Github => "GitHub",
            Self::Youtube => "YouTube",
            Self::Twitch => "Twitch",
            Self::Steam => "Steam",
            Self::Telegram => "Telegram",
            Self::Reddit => "Reddit",
            Self::Instagram => "Instagram",
            Self::Facebook => "Facebook",
            Self::Linkedin => "LinkedIn",
            Self::Tiktok => "TikTok",
            Self::Website => "Website",
            Self::Other => "Other",
        }
    }

    /// Get icon/emoji for the platform
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Twitter => "𝕏",
            Self::Discord => "🎮",
            Self::Github => "🐙",
            Self::Youtube => "▶️",
            Self::Twitch => "📺",
            Self::Steam => "🎮",
            Self::Telegram => "✈️",
            Self::Reddit => "🤖",
            Self::Instagram => "📷",
            Self::Facebook => "📘",
            Self::Linkedin => "💼",
            Self::Tiktok => "🎵",
            Self::Website => "🌐",
            Self::Other => "🔗",
        }
    }

    /// Get the base URL pattern for the platform
    /// Returns None if the platform doesn't have a standard URL pattern
    pub fn url_pattern(&self) -> Option<&'static str> {
        match self {
            Self::Twitter => Some("https://twitter.com/{}"),
            Self::Discord => None, // Discord uses invite links or user IDs
            Self::Github => Some("https://github.com/{}"),
            Self::Youtube => Some("https://youtube.com/@{}"),
            Self::Twitch => Some("https://twitch.tv/{}"),
            Self::Steam => Some("https://steamcommunity.com/id/{}"),
            Self::Telegram => Some("https://t.me/{}"),
            Self::Reddit => Some("https://reddit.com/u/{}"),
            Self::Instagram => Some("https://instagram.com/{}"),
            Self::Facebook => Some("https://facebook.com/{}"),
            Self::Linkedin => Some("https://linkedin.com/in/{}"),
            Self::Tiktok => Some("https://tiktok.com/@{}"),
            Self::Website => None, // User provides full URL
            Self::Other => None,   // User provides full URL
        }
    }

    /// Generate full URL from username
    pub fn generate_url(&self, username: &str) -> Option<String> {
        self.url_pattern()
            .map(|pattern| pattern.replace("{}", username))
    }

    /// Parse platform from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "twitter" => Some(Self::Twitter),
            "discord" => Some(Self::Discord),
            "github" => Some(Self::Github),
            "youtube" => Some(Self::Youtube),
            "twitch" => Some(Self::Twitch),
            "steam" => Some(Self::Steam),
            "telegram" => Some(Self::Telegram),
            "reddit" => Some(Self::Reddit),
            "instagram" => Some(Self::Instagram),
            "facebook" => Some(Self::Facebook),
            "linkedin" => Some(Self::Linkedin),
            "tiktok" => Some(Self::Tiktok),
            "website" => Some(Self::Website),
            "other" => Some(Self::Other),
            _ => None,
        }
    }

    /// Get all platforms for display in forms
    pub fn all() -> Vec<Self> {
        vec![
            Self::Twitter,
            Self::Discord,
            Self::Github,
            Self::Youtube,
            Self::Twitch,
            Self::Steam,
            Self::Telegram,
            Self::Reddit,
            Self::Instagram,
            Self::Facebook,
            Self::Linkedin,
            Self::Tiktok,
            Self::Website,
            Self::Other,
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_social_links")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub platform: SocialPlatform,
    pub username: String,
    pub url: Option<String>,
    pub display_order: i32,
    pub is_visible: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Model {
    /// Get the display URL for this social link
    pub fn get_url(&self) -> String {
        if let Some(ref url) = self.url {
            url.clone()
        } else {
            self.platform
                .generate_url(&self.username)
                .unwrap_or_else(|| format!("#{}", self.username))
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
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
