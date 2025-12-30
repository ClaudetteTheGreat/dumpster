//! SeaORM Entity for themes table

use sea_orm::entity::prelude::*;

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
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::CreatedBy",
        to = "super::users::Column::Id"
    )]
    Creator,
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

    /// Combine all theme CSS for injection into page
    pub fn get_full_css(&self) -> String {
        let mut css = String::new();

        // Add CSS variable overrides
        let vars_style = self.get_css_variables_style();
        if !vars_style.is_empty() {
            css.push_str(&vars_style);
            css.push('\n');
        }

        // Add custom CSS
        if let Some(custom) = &self.css_custom {
            if !custom.trim().is_empty() {
                css.push_str(custom);
            }
        }

        css
    }

    /// Check if this theme has any custom CSS
    pub fn has_custom_css(&self) -> bool {
        let has_vars = self.css_variables.as_ref().map_or(false, |v| !v.trim().is_empty());
        let has_custom = self.css_custom.as_ref().map_or(false, |c| !c.trim().is_empty());
        has_vars || has_custom
    }
}
