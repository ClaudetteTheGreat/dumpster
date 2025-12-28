//! Integration tests for user follows/followers

mod common;
use serial_test::serial;

use common::{database::*, fixtures::*};
use sea_orm::{entity::*, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

#[actix_rt::test]
#[serial]
async fn test_create_follow_relationship() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::user_follows;

    // Create two users
    let follower = create_test_user(&db, "follower_user", "password123")
        .await
        .expect("Failed to create follower");

    let following = create_test_user(&db, "following_user", "password123")
        .await
        .expect("Failed to create following user");

    // Create follow relationship
    let follow = user_follows::ActiveModel {
        follower_id: Set(follower.id),
        following_id: Set(following.id),
        ..Default::default()
    };
    let follow_model = follow.insert(&db).await.expect("Failed to create follow");

    // Verify the follow was created
    assert!(follow_model.id > 0);
    assert_eq!(follow_model.follower_id, follower.id);
    assert_eq!(follow_model.following_id, following.id);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_follow_count_trigger_on_insert() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{user_follows, users};

    // Create two users
    let follower = create_test_user(&db, "count_follower", "password123")
        .await
        .expect("Failed to create follower");

    let following = create_test_user(&db, "count_following", "password123")
        .await
        .expect("Failed to create following user");

    // Verify initial counts are 0
    let follower_before = users::Entity::find_by_id(follower.id)
        .one(&db)
        .await
        .expect("Query failed")
        .expect("User not found");
    assert_eq!(follower_before.following_count, 0);

    let following_before = users::Entity::find_by_id(following.id)
        .one(&db)
        .await
        .expect("Query failed")
        .expect("User not found");
    assert_eq!(following_before.follower_count, 0);

    // Create follow relationship
    let follow = user_follows::ActiveModel {
        follower_id: Set(follower.id),
        following_id: Set(following.id),
        ..Default::default()
    };
    follow.insert(&db).await.expect("Failed to create follow");

    // Verify counts were updated by trigger
    let follower_after = users::Entity::find_by_id(follower.id)
        .one(&db)
        .await
        .expect("Query failed")
        .expect("User not found");
    assert_eq!(follower_after.following_count, 1);

    let following_after = users::Entity::find_by_id(following.id)
        .one(&db)
        .await
        .expect("Query failed")
        .expect("User not found");
    assert_eq!(following_after.follower_count, 1);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_follow_count_trigger_on_delete() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{user_follows, users};

    // Create two users
    let follower = create_test_user(&db, "delete_follower", "password123")
        .await
        .expect("Failed to create follower");

    let following = create_test_user(&db, "delete_following", "password123")
        .await
        .expect("Failed to create following user");

    // Create follow relationship
    let follow = user_follows::ActiveModel {
        follower_id: Set(follower.id),
        following_id: Set(following.id),
        ..Default::default()
    };
    let follow_model = follow.insert(&db).await.expect("Failed to create follow");

    // Delete the follow
    user_follows::Entity::delete_by_id(follow_model.id)
        .exec(&db)
        .await
        .expect("Failed to delete follow");

    // Verify counts were decremented by trigger
    let follower_after = users::Entity::find_by_id(follower.id)
        .one(&db)
        .await
        .expect("Query failed")
        .expect("User not found");
    assert_eq!(follower_after.following_count, 0);

    let following_after = users::Entity::find_by_id(following.id)
        .one(&db)
        .await
        .expect("Query failed")
        .expect("User not found");
    assert_eq!(following_after.follower_count, 0);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_cannot_follow_self() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::user_follows;

    // Create a user
    let user = create_test_user(&db, "self_follow_user", "password123")
        .await
        .expect("Failed to create user");

    // Try to follow self - should fail due to constraint
    let follow = user_follows::ActiveModel {
        follower_id: Set(user.id),
        following_id: Set(user.id),
        ..Default::default()
    };
    let result = follow.insert(&db).await;

    // Should fail due to no_self_follow constraint
    assert!(result.is_err());

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_cannot_follow_twice() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::user_follows;

    // Create two users
    let follower = create_test_user(&db, "dup_follower", "password123")
        .await
        .expect("Failed to create follower");

    let following = create_test_user(&db, "dup_following", "password123")
        .await
        .expect("Failed to create following user");

    // Create first follow relationship
    let follow1 = user_follows::ActiveModel {
        follower_id: Set(follower.id),
        following_id: Set(following.id),
        ..Default::default()
    };
    follow1
        .insert(&db)
        .await
        .expect("Failed to create first follow");

    // Try to follow again - should fail due to unique constraint
    let follow2 = user_follows::ActiveModel {
        follower_id: Set(follower.id),
        following_id: Set(following.id),
        ..Default::default()
    };
    let result = follow2.insert(&db).await;

    // Should fail due to unique_follow constraint
    assert!(result.is_err());

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_follow_cascade_delete_on_follower_delete() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{ugc_revisions, user_follows, user_names, users};

    // Create two users
    let follower = create_test_user(&db, "cascade_follower", "password123")
        .await
        .expect("Failed to create follower");

    let following = create_test_user(&db, "cascade_following", "password123")
        .await
        .expect("Failed to create following user");

    // Create follow relationship
    let follow = user_follows::ActiveModel {
        follower_id: Set(follower.id),
        following_id: Set(following.id),
        ..Default::default()
    };
    let follow_model = follow.insert(&db).await.expect("Failed to create follow");

    // Update any ugc_revisions to not reference this user before deletion
    ugc_revisions::Entity::update_many()
        .col_expr(
            ugc_revisions::Column::UserId,
            sea_orm::sea_query::Expr::value(Option::<i32>::None),
        )
        .filter(ugc_revisions::Column::UserId.eq(follower.id))
        .exec(&db)
        .await
        .expect("Failed to update ugc_revisions");

    // Delete user_name entry first (FK constraint)
    user_names::Entity::delete_many()
        .filter(user_names::Column::UserId.eq(follower.id))
        .exec(&db)
        .await
        .expect("Failed to delete user_name");

    // Delete the follower user
    users::Entity::delete_by_id(follower.id)
        .exec(&db)
        .await
        .expect("Failed to delete follower");

    // Verify follow relationship was cascade deleted
    let follow_check = user_follows::Entity::find_by_id(follow_model.id)
        .one(&db)
        .await
        .expect("Query failed");
    assert!(follow_check.is_none());

    // Verify following user's follower count was decremented
    let following_after = users::Entity::find_by_id(following.id)
        .one(&db)
        .await
        .expect("Query failed")
        .expect("User not found");
    assert_eq!(following_after.follower_count, 0);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_multiple_followers() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{user_follows, users};

    // Create one popular user and multiple followers
    let popular = create_test_user(&db, "popular_user", "password123")
        .await
        .expect("Failed to create popular user");

    let follower1 = create_test_user(&db, "fan1", "password123")
        .await
        .expect("Failed to create follower 1");

    let follower2 = create_test_user(&db, "fan2", "password123")
        .await
        .expect("Failed to create follower 2");

    let follower3 = create_test_user(&db, "fan3", "password123")
        .await
        .expect("Failed to create follower 3");

    // All three follow the popular user
    for follower in [&follower1, &follower2, &follower3] {
        let follow = user_follows::ActiveModel {
            follower_id: Set(follower.id),
            following_id: Set(popular.id),
            ..Default::default()
        };
        follow.insert(&db).await.expect("Failed to create follow");
    }

    // Verify popular user has 3 followers
    let popular_after = users::Entity::find_by_id(popular.id)
        .one(&db)
        .await
        .expect("Query failed")
        .expect("User not found");
    assert_eq!(popular_after.follower_count, 3);

    // Verify each follower has 1 following
    for follower in [&follower1, &follower2, &follower3] {
        let f = users::Entity::find_by_id(follower.id)
            .one(&db)
            .await
            .expect("Query failed")
            .expect("User not found");
        assert_eq!(f.following_count, 1);
    }

    // Verify we can query all followers
    let followers = user_follows::Entity::find()
        .filter(user_follows::Column::FollowingId.eq(popular.id))
        .all(&db)
        .await
        .expect("Query failed");
    assert_eq!(followers.len(), 3);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_mutual_follow() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{user_follows, users};

    // Create two users who will follow each other
    let user1 = create_test_user(&db, "mutual1", "password123")
        .await
        .expect("Failed to create user 1");

    let user2 = create_test_user(&db, "mutual2", "password123")
        .await
        .expect("Failed to create user 2");

    // User 1 follows User 2
    let follow1 = user_follows::ActiveModel {
        follower_id: Set(user1.id),
        following_id: Set(user2.id),
        ..Default::default()
    };
    follow1
        .insert(&db)
        .await
        .expect("Failed to create follow 1");

    // User 2 follows User 1
    let follow2 = user_follows::ActiveModel {
        follower_id: Set(user2.id),
        following_id: Set(user1.id),
        ..Default::default()
    };
    follow2
        .insert(&db)
        .await
        .expect("Failed to create follow 2");

    // Both users should have 1 follower and 1 following
    let user1_after = users::Entity::find_by_id(user1.id)
        .one(&db)
        .await
        .expect("Query failed")
        .expect("User not found");
    assert_eq!(user1_after.follower_count, 1);
    assert_eq!(user1_after.following_count, 1);

    let user2_after = users::Entity::find_by_id(user2.id)
        .one(&db)
        .await
        .expect("Query failed")
        .expect("User not found");
    assert_eq!(user2_after.follower_count, 1);
    assert_eq!(user2_after.following_count, 1);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
