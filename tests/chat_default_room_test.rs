/// Tests for default chat room setting and user preference
mod common;

use serial_test::serial;

#[actix_rt::test]
#[serial]
async fn test_user_default_room_none_uses_site_default() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::{
        create_test_chat_room, create_test_user, get_user_default_chat_room, set_test_setting,
    };

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    // Create a test user and chat room
    let user = create_test_user(&db, "default_room_test_user", "password123")
        .await
        .unwrap();
    let room = create_test_chat_room(&db, "Test Room")
        .await
        .unwrap();

    // Set site default to this room
    set_test_setting(&db, "chat_default_room", &room.id.to_string())
        .await
        .unwrap();

    // User has no preference (NULL)
    let user_pref = get_user_default_chat_room(&db, user.id).await.unwrap();
    assert!(user_pref.is_none(), "User should have no preference set");

    // The effective default should be the site setting (tested at application layer)
    // Here we just verify the database state is correct

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_user_default_room_overrides_site_default() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::{
        create_test_chat_room, create_test_user, get_user_default_chat_room,
        set_test_setting, set_user_default_chat_room,
    };

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    // Create user and two rooms
    let user = create_test_user(&db, "override_room_user", "password123")
        .await
        .unwrap();
    let room1 = create_test_chat_room(&db, "Room 1").await.unwrap();
    let room2 = create_test_chat_room(&db, "Room 2").await.unwrap();

    // Set site default to room 1
    set_test_setting(&db, "chat_default_room", &room1.id.to_string())
        .await
        .unwrap();

    // Set user preference to room 2
    set_user_default_chat_room(&db, user.id, Some(room2.id))
        .await
        .unwrap();

    // Verify user preference is set
    let user_pref = get_user_default_chat_room(&db, user.id).await.unwrap();
    assert_eq!(user_pref, Some(room2.id), "User preference should be room 2");

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_set_user_default_room() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::{
        create_test_chat_room, create_test_user, get_user_default_chat_room,
        set_user_default_chat_room,
    };

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    let user = create_test_user(&db, "set_room_user", "password123")
        .await
        .unwrap();
    let room = create_test_chat_room(&db, "Preferred Room").await.unwrap();

    // Initially no preference
    let pref = get_user_default_chat_room(&db, user.id).await.unwrap();
    assert!(pref.is_none());

    // Set preference
    set_user_default_chat_room(&db, user.id, Some(room.id))
        .await
        .unwrap();

    // Verify it was set
    let pref = get_user_default_chat_room(&db, user.id).await.unwrap();
    assert_eq!(pref, Some(room.id));

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_clear_user_default_room() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::{
        create_test_chat_room, create_test_user, get_user_default_chat_room,
        set_user_default_chat_room,
    };

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    let user = create_test_user(&db, "clear_room_user", "password123")
        .await
        .unwrap();
    let room = create_test_chat_room(&db, "Temporary Room").await.unwrap();

    // Set a preference
    set_user_default_chat_room(&db, user.id, Some(room.id))
        .await
        .unwrap();
    let pref = get_user_default_chat_room(&db, user.id).await.unwrap();
    assert_eq!(pref, Some(room.id));

    // Clear preference (set to None)
    set_user_default_chat_room(&db, user.id, None).await.unwrap();

    // Verify it was cleared
    let pref = get_user_default_chat_room(&db, user.id).await.unwrap();
    assert!(pref.is_none(), "Preference should be cleared");

    cleanup_test_data(&db).await.unwrap();
}
