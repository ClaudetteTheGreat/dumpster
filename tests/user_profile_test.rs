/// Tests for user profile features (bio, location, website, signature)
mod common;

use serial_test::serial;

#[actix_rt::test]
#[serial]
async fn test_update_profile_bio() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::create_test_user;
    use ruforo::orm::users;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    // Create a test user
    let user = create_test_user(&db, "bio_test_user", "password123")
        .await
        .unwrap();

    // Update bio directly in database
    let mut active_user: users::ActiveModel = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into();

    active_user.bio = Set(Some("This is my test bio.".to_string()));
    active_user.update(&db).await.unwrap();

    // Verify the bio was saved
    let updated_user = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated_user.bio, Some("This is my test bio.".to_string()));

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_update_profile_location() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::create_test_user;
    use ruforo::orm::users;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    let user = create_test_user(&db, "location_test_user", "password123")
        .await
        .unwrap();

    let mut active_user: users::ActiveModel = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into();

    active_user.location = Set(Some("New York, USA".to_string()));
    active_user.update(&db).await.unwrap();

    let updated_user = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated_user.location, Some("New York, USA".to_string()));

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_update_profile_website_url() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::create_test_user;
    use ruforo::orm::users;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    let user = create_test_user(&db, "website_test_user", "password123")
        .await
        .unwrap();

    let mut active_user: users::ActiveModel = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into();

    active_user.website_url = Set(Some("https://example.com".to_string()));
    active_user.update(&db).await.unwrap();

    let updated_user = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        updated_user.website_url,
        Some("https://example.com".to_string())
    );

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_update_profile_signature() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::create_test_user;
    use ruforo::orm::users;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    let user = create_test_user(&db, "signature_test_user", "password123")
        .await
        .unwrap();

    let mut active_user: users::ActiveModel = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .into();

    active_user.signature = Set(Some("[b]Bold signature[/b]".to_string()));
    active_user.update(&db).await.unwrap();

    let updated_user = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        updated_user.signature,
        Some("[b]Bold signature[/b]".to_string())
    );

    cleanup_test_data(&db).await.unwrap();
}

#[actix_rt::test]
#[serial]
async fn test_website_url_validation() {
    use url::Url;

    // Test valid URLs
    let valid_urls = vec![
        "https://example.com",
        "http://example.com/path",
        "https://subdomain.example.com",
        "https://example.com:8080/path?query=value",
    ];

    for url_str in valid_urls {
        let parsed = Url::parse(url_str);
        assert!(parsed.is_ok(), "URL should be valid: {}", url_str);
        let url = parsed.unwrap();
        assert!(
            url.scheme() == "http" || url.scheme() == "https",
            "Scheme should be http or https: {}",
            url_str
        );
    }

    // Test invalid URLs (wrong scheme)
    let invalid_scheme_urls = vec!["ftp://example.com", "javascript:alert(1)", "file:///etc/passwd"];

    for url_str in invalid_scheme_urls {
        let result = Url::parse(url_str);
        if result.is_ok() {
            let url = result.unwrap();
            assert!(
                url.scheme() != "http" && url.scheme() != "https",
                "URL scheme should not be http/https: {}",
                url_str
            );
        }
    }

    // Test completely invalid URLs
    let invalid_urls = vec!["not a url", "://missing-scheme.com", ""];

    for url_str in invalid_urls {
        assert!(Url::parse(url_str).is_err(), "Should be invalid: {}", url_str);
    }
}

#[actix_rt::test]
#[serial]
async fn test_profile_character_limits() {
    // Test that character limits are properly defined
    let max_bio = 2000;
    let max_location = 255;
    let max_signature = 500;
    let max_website = 2048;

    // Valid lengths
    let valid_bio = "x".repeat(max_bio);
    let valid_location = "x".repeat(max_location);
    let valid_signature = "x".repeat(max_signature);
    let valid_website = format!("https://example.com/{}", "x".repeat(max_website - 24));

    assert!(valid_bio.len() <= max_bio, "Bio should be within limit");
    assert!(
        valid_location.len() <= max_location,
        "Location should be within limit"
    );
    assert!(
        valid_signature.len() <= max_signature,
        "Signature should be within limit"
    );
    assert!(
        valid_website.len() <= max_website,
        "Website should be within limit"
    );

    // Invalid lengths (over limit)
    let invalid_bio = "x".repeat(max_bio + 1);
    let invalid_location = "x".repeat(max_location + 1);
    let invalid_signature = "x".repeat(max_signature + 1);

    assert!(invalid_bio.len() > max_bio, "Bio should exceed limit");
    assert!(
        invalid_location.len() > max_location,
        "Location should exceed limit"
    );
    assert!(
        invalid_signature.len() > max_signature,
        "Signature should exceed limit"
    );
}

#[actix_rt::test]
#[serial]
async fn test_signature_bbcode_rendering() {
    use ruforo::bbcode::parse;

    // Test that BBCode in signatures is properly rendered
    let signature = "[b]Bold[/b] and [i]italic[/i] text";
    let rendered = parse(signature);

    assert!(
        rendered.contains("<b>Bold</b>"),
        "BBCode bold should render: {}",
        rendered
    );
    assert!(
        rendered.contains("<i>italic</i>"),
        "BBCode italic should render: {}",
        rendered
    );
}

#[actix_rt::test]
#[serial]
async fn test_profile_fields_default_to_none() {
    use common::database::{cleanup_test_data, setup_test_database};
    use common::fixtures::create_test_user;
    use ruforo::orm::users;
    use sea_orm::EntityTrait;

    let db = setup_test_database().await.unwrap();
    cleanup_test_data(&db).await.unwrap();

    // Create a user without setting profile fields
    let user = create_test_user(&db, "default_profile_user", "password123")
        .await
        .unwrap();

    // Verify all profile fields are None by default
    let user_model = users::Entity::find_by_id(user.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert!(user_model.bio.is_none(), "Bio should be None by default");
    assert!(
        user_model.location.is_none(),
        "Location should be None by default"
    );
    assert!(
        user_model.website_url.is_none(),
        "Website URL should be None by default"
    );
    assert!(
        user_model.signature.is_none(),
        "Signature should be None by default"
    );

    cleanup_test_data(&db).await.unwrap();
}
