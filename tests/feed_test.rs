//! RSS and Atom Feed integration tests

mod common;

use actix_web::{test, web, App};
use serial_test::serial;

#[actix_rt::test]
#[serial]
async fn test_latest_threads_feed_returns_rss() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/feed.rss").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/rss+xml; charset=utf-8"
    );

    // Parse response body and verify it's valid RSS
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("<rss"));
    assert!(body_str.contains("<channel>"));
    assert!(body_str.contains("Latest Threads"));
}

#[actix_rt::test]
#[serial]
async fn test_latest_threads_feed_includes_threads() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    // Create a test user and thread
    let user = common::fixtures::create_test_user(&db, "feeduser", "password123")
        .await
        .expect("Failed to create test user");

    let (_forum, _thread) =
        common::fixtures::create_test_forum_and_thread(&db, user.id, "Test Thread for Feed")
            .await
            .expect("Failed to create test thread");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/feed.rss").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Test Thread for Feed"));
}

#[actix_rt::test]
#[serial]
async fn test_forum_feed_returns_rss() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    // Create a test user and forum with thread
    let user = common::fixtures::create_test_user(&db, "forumfeeduser", "password123")
        .await
        .expect("Failed to create test user");

    let (forum, _thread) =
        common::fixtures::create_test_forum_and_thread(&db, user.id, "Forum Thread for Feed")
            .await
            .expect("Failed to create test thread");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/forums/{}/feed.rss", forum.id))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/rss+xml; charset=utf-8"
    );

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("<rss"));
    assert!(body_str.contains("Test Forum"));
    assert!(body_str.contains("Forum Thread for Feed"));
}

#[actix_rt::test]
#[serial]
async fn test_forum_feed_404_for_nonexistent_forum() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/forums/99999/feed.rss")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[actix_rt::test]
#[serial]
async fn test_feed_items_have_required_elements() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    // Create a test user and thread
    let user = common::fixtures::create_test_user(&db, "rssuser", "password123")
        .await
        .expect("Failed to create test user");

    let (_forum, _thread) =
        common::fixtures::create_test_forum_and_thread(&db, user.id, "RSS Item Test Thread")
            .await
            .expect("Failed to create test thread");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/feed.rss").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Check that feed items have required RSS elements
    assert!(body_str.contains("<item>"));
    assert!(body_str.contains("<title>"));
    assert!(body_str.contains("<link>"));
    assert!(body_str.contains("<guid"));
    assert!(body_str.contains("<pubDate>"));
}

// ============================================================================
// Atom Feed Tests
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_latest_threads_atom_feed_returns_atom() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/feed.atom").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/atom+xml; charset=utf-8"
    );

    // Parse response body and verify it's valid Atom
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("<feed"));
    assert!(body_str.contains("xmlns=\"http://www.w3.org/2005/Atom\""));
    assert!(body_str.contains("Latest Threads"));
}

#[actix_rt::test]
#[serial]
async fn test_latest_threads_atom_feed_includes_threads() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    // Create a test user and thread
    let user = common::fixtures::create_test_user(&db, "atomfeeduser", "password123")
        .await
        .expect("Failed to create test user");

    let (_forum, _thread) =
        common::fixtures::create_test_forum_and_thread(&db, user.id, "Test Thread for Atom Feed")
            .await
            .expect("Failed to create test thread");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/feed.atom").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Test Thread for Atom Feed"));
}

#[actix_rt::test]
#[serial]
async fn test_forum_atom_feed_returns_atom() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    // Create a test user and forum with thread
    let user = common::fixtures::create_test_user(&db, "forumatomfeeduser", "password123")
        .await
        .expect("Failed to create test user");

    let (forum, _thread) =
        common::fixtures::create_test_forum_and_thread(&db, user.id, "Forum Thread for Atom Feed")
            .await
            .expect("Failed to create test thread");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/forums/{}/feed.atom", forum.id))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/atom+xml; charset=utf-8"
    );

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("<feed"));
    assert!(body_str.contains("Test Forum"));
    assert!(body_str.contains("Forum Thread for Atom Feed"));
}

#[actix_rt::test]
#[serial]
async fn test_forum_atom_feed_404_for_nonexistent_forum() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/forums/99999/feed.atom")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[actix_rt::test]
#[serial]
async fn test_atom_feed_entries_have_required_elements() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    // Create a test user and thread
    let user = common::fixtures::create_test_user(&db, "atomuser", "password123")
        .await
        .expect("Failed to create test user");

    let (_forum, _thread) =
        common::fixtures::create_test_forum_and_thread(&db, user.id, "Atom Entry Test Thread")
            .await
            .expect("Failed to create test thread");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/feed.atom").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Check that feed entries have required Atom elements
    assert!(body_str.contains("<entry>"));
    assert!(body_str.contains("<id>"));
    assert!(body_str.contains("<title>"));
    assert!(body_str.contains("<link"));
    assert!(body_str.contains("<updated>"));
    assert!(body_str.contains("<summary>"));
}

