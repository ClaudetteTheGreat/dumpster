//! SeaORM Entity for themes table

use sea_orm::entity::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "themes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub is_dark: bool,
    pub is_active: bool,
    pub display_order: i32,
    #[sea_orm(column_type = "Text", nullable)]
    pub css_variables: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub css_custom: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub created_by: Option<i32>,
    /// Parent theme ID for inheritance (child themes inherit parent's CSS)
    pub parent_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::CreatedBy",
        to = "super::users::Column::Id"
    )]
    Creator,
    #[sea_orm(belongs_to = "Entity", from = "Column::ParentId", to = "Column::Id")]
    Parent,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Creator.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Generate CSS for this theme's variable overrides
    /// Returns CSS wrapped in html.dark or :root selector as appropriate
    pub fn get_css_variables_style(&self) -> String {
        match &self.css_variables {
            Some(vars) if !vars.trim().is_empty() => {
                let selector = if self.is_dark { "html.dark" } else { ":root" };
                format!("{} {{ {} }}", selector, vars.trim())
            }
            _ => String::new(),
        }
    }

    /// Get full custom CSS
    pub fn get_custom_css(&self) -> String {
        self.css_custom.clone().unwrap_or_default()
    }

    /// Combine all theme CSS for injection into page (without inheritance)
    pub fn get_full_css(&self) -> String {
        self.get_full_css_with_cache(&HashMap::new())
    }

    /// Combine all theme CSS including inherited parent CSS
    /// Uses theme cache to resolve parent themes recursively
    pub fn get_full_css_with_cache(&self, theme_cache: &HashMap<i32, Model>) -> String {
        self.get_full_css_recursive(theme_cache, 0)
    }

    /// Internal recursive implementation with depth limit to prevent infinite loops
    fn get_full_css_recursive(&self, theme_cache: &HashMap<i32, Model>, depth: usize) -> String {
        const MAX_DEPTH: usize = 10;

        let mut css = String::new();

        // First, include parent's CSS recursively (if we have a parent and haven't hit depth limit)
        if depth < MAX_DEPTH {
            if let Some(parent_id) = self.parent_id {
                if let Some(parent) = theme_cache.get(&parent_id) {
                    let parent_css = parent.get_full_css_recursive(theme_cache, depth + 1);
                    if !parent_css.is_empty() {
                        css.push_str(&parent_css);
                        css.push('\n');
                    }
                }
            }
        }

        // Add this theme's CSS variable overrides
        let vars_style = self.get_css_variables_style();
        if !vars_style.is_empty() {
            css.push_str(&vars_style);
            css.push('\n');
        }

        // Add this theme's custom CSS
        if let Some(custom) = &self.css_custom {
            if !custom.trim().is_empty() {
                css.push_str(custom);
            }
        }

        css
    }

    /// Check if this theme has any custom CSS (not including parent)
    pub fn has_custom_css(&self) -> bool {
        let has_vars = self
            .css_variables
            .as_ref()
            .is_some_and(|v| !v.trim().is_empty());
        let has_custom = self
            .css_custom
            .as_ref()
            .is_some_and(|c| !c.trim().is_empty());
        has_vars || has_custom
    }

    /// Check if this theme has any CSS (including inherited from parent)
    pub fn has_custom_css_with_cache(&self, theme_cache: &HashMap<i32, Model>) -> bool {
        if self.has_custom_css() {
            return true;
        }

        // Check parent recursively
        if let Some(parent_id) = self.parent_id {
            if let Some(parent) = theme_cache.get(&parent_id) {
                return parent.has_custom_css_with_cache(theme_cache);
            }
        }

        false
    }

    /// Get the parent theme name (for display purposes)
    pub fn get_parent_name<'a>(&self, theme_cache: &'a HashMap<i32, Model>) -> Option<&'a str> {
        self.parent_id
            .and_then(|pid| theme_cache.get(&pid))
            .map(|p| p.name.as_str())
    }

    /// Check if this theme's parent_id matches a given id (for template comparisons)
    pub fn has_parent_id(&self, id: &i32) -> bool {
        self.parent_id == Some(*id)
    }

    /// Check if this theme is the parent of another theme (for template comparisons)
    pub fn is_parent_of(&self, other: &Model) -> bool {
        other.parent_id == Some(self.id)
    }
}
