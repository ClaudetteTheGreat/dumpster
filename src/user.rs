use crate::attachment::AttachmentSize;
use crate::orm::{attachments, user_avatars, user_names, users};
use crate::url::UrlToken;
use sea_orm::{entity::*, query::*, DatabaseConnection, FromQueryResult};

/// Base URL fragment for resource.
pub static RESOURCE_URL: &str = "members";

pub fn find_also_user<E, C>(sel: Select<E>, col: C) -> SelectTwo<E, users::Entity>
where
    E: EntityTrait<Column = C>,
    C: IntoSimpleExpr + ColumnTrait,
{
    use sea_orm::sea_query::Expr;

    sel.select_also(users::Entity)
        .join(
            JoinType::LeftJoin,
            E::belongs_to(users::Entity)
                .from(col)
                .to(users::Column::Id)
                .into(),
        )
        .join(JoinType::LeftJoin, users::Relation::UserName.def())
        .column_as(user_names::Column::Name, "B_name")
        .join(JoinType::LeftJoin, users::Relation::UserAvatar.def())
        .join(
            JoinType::LeftJoin,
            user_avatars::Relation::Attachments.def(),
        )
        .column_as(attachments::Column::Filename, "B_avatar_filename")
        .column_as(attachments::Column::FileHeight, "B_avatar_height")
        .column_as(attachments::Column::FileWidth, "B_avatar_width")
        // Add post count subquery
        .column_as(
            Expr::cust_with_values(
                "(SELECT COUNT(*) FROM posts WHERE posts.user_id = users.id)",
                std::iter::empty::<sea_orm::Value>(),
            ),
            "B_post_count",
        )
}

/// A struct to hold all information for a user, including relational information.
#[derive(Clone, Debug, FromQueryResult)]
pub struct Profile {
    pub id: i32,
    pub name: String,
    pub created_at: chrono::NaiveDateTime,
    pub password_cipher: String,
    pub avatar_filename: Option<String>,
    pub avatar_height: Option<i32>,
    pub avatar_width: Option<i32>,
    pub posts_per_page: i32,
    pub post_count: Option<i64>,
}

impl Profile {
    /// Returns a fully qualified user profile by id.
    pub async fn get_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<Self>, sea_orm::DbErr> {
        use sea_orm::{DbBackend, Statement};

        // Use raw SQL to include post count
        let sql = r#"
            SELECT
                u.id,
                un.name,
                u.created_at,
                u.password_cipher::text as password_cipher,
                a.filename as avatar_filename,
                a.file_height as avatar_height,
                a.file_width as avatar_width,
                u.posts_per_page,
                COUNT(p.id) as post_count
            FROM users u
            LEFT JOIN user_names un ON un.user_id = u.id
            LEFT JOIN user_avatars ua ON ua.user_id = u.id
            LEFT JOIN attachments a ON a.id = ua.attachment_id
            LEFT JOIN posts p ON p.user_id = u.id
            WHERE u.id = $1
            GROUP BY u.id, un.name, u.created_at, u.password_cipher, a.filename, a.file_height, a.file_width, u.posts_per_page
        "#;

        Self::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            vec![id.into()],
        ))
        .one(db)
        .await
    }

    /// Provides semantically correct HTML for an avatar.
    pub fn get_avatar_html(&self, size: AttachmentSize) -> String {
        if let (Some(filename), Some(width), Some(height)) = (
            self.avatar_filename.as_ref(),
            self.avatar_width,
            self.avatar_width,
        ) {
            crate::attachment::get_avatar_html(&filename, (width, height), size)
        } else {
            "".to_owned()
        }
    }

    /// Provides a URL token for this resource.
    pub fn get_url_token(&self) -> UrlToken<'static> {
        UrlToken {
            id: Some(self.id),
            name: self.name.to_owned(),
            base_url: RESOURCE_URL,
            class: "username",
        }
    }
}

pub async fn get_user_id_from_name(db: &DatabaseConnection, name: &str) -> Option<i32> {
    user_names::Entity::find()
        .filter(user_names::Column::Name.eq(name))
        .one(db)
        .await
        .unwrap_or(None)
        .map(|user_name| user_name.user_id)
}
