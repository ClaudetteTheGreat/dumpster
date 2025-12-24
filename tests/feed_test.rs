//! RSS Feed integration tests

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
