//! SeaORM Entity for reaction_types table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "reaction_types")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub emoji: String,
    pub display_order: i32,
    pub is_positive: bool,
    pub is_active: bool,
    pub reputation_value: i32,
    /// Optional custom image attachment for this reaction
    pub attachment_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::ugc_reactions::Entity")]
    UgcReactions,
    #[sea_orm(
        belongs_to = "super::attachments::Entity",
        from = "Column::AttachmentId",
        to = "super::attachments::Column::Id"
    )]
    Attachment,
}

impl Related<super::ugc_reactions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UgcReactions.def()
    }
}

impl Related<super::attachments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Attachment.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Get the display HTML for this reaction (image or emoji)
    pub fn get_display_html(&self, attachment: Option<&super::attachments::Model>) -> String {
        if let Some(att) = attachment {
            format!(
                r#"<img src="/content/{}/{}" alt="{}" class="reaction-image" />"#,
                &att.hash[0..64],
                att.filename,
                self.name
            )
        } else {
            format!(r#"<span class="reaction-emoji">{}</span>"#, self.emoji)
        }
    }
}
