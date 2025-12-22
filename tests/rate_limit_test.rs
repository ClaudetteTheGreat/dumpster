mod common;

use ruforo::rate_limit::{
    check_login_rate_limit, check_post_rate_limit, check_thread_rate_limit, RateLimiter,
};
use std::time::Duration;

#[test]
fn test_login_rate_limit_allows_within_limit() {
    // Should allow 5 login attempts within 5 minutes
    for i in 0..5 {
        let result = check_login_rate_limit("192.168.1.1", "testuser");
        assert!(
            result.is_ok(),
            "Login attempt {} should be allowed within rate limit",
            i + 1
        );
    }
}

#[test]
fn test_login_rate_limit_blocks_over_limit() {
    let limiter = RateLimiter::new();

    // Use up the 5 allowed attempts
    for _ in 0..5 {
        let result = limiter.check_rate_limit(
            "login_test",
            "192.168.1.2:testuser2",
            5,
            Duration::from_secs(300),
        );
        assert!(result.is_ok(), "Should allow requests within limit");
    }

    // 6th attempt should be blocked
    let result = limiter.check_rate_limit(
        "login_test",
        "192.168.1.2:testuser2",
        5,
        Duration::from_secs(300),
    );

    assert!(result.is_err(), "6th login attempt should be blocked");

    if let Err(err) = result {
        assert!(
            err.retry_after_seconds > 0,
            "Should provide retry_after time"
        );
        assert!(
            err.retry_after_seconds <= 300,
            "Retry time should be within rate limit window"
        );
    }
}

#[test]
fn test_post_rate_limit() {
    // Should allow 10 posts per minute
    for i in 0..10 {
        let result = check_post_rate_limit(123);
        assert!(
            result.is_ok(),
            "Post {} should be allowed within rate limit",
            i + 1
        );
    }

    // 11th post should be blocked
    let result = check_post_rate_limit(123);
    assert!(result.is_err(), "11th post should be blocked");
}

#[test]
fn test_thread_rate_limit() {
    // Should allow 5 threads per 5 minutes
    for i in 0..5 {
        let result = check_thread_rate_limit(456);
        assert!(
            result.is_ok(),
            "Thread {} should be allowed within rate limit",
            i + 1
        );
    }

    // 6th thread should be blocked
    let result = check_thread_rate_limit(456);
    assert!(result.is_err(), "6th thread should be blocked");
}

#[test]
fn test_different_users_independent_limits() {
    // User 1 uses up their post limit
    for _ in 0..10 {
        check_post_rate_limit(100).unwrap();
    }

    // User 1's 11th post should be blocked
    assert!(
        check_post_rate_limit(100).is_err(),
        "User 1 should be rate limited"
    );

    // User 2 should still be able to post
    assert!(
        check_post_rate_limit(200).is_ok(),
        "User 2 should not be affected by User 1's rate limit"
    );
}

#[test]
fn test_rate_limiter_tracks_keys_correctly() {
    let limiter = RateLimiter::new();

    // Create entries for multiple users/actions
    limiter
        .check_rate_limit("test1", "user1", 10, Duration::from_secs(60))
        .unwrap();
    limiter
        .check_rate_limit("test1", "user2", 10, Duration::from_secs(60))
        .unwrap();
    limiter
        .check_rate_limit("test2", "user1", 10, Duration::from_secs(60))
        .unwrap();

    // Should have 3 different keys
    assert_eq!(
        limiter.tracked_keys_count(),
        3,
        "Should track 3 different action:identifier combinations"
    );
}

#[test]
fn test_cleanup_removes_empty_entries() {
    let limiter = RateLimiter::new();

    // Create some entries
    limiter
        .check_rate_limit("cleanup_test", "user1", 10, Duration::from_secs(1))
        .unwrap();

    assert_eq!(limiter.tracked_keys_count(), 1, "Should have 1 entry");

    // After cleanup, entry should remain (has recent request)
    limiter.cleanup_old_entries();
    assert_eq!(
        limiter.tracked_keys_count(),
        1,
        "Entry should remain after cleanup"
    );
}
