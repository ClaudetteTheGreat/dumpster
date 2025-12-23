//! Integration tests for the post reactions system

mod common;
use serial_test::serial;

use chrono::Utc;
use common::{database::*, fixtures::*};
use sea_orm::{entity::*, ActiveValue::Set, EntityTrait, QueryFilter, ColumnTrait};

#[actix_rt::test]
#[serial]
async fn test_reaction_types_exist() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    use ruforo::orm::reaction_types;

    // Query reaction types
    let types = reaction_types::Entity::find()
        .filter(reaction_types::Column::IsActive.eq(true))
        .all(&db)
        .await
        .expect("Failed to fetch reaction types");

    // Should have at least the 6 default reaction types
    assert!(types.len() >= 6, "Should have at least 6 reaction types");

    // Check for expected reactions
    let names: Vec<String> = types.iter().map(|t| t.name.clone()).collect();
    assert!(names.contains(&"like".to_string()), "Should have 'like' reaction");
    assert!(names.contains(&"thanks".to_string()), "Should have 'thanks' reaction");
    assert!(names.contains(&"funny".to_string()), "Should have 'funny' reaction");
}

#[actix_rt::test]
#[serial]
async fn test_add_reaction_to_post() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{ugc, ugc_reactions};

    // Create a test user
    let user = create_test_user(&db, "reaction_user1", "password123")
        .await
        .expect("Failed to create user");

    // Create a test UGC entry (simulating a post)
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    // Add a reaction
    let reaction = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(user.id),
        reaction_type_id: Set(1), // "like"
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    reaction.insert(&db).await.expect("Failed to add reaction");

    // Verify reaction was added
    let reactions = ugc_reactions::Entity::find()
        .filter(ugc_reactions::Column::UgcId.eq(ugc_model.id))
        .all(&db)
        .await
        .expect("Failed to fetch reactions");

    assert_eq!(reactions.len(), 1, "Should have 1 reaction");
    assert_eq!(reactions[0].user_id, user.id);
    assert_eq!(reactions[0].reaction_type_id, 1);

    // Check that the trigger updated the reaction count
    let updated_ugc = ugc::Entity::find_by_id(ugc_model.id)
        .one(&db)
        .await
        .expect("Failed to fetch UGC")
        .expect("UGC not found");

    assert_eq!(updated_ugc.reaction_count, 1, "Reaction count should be 1");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_remove_reaction() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{ugc, ugc_reactions};

    // Create a test user
    let user = create_test_user(&db, "reaction_user2", "password123")
        .await
        .expect("Failed to create user");

    // Create a test UGC entry
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    // Add a reaction
    let reaction = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(user.id),
        reaction_type_id: Set(1),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    let reaction_model = reaction.insert(&db).await.expect("Failed to add reaction");

    // Remove the reaction
    ugc_reactions::Entity::delete_by_id(reaction_model.id)
        .exec(&db)
        .await
        .expect("Failed to delete reaction");

    // Verify reaction was removed
    let reactions = ugc_reactions::Entity::find()
        .filter(ugc_reactions::Column::UgcId.eq(ugc_model.id))
        .all(&db)
        .await
        .expect("Failed to fetch reactions");

    assert_eq!(reactions.len(), 0, "Should have 0 reactions after removal");

    // Check that the trigger updated the reaction count
    let updated_ugc = ugc::Entity::find_by_id(ugc_model.id)
        .one(&db)
        .await
        .expect("Failed to fetch UGC")
        .expect("UGC not found");

    assert_eq!(updated_ugc.reaction_count, 0, "Reaction count should be 0 after removal");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_multiple_users_can_react() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{ugc, ugc_reactions};

    // Create two test users
    let user1 = create_test_user(&db, "reaction_user3", "password123")
        .await
        .expect("Failed to create user1");
    let user2 = create_test_user(&db, "reaction_user4", "password123")
        .await
        .expect("Failed to create user2");

    // Create a test UGC entry
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    // Both users add "like" reactions
    let reaction1 = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(user1.id),
        reaction_type_id: Set(1),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    reaction1.insert(&db).await.expect("Failed to add reaction1");

    let reaction2 = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(user2.id),
        reaction_type_id: Set(1),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    reaction2.insert(&db).await.expect("Failed to add reaction2");

    // Check reaction count
    let updated_ugc = ugc::Entity::find_by_id(ugc_model.id)
        .one(&db)
        .await
        .expect("Failed to fetch UGC")
        .expect("UGC not found");

    assert_eq!(updated_ugc.reaction_count, 2, "Reaction count should be 2");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_user_can_add_different_reaction_types() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{ugc, ugc_reactions};

    // Create a test user
    let user = create_test_user(&db, "reaction_user5", "password123")
        .await
        .expect("Failed to create user");

    // Create a test UGC entry
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    // Add "like" reaction (type 1)
    let reaction1 = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(user.id),
        reaction_type_id: Set(1),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    reaction1.insert(&db).await.expect("Failed to add like");

    // Add "thanks" reaction (type 2)
    let reaction2 = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(user.id),
        reaction_type_id: Set(2),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    reaction2.insert(&db).await.expect("Failed to add thanks");

    // Verify both reactions exist
    let reactions = ugc_reactions::Entity::find()
        .filter(ugc_reactions::Column::UgcId.eq(ugc_model.id))
        .filter(ugc_reactions::Column::UserId.eq(user.id))
        .all(&db)
        .await
        .expect("Failed to fetch reactions");

    assert_eq!(reactions.len(), 2, "User should have 2 different reactions");

    let reaction_types: Vec<i32> = reactions.iter().map(|r| r.reaction_type_id).collect();
    assert!(reaction_types.contains(&1), "Should have 'like' reaction");
    assert!(reaction_types.contains(&2), "Should have 'thanks' reaction");

    // Check total reaction count
    let updated_ugc = ugc::Entity::find_by_id(ugc_model.id)
        .one(&db)
        .await
        .expect("Failed to fetch UGC")
        .expect("UGC not found");

    assert_eq!(updated_ugc.reaction_count, 2, "Reaction count should be 2");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_unique_constraint_prevents_duplicate() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{ugc, ugc_reactions};

    // Create a test user
    let user = create_test_user(&db, "reaction_user6", "password123")
        .await
        .expect("Failed to create user");

    // Create a test UGC entry
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    // Add a reaction
    let reaction1 = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(user.id),
        reaction_type_id: Set(1),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    reaction1.insert(&db).await.expect("Failed to add first reaction");

    // Try to add the same reaction again (should fail)
    let reaction2 = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(user.id),
        reaction_type_id: Set(1),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    let result = reaction2.insert(&db).await;

    assert!(result.is_err(), "Duplicate reaction should fail due to unique constraint");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
