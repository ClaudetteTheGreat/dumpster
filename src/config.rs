//! Configuration management module
//!
//! Provides database-backed configuration with in-memory caching.
//! Settings are loaded from the database on startup and cached for fast access.

use crate::orm::{feature_flags, setting_history, settings};
use chrono::Utc;
use dashmap::DashMap;
use sea_orm::{entity::*, query::*, sea_query::Expr, DatabaseConnection, DbErr, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Represents a typed setting value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettingValue {
    String(String),
    Int(i64),
    Bool(bool),
    Json(serde_json::Value),
}

impl SettingValue {
    /// Parse a string value based on the value_type
    pub fn parse(value: &str, value_type: &str) -> Option<Self> {
        match value_type {
            "string" => Some(SettingValue::String(value.to_string())),
            "int" => value.parse().ok().map(SettingValue::Int),
            "bool" => value.parse().ok().map(SettingValue::Bool),
            "json" => serde_json::from_str(value).ok().map(SettingValue::Json),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn to_string_value(&self) -> String {
        match self {
            SettingValue::String(s) => s.clone(),
            SettingValue::Int(i) => i.to_string(),
            SettingValue::Bool(b) => b.to_string(),
            SettingValue::Json(j) => j.to_string(),
        }
    }

    /// Get the type name
    pub fn type_name(&self) -> &'static str {
        match self {
            SettingValue::String(_) => "string",
            SettingValue::Int(_) => "int",
            SettingValue::Bool(_) => "bool",
            SettingValue::Json(_) => "json",
        }
    }

    /// Try to get as string
    pub fn as_string(&self) -> Option<&String> {
        match self {
            SettingValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as int
    pub fn as_int(&self) -> Option<i64> {
        match self {
            SettingValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SettingValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Configuration manager with caching
pub struct Config {
    settings: DashMap<String, SettingValue>,
    feature_flags: DashMap<String, bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Create a new empty config
    pub fn new() -> Self {
        Self {
            settings: DashMap::new(),
            feature_flags: DashMap::new(),
        }
    }

    /// Load all settings and feature flags from the database
    pub async fn load_from_database(&self, db: &DatabaseConnection) -> Result<(), DbErr> {
        // Load settings
        let db_settings = settings::Entity::find().all(db).await?;

        for setting in db_settings {
            if let Some(value) = SettingValue::parse(&setting.value, &setting.value_type) {
                self.settings.insert(setting.key, value);
            }
        }

        // Load feature flags
        let flags = feature_flags::Entity::find().all(db).await?;

        for flag in flags {
            self.feature_flags.insert(flag.key, flag.enabled);
        }

        log::info!(
            "Loaded {} settings and {} feature flags from database",
            self.settings.len(),
            self.feature_flags.len()
        );

        Ok(())
    }

    /// Get a string setting
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.settings
            .get(key)
            .and_then(|v| v.as_string().cloned())
    }

    /// Get a string setting with a default value
    pub fn get_string_or(&self, key: &str, default: &str) -> String {
        self.get_string(key).unwrap_or_else(|| default.to_string())
    }

    /// Get an integer setting
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.settings.get(key).and_then(|v| v.as_int())
    }

    /// Get an integer setting with a default value
    pub fn get_int_or(&self, key: &str, default: i64) -> i64 {
        self.get_int(key).unwrap_or(default)
    }

    /// Get a boolean setting
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.settings.get(key).and_then(|v| v.as_bool())
    }

    /// Get a boolean setting with a default value
    pub fn get_bool_or(&self, key: &str, default: bool) -> bool {
        self.get_bool(key).unwrap_or(default)
    }

    /// Check if a feature flag is enabled
    pub fn is_feature_enabled(&self, key: &str) -> bool {
        self.feature_flags
            .get(key)
            .map(|v| *v)
            .unwrap_or(false)
    }

    /// Update a setting value (also updates database and history)
    pub async fn set_value(
        &self,
        db: &DatabaseConnection,
        key: &str,
        value: SettingValue,
        user_id: Option<i32>,
    ) -> Result<(), DbErr> {
        // Get old value for history
        let old_setting = settings::Entity::find_by_id(key.to_string()).one(db).await?;

        let value_str = value.to_string_value();
        let value_type = value.type_name().to_string();

        // Update or insert setting
        if old_setting.is_some() {
            settings::Entity::update_many()
                .col_expr(settings::Column::Value, Expr::value(value_str.clone()))
                .col_expr(
                    settings::Column::UpdatedAt,
                    Expr::value(Utc::now().naive_utc()),
                )
                .col_expr(settings::Column::UpdatedBy, Expr::value(user_id))
                .filter(settings::Column::Key.eq(key))
                .exec(db)
                .await?;

            // Save history
            if let Some(old) = old_setting {
                let history = setting_history::ActiveModel {
                    setting_key: Set(key.to_string()),
                    old_value: Set(Some(old.value)),
                    new_value: Set(value_str.clone()),
                    changed_by: Set(user_id),
                    changed_at: Set(Utc::now().naive_utc()),
                    ..Default::default()
                };
                history.insert(db).await?;
            }
        } else {
            // Insert new setting
            let setting = settings::ActiveModel {
                key: Set(key.to_string()),
                value: Set(value_str.clone()),
                value_type: Set(value_type),
                description: Set(None),
                category: Set("custom".to_string()),
                is_public: Set(false),
                updated_at: Set(Utc::now().naive_utc()),
                updated_by: Set(user_id),
            };
            setting.insert(db).await?;
        }

        // Update cache
        self.settings.insert(key.to_string(), value);

        Ok(())
    }

    /// Toggle a feature flag
    pub async fn set_feature_flag(
        &self,
        db: &DatabaseConnection,
        key: &str,
        enabled: bool,
    ) -> Result<(), DbErr> {
        feature_flags::Entity::update_many()
            .col_expr(feature_flags::Column::Enabled, Expr::value(enabled))
            .col_expr(
                feature_flags::Column::UpdatedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(feature_flags::Column::Key.eq(key))
            .exec(db)
            .await?;

        // Update cache
        self.feature_flags.insert(key.to_string(), enabled);

        Ok(())
    }

    /// Get all settings grouped by category
    pub async fn get_all_by_category(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<(String, Vec<settings::Model>)>, DbErr> {
        let all_settings = settings::Entity::find()
            .order_by_asc(settings::Column::Category)
            .order_by_asc(settings::Column::Key)
            .all(db)
            .await?;

        let mut categories: Vec<(String, Vec<settings::Model>)> = Vec::new();
        let mut current_category = String::new();

        for setting in all_settings {
            if setting.category != current_category {
                current_category = setting.category.clone();
                categories.push((current_category.clone(), Vec::new()));
            }
            if let Some((_, settings)) = categories.last_mut() {
                settings.push(setting);
            }
        }

        Ok(categories)
    }

    /// Get all feature flags
    pub async fn get_all_feature_flags(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<feature_flags::Model>, DbErr> {
        feature_flags::Entity::find()
            .order_by_asc(feature_flags::Column::Key)
            .all(db)
            .await
    }

    /// Get setting history
    pub async fn get_setting_history(
        &self,
        db: &DatabaseConnection,
        key: &str,
        limit: u64,
    ) -> Result<Vec<setting_history::Model>, DbErr> {
        setting_history::Entity::find()
            .filter(setting_history::Column::SettingKey.eq(key))
            .order_by_desc(setting_history::Column::ChangedAt)
            .limit(limit)
            .all(db)
            .await
    }

    // Convenience methods for common settings

    /// Get site name
    pub fn site_name(&self) -> String {
        self.get_string_or("site_name", "Ruforo")
    }

    /// Get site description
    pub fn site_description(&self) -> String {
        self.get_string_or("site_description", "A forum built in Rust")
    }

    /// Get posts per page default
    pub fn posts_per_page(&self) -> i64 {
        self.get_int_or("posts_per_page", 25)
    }

    /// Get threads per page default
    pub fn threads_per_page(&self) -> i64 {
        self.get_int_or("threads_per_page", 20)
    }

    /// Check if registration is enabled
    pub fn registration_enabled(&self) -> bool {
        self.get_bool_or("registration_enabled", true)
    }

    /// Check if maintenance mode is active
    pub fn maintenance_mode(&self) -> bool {
        self.get_bool_or("maintenance_mode", false)
    }

    /// Check if chat is enabled
    pub fn chat_enabled(&self) -> bool {
        self.get_bool_or("chat_enabled", true)
    }

    /// Check if reactions are enabled
    pub fn reactions_enabled(&self) -> bool {
        self.get_bool_or("reactions_enabled", true)
    }

    /// Check if polls are enabled
    pub fn polls_enabled(&self) -> bool {
        self.get_bool_or("polls_enabled", true)
    }

    /// Get session timeout in minutes
    pub fn session_timeout_minutes(&self) -> i64 {
        self.get_int_or("session_timeout_minutes", 1440)
    }

    /// Get max upload size in MB
    pub fn max_upload_size_mb(&self) -> i64 {
        self.get_int_or("max_upload_size_mb", 10)
    }
}

/// Create a new Arc-wrapped Config
pub fn create_config() -> Arc<Config> {
    Arc::new(Config::new())
}