// ============================================================================
// Thread Feed Tests (Per-Thread Replies)
// ============================================================================

#[actix_rt::test]
#[serial]
async fn test_thread_rss_feed_returns_rss() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    // Create a test user and thread with replies
    let user = common::fixtures::create_test_user(&db, "threadfeeduser", "password123")
        .await
        .expect("Failed to create test user");

    let (_forum, thread) =
        common::fixtures::create_test_forum_and_thread(&db, user.id, "Thread with Replies")
            .await
            .expect("Failed to create test thread");

    // Create some replies (position > 1 to be replies, not the OP)
    common::fixtures::create_test_post(&db, thread.id, user.id, "First reply content", 2)
        .await
        .expect("Failed to create first reply");
    common::fixtures::create_test_post(&db, thread.id, user.id, "Second reply content", 3)
        .await
        .expect("Failed to create second reply");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/threads/{}/feed.rss", thread.id))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/rss+xml; charset=utf-8"
    );

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("<rss"));
    assert!(body_str.contains("<channel>"));
    assert!(body_str.contains("Thread with Replies"));
    assert!(body_str.contains("First reply content"));
    assert!(body_str.contains("Second reply content"));
}

#[actix_rt::test]
#[serial]
async fn test_thread_atom_feed_returns_atom() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    // Create a test user and thread with replies
    let user = common::fixtures::create_test_user(&db, "threadatomuser", "password123")
        .await
        .expect("Failed to create test user");

    let (_forum, thread) =
        common::fixtures::create_test_forum_and_thread(&db, user.id, "Thread Atom Replies")
            .await
            .expect("Failed to create test thread");

    // Create some replies
    common::fixtures::create_test_post(&db, thread.id, user.id, "Atom reply content", 2)
        .await
        .expect("Failed to create reply");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/threads/{}/feed.atom", thread.id))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/atom+xml; charset=utf-8"
    );

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("<feed"));
    assert!(body_str.contains("xmlns=\"http://www.w3.org/2005/Atom\""));
    assert!(body_str.contains("Thread Atom Replies"));
    assert!(body_str.contains("Atom reply content"));
}

#[actix_rt::test]
#[serial]
async fn test_thread_feed_404_for_nonexistent_thread() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    // Test RSS
    let req = test::TestRequest::get()
        .uri("/threads/99999/feed.rss")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    // Test Atom
    let req = test::TestRequest::get()
        .uri("/threads/99999/feed.atom")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_rt::test]
#[serial]
async fn test_thread_feed_excludes_first_post() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    // Create a test user and thread
    let user = common::fixtures::create_test_user(&db, "threadopuser", "password123")
        .await
        .expect("Failed to create test user");

    let (_forum, thread) =
        common::fixtures::create_test_forum_and_thread(&db, user.id, "Thread OP Test")
            .await
            .expect("Failed to create test thread");

    // Create the original post (position 1) - should NOT appear in feed
    common::fixtures::create_test_post(&db, thread.id, user.id, "Original post content", 1)
        .await
        .expect("Failed to create OP");

    // Create a reply (position 2) - should appear in feed
    common::fixtures::create_test_post(&db, thread.id, user.id, "Reply content here", 2)
        .await
        .expect("Failed to create reply");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/threads/{}/feed.rss", thread.id))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Reply should be in the feed
    assert!(body_str.contains("Reply content here"));
    // Original post should NOT be in the feed
    assert!(!body_str.contains("Original post content"));
}

#[actix_rt::test]
#[serial]
async fn test_thread_feed_includes_author_name() {
    let db = common::database::setup_test_database()
        .await
        .expect("Failed to setup test database");
    common::database::cleanup_test_data(&db)
        .await
        .expect("Failed to cleanup test data");
    ruforo::web::feed::clear_feed_cache();

    // Create a test user with a specific username
    let user = common::fixtures::create_test_user(&db, "ReplyAuthor123", "password123")
        .await
        .expect("Failed to create test user");

    let (_forum, thread) =
        common::fixtures::create_test_forum_and_thread(&db, user.id, "Author Test Thread")
            .await
            .expect("Failed to create test thread");

    // Create a reply
    common::fixtures::create_test_post(&db, thread.id, user.id, "Test reply with author", 2)
        .await
        .expect("Failed to create reply");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .configure(ruforo::web::feed::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/threads/{}/feed.rss", thread.id))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Author name should be in the feed
    assert!(body_str.contains("ReplyAuthor123"));
}
