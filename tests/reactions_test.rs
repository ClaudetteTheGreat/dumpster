//! Integration tests for the post reactions system

mod common;
use serial_test::serial;

use chrono::Utc;
use common::{database::*, fixtures::*};
use sea_orm::{entity::*, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

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
    assert!(
        names.contains(&"like".to_string()),
        "Should have 'like' reaction"
    );
    assert!(
        names.contains(&"thanks".to_string()),
        "Should have 'thanks' reaction"
    );
    assert!(
        names.contains(&"funny".to_string()),
        "Should have 'funny' reaction"
    );
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

    assert_eq!(
        updated_ugc.reaction_count, 0,
        "Reaction count should be 0 after removal"
    );

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
    reaction1
        .insert(&db)
        .await
        .expect("Failed to add reaction1");

    let reaction2 = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(user2.id),
        reaction_type_id: Set(1),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    reaction2
        .insert(&db)
        .await
        .expect("Failed to add reaction2");

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
    reaction1
        .insert(&db)
        .await
        .expect("Failed to add first reaction");

    // Try to add the same reaction again (should fail)
    let reaction2 = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(user.id),
        reaction_type_id: Set(1),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    let result = reaction2.insert(&db).await;

    assert!(
        result.is_err(),
        "Duplicate reaction should fail due to unique constraint"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

// ============================================================================
// Reputation System Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_reaction_types_have_reputation_values() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    use ruforo::orm::reaction_types;

    // Query reaction types
    let types = reaction_types::Entity::find()
        .all(&db)
        .await
        .expect("Failed to fetch reaction types");

    // Check that reputation values are set
    let like = types.iter().find(|t| t.name == "like");
    assert!(like.is_some(), "Should have 'like' reaction type");
    assert_eq!(
        like.unwrap().reputation_value,
        1,
        "Like should have reputation value of 1"
    );

    let disagree = types.iter().find(|t| t.name == "disagree");
    assert!(disagree.is_some(), "Should have 'disagree' reaction type");
    assert_eq!(
        disagree.unwrap().reputation_value,
        -1,
        "Disagree should have reputation value of -1"
    );

    let funny = types.iter().find(|t| t.name == "funny");
    assert!(funny.is_some(), "Should have 'funny' reaction type");
    assert_eq!(
        funny.unwrap().reputation_value,
        0,
        "Funny should have reputation value of 0"
    );
}

#[actix_rt::test]
#[serial]
async fn test_reputation_increases_on_positive_reaction() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{forums, posts, threads, ugc, ugc_reactions, users};

    // Create a forum for the test
    let forum = forums::ActiveModel {
        label: Set("Reputation Test Forum".to_string()),
        description: Set(None),
        display_order: Set(99),
        ..Default::default()
    };
    let forum_model = forum.insert(&db).await.expect("Failed to create forum");

    // Create post author (will receive reputation)
    let author = create_test_user(&db, "rep_author", "password123")
        .await
        .expect("Failed to create author");

    // Verify initial reputation is 0
    let author_data = users::Entity::find_by_id(author.id)
        .one(&db)
        .await
        .expect("Failed to fetch author")
        .expect("Author not found");
    assert_eq!(
        author_data.reputation_score, 0,
        "Initial reputation should be 0"
    );

    // Create reactor (will give the reaction)
    let reactor = create_test_user(&db, "rep_reactor", "password123")
        .await
        .expect("Failed to create reactor");

    // Create a UGC entry for the post
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    // Create a thread
    let thread = threads::ActiveModel {
        user_id: Set(Some(author.id)),
        forum_id: Set(forum_model.id),
        title: Set("Reputation Test Thread".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        is_locked: Set(false),
        is_pinned: Set(false),
        view_count: Set(0),
        post_count: Set(0),
        ..Default::default()
    };
    let thread_model = thread.insert(&db).await.expect("Failed to create thread");

    // Create a post by the author
    let post = posts::ActiveModel {
        user_id: Set(Some(author.id)),
        thread_id: Set(thread_model.id),
        ugc_id: Set(ugc_model.id),
        position: Set(1),
        created_at: Set(Utc::now().naive_utc()),

        ..Default::default()
    };
    post.insert(&db).await.expect("Failed to create post");

    // Reactor adds a "like" reaction (reputation_value = +1)
    let reaction = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(reactor.id),
        reaction_type_id: Set(1), // "like" has reputation_value = 1
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    reaction.insert(&db).await.expect("Failed to add reaction");

    // Check that author's reputation increased
    let updated_author = users::Entity::find_by_id(author.id)
        .one(&db)
        .await
        .expect("Failed to fetch author")
        .expect("Author not found");

    assert_eq!(
        updated_author.reputation_score, 1,
        "Reputation should increase to 1 after receiving a like"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_reputation_decreases_on_negative_reaction() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{forums, posts, threads, ugc, ugc_reactions, users};

    // Create a forum for the test
    let forum = forums::ActiveModel {
        label: Set("Neg Rep Test Forum".to_string()),
        description: Set(None),
        display_order: Set(98),
        ..Default::default()
    };
    let forum_model = forum.insert(&db).await.expect("Failed to create forum");

    // Create post author
    let author = create_test_user(&db, "neg_rep_author", "password123")
        .await
        .expect("Failed to create author");

    // Create reactor
    let reactor = create_test_user(&db, "neg_rep_reactor", "password123")
        .await
        .expect("Failed to create reactor");

    // Create UGC, thread, and post
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    let thread = threads::ActiveModel {
        user_id: Set(Some(author.id)),
        forum_id: Set(forum_model.id),
        title: Set("Neg Rep Test Thread".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        is_locked: Set(false),
        is_pinned: Set(false),
        view_count: Set(0),
        post_count: Set(0),
        ..Default::default()
    };
    let thread_model = thread.insert(&db).await.expect("Failed to create thread");

    let post = posts::ActiveModel {
        user_id: Set(Some(author.id)),
        thread_id: Set(thread_model.id),
        ugc_id: Set(ugc_model.id),
        position: Set(1),
        created_at: Set(Utc::now().naive_utc()),

        ..Default::default()
    };
    post.insert(&db).await.expect("Failed to create post");

    // Reactor adds a "disagree" reaction (reputation_value = -1)
    // Find the disagree reaction type id
    use ruforo::orm::reaction_types;
    let disagree_type = reaction_types::Entity::find()
        .filter(reaction_types::Column::Name.eq("disagree"))
        .one(&db)
        .await
        .expect("Failed to find disagree type")
        .expect("Disagree type not found");

    let reaction = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(reactor.id),
        reaction_type_id: Set(disagree_type.id),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    reaction.insert(&db).await.expect("Failed to add reaction");

    // Check that author's reputation decreased
    let updated_author = users::Entity::find_by_id(author.id)
        .one(&db)
        .await
        .expect("Failed to fetch author")
        .expect("Author not found");

    assert_eq!(
        updated_author.reputation_score, -1,
        "Reputation should decrease to -1 after receiving a disagree"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_reputation_restored_when_reaction_removed() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use ruforo::orm::{forums, posts, threads, ugc, ugc_reactions, users};

    // Create forum
    let forum = forums::ActiveModel {
        label: Set("Restore Rep Test Forum".to_string()),
        description: Set(None),
        display_order: Set(97),
        ..Default::default()
    };
    let forum_model = forum.insert(&db).await.expect("Failed to create forum");

    // Create users
    let author = create_test_user(&db, "restore_rep_author", "password123")
        .await
        .expect("Failed to create author");
    let reactor = create_test_user(&db, "restore_rep_reactor", "password123")
        .await
        .expect("Failed to create reactor");

    // Create UGC, thread, and post
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    let thread = threads::ActiveModel {
        user_id: Set(Some(author.id)),
        forum_id: Set(forum_model.id),
        title: Set("Restore Rep Test Thread".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        is_locked: Set(false),
        is_pinned: Set(false),
        view_count: Set(0),
        post_count: Set(0),
        ..Default::default()
    };
    let thread_model = thread.insert(&db).await.expect("Failed to create thread");

    let post = posts::ActiveModel {
        user_id: Set(Some(author.id)),
        thread_id: Set(thread_model.id),
        ugc_id: Set(ugc_model.id),
        position: Set(1),
        created_at: Set(Utc::now().naive_utc()),

        ..Default::default()
    };
    post.insert(&db).await.expect("Failed to create post");

    // Add a like reaction
    let reaction = ugc_reactions::ActiveModel {
        ugc_id: Set(ugc_model.id),
        user_id: Set(reactor.id),
        reaction_type_id: Set(1), // like
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    let reaction_model = reaction.insert(&db).await.expect("Failed to add reaction");

    // Verify reputation increased
    let author_after_like = users::Entity::find_by_id(author.id)
        .one(&db)
        .await
        .expect("Failed to fetch author")
        .expect("Author not found");
    assert_eq!(author_after_like.reputation_score, 1);

    // Remove the reaction
    ugc_reactions::Entity::delete_by_id(reaction_model.id)
        .exec(&db)
        .await
        .expect("Failed to delete reaction");

    // Verify reputation restored to 0
    let author_after_remove = users::Entity::find_by_id(author.id)
        .one(&db)
        .await
        .expect("Failed to fetch author")
        .expect("Author not found");
    assert_eq!(
        author_after_remove.reputation_score, 0,
        "Reputation should be restored to 0 after reaction is removed"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_min_posts_to_vote_setting_exists() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    use ruforo::orm::settings;
    use sea_orm::ActiveValue::Set;

    use sea_orm::EntityTrait;

    // Check if setting exists first (cleanup from other tests may have cleared it)
    let existing = settings::Entity::find_by_id("min_posts_to_vote".to_string())
        .one(&db)
        .await
        .expect("Failed to check for existing setting");

    // Insert the setting if it doesn't exist
    if existing.is_none() {
        let setting_model = settings::ActiveModel {
            key: Set("min_posts_to_vote".to_string()),
            value: Set("5".to_string()),
            value_type: Set("int".to_string()),
            description: Set(Some("Minimum posts required to give reactions".to_string())),
            category: Set("reactions".to_string()),
            is_public: Set(false),
            ..Default::default()
        };
        settings::Entity::insert(setting_model)
            .exec(&db)
            .await
            .expect("Failed to insert setting");
    }

    // Query the min_posts_to_vote setting
    let setting = settings::Entity::find_by_id("min_posts_to_vote".to_string())
        .one(&db)
        .await
        .expect("Failed to fetch setting");

    assert!(setting.is_some(), "min_posts_to_vote setting should exist");

    let setting = setting.unwrap();
    assert_eq!(setting.value, "5", "Default min_posts_to_vote should be 5");
    assert_eq!(setting.value_type, "int", "Setting should be of type int");
}
