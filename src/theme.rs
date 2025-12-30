//! Theme management service with caching

use crate::db::get_db_pool;
use crate::orm::themes;
use once_cell::sync::OnceCell;
use sea_orm::{entity::*, query::*};
use std::collections::HashMap;
use std::sync::RwLock;

/// Cache of active themes (slug -> Model)
static THEME_CACHE: OnceCell<RwLock<HashMap<String, themes::Model>>> = OnceCell::new();

/// Initialize the theme cache (call once at startup)
fn init_cache() {
    THEME_CACHE
        .set(RwLock::new(HashMap::new()))
        .expect("Theme cache already initialized");
}

/// Load all active themes from database into cache
pub async fn load_themes() -> Result<(), sea_orm::DbErr> {
    // Initialize cache if not already done
    if THEME_CACHE.get().is_none() {
        init_cache();
    }

    let db = get_db_pool();

    let active_themes = themes::Entity::find()
        .filter(themes::Column::IsActive.eq(true))
        .order_by_asc(themes::Column::DisplayOrder)
        .all(db)
        .await?;

    let mut cache = THEME_CACHE
        .get()
        .expect("Theme cache not initialized")
        .write()
        .expect("Theme cache lock poisoned");

    cache.clear();

    for theme in active_themes {
        cache.insert(theme.slug.clone(), theme);
    }

    log::info!("Loaded {} themes into cache", cache.len());
    Ok(())
}

/// Reload theme cache (call after admin changes)
pub async fn reload_cache() {
    if let Err(e) = load_themes().await {
        log::error!("Failed to reload theme cache: {}", e);
    }
}

/// Get theme by slug from cache
pub fn get_theme(slug: &str) -> Option<themes::Model> {
    THEME_CACHE
        .get()
        .and_then(|cache| cache.read().ok())
        .and_then(|cache| cache.get(slug).cloned())
}

/// Get all active themes sorted by display order (for selection dropdowns)
pub fn get_active_themes() -> Vec<themes::Model> {
    THEME_CACHE
        .get()
        .and_then(|cache| cache.read().ok())
        .map(|cache| {
            let mut themes: Vec<_> = cache.values().cloned().collect();
            themes.sort_by_key(|t| t.display_order);
            themes
        })
        .unwrap_or_default()
}

/// Get the first dark theme for auto mode detection
pub fn get_default_dark_theme() -> Option<themes::Model> {
    THEME_CACHE
        .get()
        .and_then(|cache| cache.read().ok())
        .and_then(|cache| {
            cache
                .values()
                .filter(|t| t.is_dark)
                .min_by_key(|t| t.display_order)
                .cloned()
        })
}

/// Get the first light theme (default)
pub fn get_default_light_theme() -> Option<themes::Model> {
    THEME_CACHE
        .get()
        .and_then(|cache| cache.read().ok())
        .and_then(|cache| {
            cache
                .values()
                .filter(|t| !t.is_dark)
                .min_by_key(|t| t.display_order)
                .cloned()
        })
}

/// Check if a theme slug exists and is active
pub fn theme_exists(slug: &str) -> bool {
    get_theme(slug).is_some()
}
