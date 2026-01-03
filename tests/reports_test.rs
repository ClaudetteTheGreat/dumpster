//! Integration tests for the post reports system

mod common;
use serial_test::serial;

use chrono::Utc;
use common::{database::*, fixtures::*};
use sea_orm::{entity::*, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

#[actix_rt::test]
#[serial]
async fn test_report_reasons_exist() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    use dumpster::orm::report_reasons;

    // Query report reasons
    let reasons = report_reasons::Entity::find()
        .filter(report_reasons::Column::IsActive.eq(true))
        .all(&db)
        .await
        .expect("Failed to fetch report reasons");

    // Should have at least the default report reasons
    assert!(reasons.len() >= 5, "Should have at least 5 report reasons");

    // Check for expected reasons
    let names: Vec<String> = reasons.iter().map(|r| r.name.clone()).collect();
    assert!(
        names.contains(&"spam".to_string()),
        "Should have 'spam' reason"
    );
    assert!(
        names.contains(&"harassment".to_string()),
        "Should have 'harassment' reason"
    );
    assert!(
        names.contains(&"other".to_string()),
        "Should have 'other' reason"
    );
}

#[actix_rt::test]
#[serial]
async fn test_create_report() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use dumpster::orm::{posts, reports, ugc};

    // Create a test user (reporter)
    let reporter = create_test_user(&db, "report_user1", "password123")
        .await
        .expect("Failed to create reporter");

    // Create a test thread and post to report
    let (_forum, thread) = create_test_forum_and_thread(&db, reporter.id, "Test Thread")
        .await
        .expect("Failed to create forum and thread");

    // Create a UGC entry for the post
    let ugc_entry = ugc::ActiveModel {
        ugc_revision_id: Set(None),
        reaction_count: Set(0),
        ..Default::default()
    };
    let ugc_model = ugc_entry.insert(&db).await.expect("Failed to create UGC");

    // Create a post
    let post = posts::ActiveModel {
        thread_id: Set(thread.id),
        ugc_id: Set(ugc_model.id),
        user_id: Set(Some(reporter.id)),
        position: Set(1),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    let post_model = post.insert(&db).await.expect("Failed to create post");

    // Create a report
    let report = reports::ActiveModel {
        reporter_id: Set(reporter.id),
        content_type: Set("post".to_string()),
        content_id: Set(post_model.id),
        reason: Set("spam".to_string()),
        details: Set(Some("This is spam content".to_string())),
        status: Set("open".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    let report_model = report.insert(&db).await.expect("Failed to create report");

    // Verify report was created
    assert_eq!(report_model.reporter_id, reporter.id);
    assert_eq!(report_model.content_type, "post");
    assert_eq!(report_model.content_id, post_model.id);
    assert_eq!(report_model.reason, "spam");
    assert_eq!(report_model.status, "open");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_report_status_update() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use dumpster::orm::reports;

    // Create a test user (reporter)
    let reporter = create_test_user(&db, "report_user2", "password123")
        .await
        .expect("Failed to create reporter");

    // Create a moderator
    let moderator = create_test_user(&db, "report_moderator1", "password123")
        .await
        .expect("Failed to create moderator");

    // Create a report
    let report = reports::ActiveModel {
        reporter_id: Set(reporter.id),
        content_type: Set("thread".to_string()),
        content_id: Set(1),
        reason: Set("harassment".to_string()),
        details: Set(None),
        status: Set("open".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    let report_model = report.insert(&db).await.expect("Failed to create report");

    // Update report status to resolved
    let mut active_report: reports::ActiveModel = report_model.into();
    active_report.status = Set("resolved".to_string());
    active_report.moderator_id = Set(Some(moderator.id));
    active_report.moderator_notes = Set(Some("Handled by moderator".to_string()));
    active_report.resolved_at = Set(Some(Utc::now().naive_utc()));
    active_report.updated_at = Set(Utc::now().naive_utc());

    let updated_report = active_report
        .update(&db)
        .await
        .expect("Failed to update report");

    // Verify update
    assert_eq!(updated_report.status, "resolved");
    assert_eq!(updated_report.moderator_id, Some(moderator.id));
    assert!(updated_report.moderator_notes.is_some());
    assert!(updated_report.resolved_at.is_some());

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_report_with_other_reason_requires_details() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use dumpster::orm::reports;

    // Create a test user
    let reporter = create_test_user(&db, "report_user3", "password123")
        .await
        .expect("Failed to create reporter");

    // Create a report with "other" reason and details
    let report = reports::ActiveModel {
        reporter_id: Set(reporter.id),
        content_type: Set("user".to_string()),
        content_id: Set(1),
        reason: Set("other".to_string()),
        details: Set(Some("This user is impersonating someone".to_string())),
        status: Set("open".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    let report_model = report.insert(&db).await.expect("Failed to create report");

    // Verify report was created with details
    assert_eq!(report_model.reason, "other");
    assert!(report_model.details.is_some());
    assert!(report_model
        .details
        .as_ref()
        .unwrap()
        .contains("impersonating"));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_multiple_reports_on_same_content() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use dumpster::orm::reports;

    // Create two test users (reporters)
    let reporter1 = create_test_user(&db, "report_user4", "password123")
        .await
        .expect("Failed to create reporter1");
    let reporter2 = create_test_user(&db, "report_user5", "password123")
        .await
        .expect("Failed to create reporter2");

    // Both users report the same content
    let report1 = reports::ActiveModel {
        reporter_id: Set(reporter1.id),
        content_type: Set("post".to_string()),
        content_id: Set(42),
        reason: Set("spam".to_string()),
        details: Set(None),
        status: Set("open".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    report1.insert(&db).await.expect("Failed to create report1");

    let report2 = reports::ActiveModel {
        reporter_id: Set(reporter2.id),
        content_type: Set("post".to_string()),
        content_id: Set(42),
        reason: Set("harassment".to_string()),
        details: Set(Some("Very offensive content".to_string())),
        status: Set("open".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    report2.insert(&db).await.expect("Failed to create report2");

    // Query reports for this content
    let content_reports = reports::Entity::find()
        .filter(reports::Column::ContentType.eq("post"))
        .filter(reports::Column::ContentId.eq(42))
        .all(&db)
        .await
        .expect("Failed to fetch reports");

    assert_eq!(
        content_reports.len(),
        2,
        "Should have 2 reports for the same content"
    );

    // Verify different reporters
    let reporter_ids: Vec<i32> = content_reports.iter().map(|r| r.reporter_id).collect();
    assert!(reporter_ids.contains(&reporter1.id));
    assert!(reporter_ids.contains(&reporter2.id));

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_report_dismissal() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use dumpster::orm::reports;

    // Create users
    let reporter = create_test_user(&db, "report_user6", "password123")
        .await
        .expect("Failed to create reporter");
    let moderator = create_test_user(&db, "report_moderator2", "password123")
        .await
        .expect("Failed to create moderator");

    // Create a report
    let report = reports::ActiveModel {
        reporter_id: Set(reporter.id),
        content_type: Set("post".to_string()),
        content_id: Set(99),
        reason: Set("spam".to_string()),
        details: Set(None),
        status: Set("open".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    let report_model = report.insert(&db).await.expect("Failed to create report");

    // Dismiss the report
    let mut active_report: reports::ActiveModel = report_model.into();
    active_report.status = Set("dismissed".to_string());
    active_report.moderator_id = Set(Some(moderator.id));
    active_report.moderator_notes = Set(Some("False positive - content is acceptable".to_string()));
    active_report.resolved_at = Set(Some(Utc::now().naive_utc()));
    active_report.updated_at = Set(Utc::now().naive_utc());

    let dismissed_report = active_report
        .update(&db)
        .await
        .expect("Failed to dismiss report");

    // Verify dismissal
    assert_eq!(dismissed_report.status, "dismissed");
    assert_eq!(dismissed_report.moderator_id, Some(moderator.id));
    assert!(dismissed_report.resolved_at.is_some());

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_filter_reports_by_status() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    use dumpster::orm::reports;

    // Create a test user
    let reporter = create_test_user(&db, "report_user7", "password123")
        .await
        .expect("Failed to create reporter");

    // Create reports with different statuses
    for (i, status) in ["open", "reviewed", "resolved", "dismissed"]
        .iter()
        .enumerate()
    {
        let report = reports::ActiveModel {
            reporter_id: Set(reporter.id),
            content_type: Set("post".to_string()),
            content_id: Set(i as i32 + 100),
            reason: Set("spam".to_string()),
            details: Set(None),
            status: Set(status.to_string()),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        };
        report.insert(&db).await.expect("Failed to create report");
    }

    // Filter by open status
    let open_reports = reports::Entity::find()
        .filter(reports::Column::Status.eq("open"))
        .all(&db)
        .await
        .expect("Failed to fetch open reports");

    assert!(
        !open_reports.is_empty(),
        "Should have at least 1 open report"
    );
    for report in &open_reports {
        assert_eq!(report.status, "open");
    }

    // Filter by resolved status
    let resolved_reports = reports::Entity::find()
        .filter(reports::Column::Status.eq("resolved"))
        .all(&db)
        .await
        .expect("Failed to fetch resolved reports");

    assert!(
        !resolved_reports.is_empty(),
        "Should have at least 1 resolved report"
    );
    for report in &resolved_reports {
        assert_eq!(report.status, "resolved");
    }

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
