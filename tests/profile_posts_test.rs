//! Integration tests for profile wall posts

mod common;
use serial_test::serial;

use chrono::Utc;
use common::{database::*, fixtures::*};
use sea_orm::{entity::*, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

#[actix_rt::test]
#[serial]
async fn test_create_profile_post() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{profile_posts, ugc, ugc_revisions};

    // Create profile owner
    let profile_owner = create_test_user(&db, "profile_owner", "password123")
        .await
        .expect("Failed to create profile owner");

    // Create author (poster)
    let author = create_test_user(&db, "wall_poster", "password123")
        .await
        .expect("Failed to create author");

    // Create UGC for the post content
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    // Create UGC revision with content
    let revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        ip_id: Set(None),
        user_id: Set(Some(author.id)),
        content: Set("Hello from your wall!".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    let revision_model = revision
        .insert(&db)
        .await
        .expect("Failed to create revision");

    // Update UGC with revision
    use sea_orm::sea_query::Expr;
    ugc::Entity::update_many()
        .col_expr(ugc::Column::UgcRevisionId, Expr::value(revision_model.id))
        .filter(ugc::Column::Id.eq(ugc_model.id))
        .exec(&db)
        .await
        .expect("Failed to update UGC");

    // Create the profile post
    let profile_post = profile_posts::ActiveModel {
        profile_user_id: Set(profile_owner.id),
        author_id: Set(Some(author.id)),
        ugc_id: Set(ugc_model.id),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    };
    let post_model = profile_post
        .insert(&db)
        .await
        .expect("Failed to create profile post");

    // Verify the post was created
    assert!(post_model.id > 0);
    assert_eq!(post_model.profile_user_id, profile_owner.id);
    assert_eq!(post_model.author_id, Some(author.id));
    assert_eq!(post_model.ugc_id, ugc_model.id);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_fetch_profile_posts_ordered_by_date() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{profile_posts, ugc, ugc_revisions};

    // Create profile owner
    let profile_owner = create_test_user(&db, "profile_owner2", "password123")
        .await
        .expect("Failed to create profile owner");

    // Create author
    let author = create_test_user(&db, "wall_poster2", "password123")
        .await
        .expect("Failed to create author");

    // Create 3 posts with different timestamps
    for i in 0..3 {
        let ugc_entry = ugc::ActiveModel {
            ugc_revision_id: Set(None),
            reaction_count: Set(0),
            ..Default::default()
        };
        let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

        let revision = ugc_revisions::ActiveModel {
            ugc_id: Set(ugc_model.id),
            ip_id: Set(None),
            user_id: Set(Some(author.id)),
            content: Set(format!("Post number {}", i + 1)),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        };
        let revision_model = revision
            .insert(&db)
            .await
            .expect("Failed to create revision");

        use sea_orm::sea_query::Expr;
        ugc::Entity::update_many()
            .col_expr(ugc::Column::UgcRevisionId, Expr::value(revision_model.id))
            .filter(ugc::Column::Id.eq(ugc_model.id))
            .exec(&db)
            .await
            .expect("Failed to update UGC");

        let profile_post = profile_posts::ActiveModel {
            profile_user_id: Set(profile_owner.id),
            author_id: Set(Some(author.id)),
            ugc_id: Set(ugc_model.id),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        };
        profile_post
            .insert(&db)
            .await
            .expect("Failed to create profile post");

        // Small delay to ensure different timestamps
        actix_rt::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // Fetch posts ordered by created_at DESC
    let posts = profile_posts::Entity::find()
        .filter(profile_posts::Column::ProfileUserId.eq(profile_owner.id))
        .order_by_desc(profile_posts::Column::CreatedAt)
        .all(&db)
        .await
        .expect("Failed to fetch posts");

    assert_eq!(posts.len(), 3);
    // Verify order (newest first)
    assert!(posts[0].created_at >= posts[1].created_at);
    assert!(posts[1].created_at >= posts[2].created_at);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_delete_profile_post() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{profile_posts, ugc, ugc_revisions};

    // Create users
    let profile_owner = create_test_user(&db, "profile_owner3", "password123")
        .await
        .expect("Failed to create profile owner");
    let author = create_test_user(&db, "wall_poster3", "password123")
        .await
        .expect("Failed to create author");

    // Create a profile post
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    let revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        ip_id: Set(None),
        user_id: Set(Some(author.id)),
        content: Set("Post to be deleted".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    revision
        .insert(&db)
        .await
        .expect("Failed to create revision");

    let profile_post = profile_posts::ActiveModel {
        profile_user_id: Set(profile_owner.id),
        author_id: Set(Some(author.id)),
        ugc_id: Set(ugc_model.id),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    };
    let post_model = profile_post
        .insert(&db)
        .await
        .expect("Failed to create profile post");

    let post_id = post_model.id;

    // Delete the post
    profile_posts::Entity::delete_by_id(post_id)
        .exec(&db)
        .await
        .expect("Failed to delete profile post");

    // Verify it's gone
    let deleted_post = profile_posts::Entity::find_by_id(post_id)
        .one(&db)
        .await
        .expect("Failed to query");
    assert!(deleted_post.is_none(), "Post should be deleted");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_profile_post_author_set_null_on_user_delete() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{profile_posts, ugc, ugc_revisions, user_names, users};

    // Create profile owner
    let profile_owner = create_test_user(&db, "profile_owner4", "password123")
        .await
        .expect("Failed to create profile owner");

    // Create author (will be deleted)
    let author = create_test_user(&db, "deleted_author", "password123")
        .await
        .expect("Failed to create author");

    // Create a profile post
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    let revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        ip_id: Set(None),
        user_id: Set(Some(author.id)),
        content: Set("Post from soon-deleted user".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    revision
        .insert(&db)
        .await
        .expect("Failed to create revision");

    let profile_post = profile_posts::ActiveModel {
        profile_user_id: Set(profile_owner.id),
        author_id: Set(Some(author.id)),
        ugc_id: Set(ugc_model.id),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    };
    let post_model = profile_post
        .insert(&db)
        .await
        .expect("Failed to create profile post");

    let post_id = post_model.id;

    // Delete the author's user_names first (FK constraint)
    user_names::Entity::delete_many()
        .filter(user_names::Column::UserId.eq(author.id))
        .exec(&db)
        .await
        .expect("Failed to delete user names");

    // Update ugc_revisions to remove user_id reference (FK constraint)
    use sea_orm::sea_query::Expr;
    ugc_revisions::Entity::update_many()
        .col_expr(ugc_revisions::Column::UserId, Expr::value(None::<i32>))
        .filter(ugc_revisions::Column::UserId.eq(author.id))
        .exec(&db)
        .await
        .expect("Failed to update ugc_revisions");

    // Delete the author
    users::Entity::delete_by_id(author.id)
        .exec(&db)
        .await
        .expect("Failed to delete author");

    // Verify the post still exists but author_id is NULL
    let updated_post = profile_posts::Entity::find_by_id(post_id)
        .one(&db)
        .await
        .expect("Failed to query")
        .expect("Post should still exist");

    assert!(
        updated_post.author_id.is_none(),
        "Author ID should be NULL after author deletion"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_profile_posts_cascade_on_profile_owner_delete() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{profile_posts, ugc, ugc_revisions, user_names, users};

    // Create profile owner (will be deleted)
    let profile_owner = create_test_user(&db, "profile_owner5", "password123")
        .await
        .expect("Failed to create profile owner");

    // Create author
    let author = create_test_user(&db, "wall_poster5", "password123")
        .await
        .expect("Failed to create author");

    // Create a profile post
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    let revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        ip_id: Set(None),
        user_id: Set(Some(author.id)),
        content: Set("Post on soon-deleted profile".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    revision
        .insert(&db)
        .await
        .expect("Failed to create revision");

    let profile_post = profile_posts::ActiveModel {
        profile_user_id: Set(profile_owner.id),
        author_id: Set(Some(author.id)),
        ugc_id: Set(ugc_model.id),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    };
    let post_model = profile_post
        .insert(&db)
        .await
        .expect("Failed to create profile post");

    let post_id = post_model.id;

    // Delete the profile owner's user_names first (FK constraint)
    user_names::Entity::delete_many()
        .filter(user_names::Column::UserId.eq(profile_owner.id))
        .exec(&db)
        .await
        .expect("Failed to delete user names");

    // Delete the profile owner
    users::Entity::delete_by_id(profile_owner.id)
        .exec(&db)
        .await
        .expect("Failed to delete profile owner");

    // Verify the post is also deleted (CASCADE)
    let deleted_post = profile_posts::Entity::find_by_id(post_id)
        .one(&db)
        .await
        .expect("Failed to query");

    assert!(
        deleted_post.is_none(),
        "Profile post should be deleted when profile owner is deleted"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_allow_profile_posts_setting() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::users;

    // Create user with default allow_profile_posts (should be true)
    let user = create_test_user(&db, "profile_user6", "password123")
        .await
        .expect("Failed to create user");

    // Verify default is true
    let loaded_user = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .expect("Failed to load user")
        .expect("User not found");

    assert!(
        loaded_user.allow_profile_posts,
        "Default allow_profile_posts should be true"
    );

    // Disable profile posts
    use sea_orm::sea_query::Expr;
    users::Entity::update_many()
        .col_expr(users::Column::AllowProfilePosts, Expr::value(false))
        .filter(users::Column::Id.eq(user.id))
        .exec(&db)
        .await
        .expect("Failed to update user");

    // Verify it's now false
    let updated_user = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .expect("Failed to load user")
        .expect("User not found");

    assert!(
        !updated_user.allow_profile_posts,
        "allow_profile_posts should be false after update"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_profile_post_content_stored_in_ugc() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{profile_posts, ugc, ugc_revisions};

    // Create users
    let profile_owner = create_test_user(&db, "profile_owner7", "password123")
        .await
        .expect("Failed to create profile owner");
    let author = create_test_user(&db, "wall_poster7", "password123")
        .await
        .expect("Failed to create author");

    let test_content = "This is a test wall post with special chars: <script>alert('xss')</script>";

    // Create UGC
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    let revision = ugc_revisions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        ip_id: Set(None),
        user_id: Set(Some(author.id)),
        content: Set(test_content.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    let revision_model = revision
        .insert(&db)
        .await
        .expect("Failed to create revision");

    use sea_orm::sea_query::Expr;
    ugc::Entity::update_many()
        .col_expr(ugc::Column::UgcRevisionId, Expr::value(revision_model.id))
        .filter(ugc::Column::Id.eq(ugc_model.id))
        .exec(&db)
        .await
        .expect("Failed to update UGC");

    let profile_post = profile_posts::ActiveModel {
        profile_user_id: Set(profile_owner.id),
        author_id: Set(Some(author.id)),
        ugc_id: Set(ugc_model.id),
        created_at: Set(Utc::now().into()),
        ..Default::default()
    };
    let post_model = profile_post
        .insert(&db)
        .await
        .expect("Failed to create profile post");

    // Fetch the post and verify content via UGC join
    let post_with_content = profile_posts::Entity::find_by_id(post_model.id)
        .find_also_linked(profile_posts::ProfilePostToUgcRevision)
        .one(&db)
        .await
        .expect("Failed to fetch post with content");

    let (post, revision) = post_with_content.expect("Post not found");
    let revision = revision.expect("Revision not found");

    assert_eq!(revision.content, test_content);
    assert_eq!(post.ugc_id, ugc_model.id);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_multiple_users_can_post_on_same_profile() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{profile_posts, ugc, ugc_revisions};

    // Create profile owner
    let profile_owner = create_test_user(&db, "profile_owner8", "password123")
        .await
        .expect("Failed to create profile owner");

    // Create multiple authors
    let author1 = create_test_user(&db, "wall_poster8a", "password123")
        .await
        .expect("Failed to create author1");
    let author2 = create_test_user(&db, "wall_poster8b", "password123")
        .await
        .expect("Failed to create author2");
    let author3 = create_test_user(&db, "wall_poster8c", "password123")
        .await
        .expect("Failed to create author3");

    // Create posts from each author
    for (author, content) in [
        (&author1, "Hello from author 1"),
        (&author2, "Hello from author 2"),
        (&author3, "Hello from author 3"),
    ] {
        let ugc_entry = ugc::ActiveModel {
            ugc_revision_id: Set(None),
            reaction_count: Set(0),
            ..Default::default()
        };
        let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

        let revision = ugc_revisions::ActiveModel {
            ugc_id: Set(ugc_model.id),
            ip_id: Set(None),
            user_id: Set(Some(author.id)),
            content: Set(content.to_string()),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        };
        revision
            .insert(&db)
            .await
            .expect("Failed to create revision");

        let profile_post = profile_posts::ActiveModel {
            profile_user_id: Set(profile_owner.id),
            author_id: Set(Some(author.id)),
            ugc_id: Set(ugc_model.id),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        };
        profile_post
            .insert(&db)
            .await
            .expect("Failed to create profile post");
    }

    // Fetch all posts on this profile
    let posts = profile_posts::Entity::find()
        .filter(profile_posts::Column::ProfileUserId.eq(profile_owner.id))
        .all(&db)
        .await
        .expect("Failed to fetch posts");

    assert_eq!(posts.len(), 3, "Should have 3 posts from different authors");

    // Verify different authors
    let author_ids: Vec<Option<i32>> = posts.iter().map(|p| p.author_id).collect();
    assert!(author_ids.contains(&Some(author1.id)));
    assert!(author_ids.contains(&Some(author2.id)));
    assert!(author_ids.contains(&Some(author3.id)));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
