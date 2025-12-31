//! Theme management service with caching

use crate::db::get_db_pool;
use crate::orm::themes;
use once_cell::sync::OnceCell;
use sea_orm::{entity::*, query::*};
use std::collections::HashMap;
use std::sync::RwLock;

/// Cache of active themes (slug -> Model)
static THEME_CACHE: OnceCell<RwLock<HashMap<String, themes::Model>>> = OnceCell::new();

/// Cache of active themes by ID (id -> Model) for parent lookups
static THEME_CACHE_BY_ID: OnceCell<RwLock<HashMap<i32, themes::Model>>> = OnceCell::new();

/// Initialize the theme caches (call once at startup)
fn init_cache() {
    let _ = THEME_CACHE.set(RwLock::new(HashMap::new()));
    let _ = THEME_CACHE_BY_ID.set(RwLock::new(HashMap::new()));
}

/// Load all active themes from database into cache
pub async fn load_themes() -> Result<(), sea_orm::DbErr> {
    // Initialize caches if not already done
    if THEME_CACHE.get().is_none() {
        init_cache();
    }

    let db = get_db_pool();

    let active_themes = themes::Entity::find()
        .filter(themes::Column::IsActive.eq(true))
        .order_by_asc(themes::Column::DisplayOrder)
        .all(db)
        .await?;

    // Update slug-based cache
    {
        let mut cache = THEME_CACHE
            .get()
            .expect("Theme cache not initialized")
            .write()
            .expect("Theme cache lock poisoned");

        cache.clear();
        for theme in &active_themes {
            cache.insert(theme.slug.clone(), theme.clone());
        }
    }

    // Update ID-based cache
    {
        let mut cache = THEME_CACHE_BY_ID
            .get()
            .expect("Theme ID cache not initialized")
            .write()
            .expect("Theme ID cache lock poisoned");

        cache.clear();
        for theme in active_themes {
            cache.insert(theme.id, theme);
        }
    }

    log::info!(
        "Loaded {} themes into cache",
        THEME_CACHE
            .get()
            .and_then(|c| c.read().ok())
            .map(|c| c.len())
            .unwrap_or(0)
    );
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

/// Get theme by ID from cache
pub fn get_theme_by_id(id: i32) -> Option<themes::Model> {
    THEME_CACHE_BY_ID
        .get()
        .and_then(|cache| cache.read().ok())
        .and_then(|cache| cache.get(&id).cloned())
}

/// Get a snapshot of the ID-based theme cache for CSS rendering with inheritance
pub fn get_theme_cache_by_id() -> HashMap<i32, themes::Model> {
    THEME_CACHE_BY_ID
        .get()
        .and_then(|cache| cache.read().ok())
        .map(|cache| cache.clone())
        .unwrap_or_default()
}

/// Get the full CSS for a theme including inherited parent CSS
pub fn get_theme_full_css(theme: &themes::Model) -> String {
    let cache = get_theme_cache_by_id();
    theme.get_full_css_with_cache(&cache)
}

/// Check if a theme has any CSS including inherited from parents
pub fn theme_has_css(theme: &themes::Model) -> bool {
    let cache = get_theme_cache_by_id();
    theme.has_custom_css_with_cache(&cache)
}

/// Get available themes that can be used as parents (excludes self and descendants)
pub fn get_available_parents(exclude_id: Option<i32>) -> Vec<themes::Model> {
    let all_themes = get_active_themes();

    match exclude_id {
        Some(id) => {
            // Filter out self and any themes that have this as an ancestor
            let cache = get_theme_cache_by_id();
            all_themes
                .into_iter()
                .filter(|t| t.id != id && !is_descendant_of(t, id, &cache))
                .collect()
        }
        None => all_themes,
    }
}

/// Check if a theme is a descendant of another theme (to prevent cycles)
fn is_descendant_of(theme: &themes::Model, ancestor_id: i32, cache: &HashMap<i32, themes::Model>) -> bool {
    let mut current = theme.parent_id;
    let mut depth = 0;
    const MAX_DEPTH: usize = 10;

    while let Some(parent_id) = current {
        if parent_id == ancestor_id {
            return true;
        }
        if depth >= MAX_DEPTH {
            break;
        }
        current = cache.get(&parent_id).and_then(|p| p.parent_id);
        depth += 1;
    }

    false
}
