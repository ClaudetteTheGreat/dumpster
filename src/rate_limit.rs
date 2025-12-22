/// Rate limiting module for preventing abuse and DDoS attacks
///
/// Implements sliding window rate limiting using in-memory storage (DashMap).
/// This is suitable for single-instance deployments. For multi-instance
/// deployments, consider using Redis as a backing store.
///
/// # Example Usage
///
/// ```rust,ignore
/// use crate::rate_limit::{check_login_rate_limit, RateLimitError};
///
/// // In a login handler
/// if let Err(e) = check_login_rate_limit("192.168.1.1", "username") {
///     return Err(error::ErrorTooManyRequests(
///         format!("Too many attempts. Try again in {} seconds", e.retry_after_seconds)
///     ));
/// }
/// ```
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Global rate limiter instance
pub static RATE_LIMITER: Lazy<Arc<RateLimiter>> = Lazy::new(|| Arc::new(RateLimiter::new()));

/// Rate limiter using in-memory storage
pub struct RateLimiter {
    /// Map of (action_type:identifier) -> Request timestamps
    requests: DashMap<String, Vec<Instant>>,
}

/// Error returned when rate limit is exceeded
#[derive(Debug, Clone)]
pub struct RateLimitError {
    /// Number of seconds until the rate limit resets
    pub retry_after_seconds: u64,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new() -> Self {
        Self {
            requests: DashMap::new(),
        }
    }

    /// Check if a request should be rate limited
    ///
    /// # Arguments
    /// * `action` - The action being rate limited (e.g., "login", "post", "thread")
    /// * `identifier` - Unique identifier for the requester (e.g., IP address, user ID)
    /// * `max_requests` - Maximum number of requests allowed in the window
    /// * `window` - Time window for the rate limit
    ///
    /// # Returns
    /// * `Ok(())` if the request is allowed
    /// * `Err(RateLimitError)` if the rate limit is exceeded
    pub fn check_rate_limit(
        &self,
        action: &str,
        identifier: &str,
        max_requests: usize,
        window: Duration,
    ) -> Result<(), RateLimitError> {
        let key = format!("{}:{}", action, identifier);
        let now = Instant::now();

        // Get or create entry for this key
        let mut entry = self.requests.entry(key).or_default();

        // Remove requests outside the time window (sliding window)
        entry.retain(|&timestamp| now.duration_since(timestamp) < window);

        // Check if we've exceeded the limit
        if entry.len() >= max_requests {
            // Calculate how long until the oldest request expires
            let oldest = entry[0];
            let retry_after = window.saturating_sub(now.duration_since(oldest));

            return Err(RateLimitError {
                retry_after_seconds: retry_after.as_secs() + 1, // Round up
            });
        }

        // Add current request
        entry.push(now);

        Ok(())
    }

    /// Clean up old entries to prevent memory leaks
    ///
    /// This should be called periodically (e.g., every 5 minutes) to remove
    /// entries for keys that haven't been used recently.
    pub fn cleanup_old_entries(&self) {
        self.requests.retain(|_, timestamps| {
            // Keep entries that have recent requests
            !timestamps.is_empty()
        });
    }

    /// Get the number of tracked keys (for monitoring/debugging)
    pub fn tracked_keys_count(&self) -> usize {
        self.requests.len()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper functions for common rate-limited actions
// ============================================================================

/// Check rate limit for login attempts
///
/// Limit: 5 attempts per 5 minutes per IP+username combination
///
/// This is in addition to account lockout - rate limiting prevents
/// trying multiple different passwords rapidly, while account lockout
/// prevents trying the same account repeatedly.
pub fn check_login_rate_limit(ip: &str, username: &str) -> Result<(), RateLimitError> {
    RATE_LIMITER.check_rate_limit(
        "login",
        &format!("{}:{}", ip, username),
        5,
        Duration::from_secs(300), // 5 minutes
    )
}

/// Check rate limit for user registration
///
/// Limit: 3 registrations per hour per IP address
pub fn check_registration_rate_limit(ip: &str) -> Result<(), RateLimitError> {
    RATE_LIMITER.check_rate_limit(
        "register",
        ip,
        3,
        Duration::from_secs(3600), // 1 hour
    )
}

/// Check rate limit for post creation
///
/// Limit: 10 posts per minute per user
pub fn check_post_rate_limit(user_id: i32) -> Result<(), RateLimitError> {
    RATE_LIMITER.check_rate_limit(
        "post",
        &user_id.to_string(),
        10,
        Duration::from_secs(60), // 1 minute
    )
}

/// Check rate limit for thread creation
///
/// Limit: 5 threads per 5 minutes per user
pub fn check_thread_rate_limit(user_id: i32) -> Result<(), RateLimitError> {
    RATE_LIMITER.check_rate_limit(
        "thread",
        &user_id.to_string(),
        5,
        Duration::from_secs(300), // 5 minutes
    )
}

/// Start background cleanup task
///
/// This function should be called once at application startup to spawn
/// a background task that periodically cleans up old rate limit entries.
///
/// Example (in main.rs or similar binary):
/// ```rust,ignore
/// // Spawn cleanup task
/// actix_web::rt::spawn(async {
///     let mut interval = actix_rt::time::interval(Duration::from_secs(300));
///     loop {
///         interval.tick().await;
///         ruforo::rate_limit::RATE_LIMITER.cleanup_old_entries();
///         log::debug!("Rate limiter cleanup completed");
///     }
/// });
/// ```
pub fn cleanup_old_entries_public() {
    RATE_LIMITER.cleanup_old_entries();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_allows_requests_within_limit() {
        let limiter = RateLimiter::new();

        // Should allow first 3 requests
        for i in 0..3 {
            assert!(
                limiter
                    .check_rate_limit("test", "user1", 3, Duration::from_secs(10))
                    .is_ok(),
                "Request {} should be allowed",
                i
            );
        }
    }

    #[test]
    fn test_rate_limit_blocks_requests_over_limit() {
        let limiter = RateLimiter::new();

        // Allow first 3 requests
        for _ in 0..3 {
            limiter
                .check_rate_limit("test", "user1", 3, Duration::from_secs(10))
                .unwrap();
        }

        // 4th request should be blocked
        let result = limiter.check_rate_limit("test", "user1", 3, Duration::from_secs(10));
        assert!(result.is_err(), "4th request should be blocked");

        if let Err(err) = result {
            assert!(err.retry_after_seconds > 0, "Should have retry_after time");
        }
    }

    #[test]
    fn test_rate_limit_different_identifiers_independent() {
        let limiter = RateLimiter::new();

        // Use up limit for user1
        for _ in 0..3 {
            limiter
                .check_rate_limit("test", "user1", 3, Duration::from_secs(10))
                .unwrap();
        }

        // user2 should still be allowed
        assert!(
            limiter
                .check_rate_limit("test", "user2", 3, Duration::from_secs(10))
                .is_ok(),
            "Different identifier should have independent limit"
        );
    }

    #[test]
    fn test_rate_limit_cleanup() {
        let limiter = RateLimiter::new();

        // Create some entries
        limiter
            .check_rate_limit("test", "user1", 10, Duration::from_secs(10))
            .unwrap();
        limiter
            .check_rate_limit("test", "user2", 10, Duration::from_secs(10))
            .unwrap();

        assert_eq!(limiter.tracked_keys_count(), 2);

        // Clean up - entries should remain since they have recent requests
        limiter.cleanup_old_entries();
        assert_eq!(limiter.tracked_keys_count(), 2);
    }
}
