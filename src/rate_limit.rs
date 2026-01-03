/// Rate limiting module for preventing abuse and DDoS attacks
///
/// Implements sliding window rate limiting using in-memory storage (DashMap).
/// This is suitable for single-instance deployments. For multi-instance
/// deployments, consider using Redis as a backing store.
///
/// Rate limits are configurable via database settings and support hot reload.
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
use arc_swap::ArcSwap;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::Config;

/// Global rate limiter instance
pub static RATE_LIMITER: Lazy<Arc<RateLimiter>> = Lazy::new(|| Arc::new(RateLimiter::new()));

/// Global rate limit configuration (hot-reloadable)
static RATE_LIMIT_CONFIG: Lazy<ArcSwap<RateLimitConfig>> =
    Lazy::new(|| ArcSwap::from_pointee(RateLimitConfig::default()));

/// Rate limit configuration loaded from database settings
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    // Authentication (individual - critical)
    pub login_max: usize,
    pub login_window: Duration,
    pub two_factor_max: usize,
    pub two_factor_window: Duration,
    pub password_reset_max: usize,
    pub password_reset_window: Duration,
    pub email_verification_max: usize,
    pub email_verification_window: Duration,

    // Account creation
    pub registration_max: usize,
    pub registration_window: Duration,

    // Content creation (grouped)
    pub post_creation_max: usize,
    pub post_creation_window: Duration,
    pub thread_creation_max: usize,
    pub thread_creation_window: Duration,

    // Search & API (grouped)
    pub search_max: usize,
    pub search_window: Duration,
    pub api_max: usize,
    pub api_window: Duration,

    // File uploads
    pub file_upload_max: usize,
    pub file_upload_window: Duration,

    // Reports
    pub report_max: usize,
    pub report_window: Duration,

    // Reactions
    pub reaction_max: usize,
    pub reaction_window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            // Authentication (individual - critical)
            login_max: 5,
            login_window: Duration::from_secs(300), // 5 minutes
            two_factor_max: 5,
            two_factor_window: Duration::from_secs(300), // 5 minutes
            password_reset_max: 3,
            password_reset_window: Duration::from_secs(3600), // 1 hour
            email_verification_max: 3,
            email_verification_window: Duration::from_secs(3600), // 1 hour

            // Account creation
            registration_max: 3,
            registration_window: Duration::from_secs(3600), // 1 hour

            // Content creation (grouped)
            post_creation_max: 10,
            post_creation_window: Duration::from_secs(60), // 1 minute
            thread_creation_max: 5,
            thread_creation_window: Duration::from_secs(300), // 5 minutes

            // Search & API (grouped)
            search_max: 30,
            search_window: Duration::from_secs(60), // 1 minute
            api_max: 60,
            api_window: Duration::from_secs(60), // 1 minute

            // File uploads
            file_upload_max: 20,
            file_upload_window: Duration::from_secs(60), // 1 minute

            // Reports
            report_max: 5,
            report_window: Duration::from_secs(300), // 5 minutes

            // Reactions
            reaction_max: 30,
            reaction_window: Duration::from_secs(60), // 1 minute
        }
    }
}

impl RateLimitConfig {
    /// Load rate limit configuration from the Config settings
    pub fn from_config(config: &Config) -> Self {
        Self {
            // Authentication
            login_max: config.get_int_or("rate_limit.login.max_requests", 5) as usize,
            login_window: Duration::from_secs(
                config.get_int_or("rate_limit.login.window_seconds", 300) as u64,
            ),
            two_factor_max: config.get_int_or("rate_limit.two_factor.max_requests", 5) as usize,
            two_factor_window: Duration::from_secs(
                config.get_int_or("rate_limit.two_factor.window_seconds", 300) as u64,
            ),
            password_reset_max: config.get_int_or("rate_limit.password_reset.max_requests", 3)
                as usize,
            password_reset_window: Duration::from_secs(
                config.get_int_or("rate_limit.password_reset.window_seconds", 3600) as u64,
            ),
            email_verification_max: config
                .get_int_or("rate_limit.email_verification.max_requests", 3)
                as usize,
            email_verification_window: Duration::from_secs(
                config.get_int_or("rate_limit.email_verification.window_seconds", 3600) as u64,
            ),

            // Account creation
            registration_max: config.get_int_or("rate_limit.registration.max_requests", 3)
                as usize,
            registration_window: Duration::from_secs(
                config.get_int_or("rate_limit.registration.window_seconds", 3600) as u64,
            ),

            // Content creation
            post_creation_max: config.get_int_or("rate_limit.post_creation.max_requests", 10)
                as usize,
            post_creation_window: Duration::from_secs(
                config.get_int_or("rate_limit.post_creation.window_seconds", 60) as u64,
            ),
            thread_creation_max: config.get_int_or("rate_limit.thread_creation.max_requests", 5)
                as usize,
            thread_creation_window: Duration::from_secs(
                config.get_int_or("rate_limit.thread_creation.window_seconds", 300) as u64,
            ),

            // Search & API
            search_max: config.get_int_or("rate_limit.search.max_requests", 30) as usize,
            search_window: Duration::from_secs(
                config.get_int_or("rate_limit.search.window_seconds", 60) as u64,
            ),
            api_max: config.get_int_or("rate_limit.api.max_requests", 60) as usize,
            api_window: Duration::from_secs(
                config.get_int_or("rate_limit.api.window_seconds", 60) as u64,
            ),

            // File uploads
            file_upload_max: config.get_int_or("rate_limit.file_upload.max_requests", 20) as usize,
            file_upload_window: Duration::from_secs(
                config.get_int_or("rate_limit.file_upload.window_seconds", 60) as u64,
            ),

            // Reports
            report_max: config.get_int_or("rate_limit.report.max_requests", 5) as usize,
            report_window: Duration::from_secs(
                config.get_int_or("rate_limit.report.window_seconds", 300) as u64,
            ),

            // Reactions
            reaction_max: config.get_int_or("rate_limit.reaction.max_requests", 30) as usize,
            reaction_window: Duration::from_secs(
                config.get_int_or("rate_limit.reaction.window_seconds", 60) as u64,
            ),
        }
    }
}

/// Initialize rate limits from config (call at startup after loading settings)
pub fn init_rate_limits(config: &Config) {
    let rate_config = RateLimitConfig::from_config(config);
    RATE_LIMIT_CONFIG.store(Arc::new(rate_config));
    log::info!("Rate limit configuration initialized from database settings");
}

/// Reload rate limits from config (call when rate limit settings are changed)
pub fn reload_rate_limits(config: &Config) {
    let rate_config = RateLimitConfig::from_config(config);
    RATE_LIMIT_CONFIG.store(Arc::new(rate_config));
    log::info!("Rate limit configuration reloaded");
}

/// Get the current rate limit configuration
pub fn get_rate_limit_config() -> Arc<RateLimitConfig> {
    RATE_LIMIT_CONFIG.load_full()
}

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

    /// Get the current request count for a specific action/identifier
    ///
    /// Returns the number of requests within the time window
    pub fn get_request_count(&self, action: &str, identifier: &str, window: Duration) -> u32 {
        let key = format!("{}:{}", action, identifier);
        let now = Instant::now();

        if let Some(entry) = self.requests.get(&key) {
            entry
                .iter()
                .filter(|&&timestamp| now.duration_since(timestamp) < window)
                .count() as u32
        } else {
            0
        }
    }

    /// Clear all requests for a specific action/identifier
    pub fn clear_requests(&self, action: &str, identifier: &str) {
        let key = format!("{}:{}", action, identifier);
        self.requests.remove(&key);
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
/// Uses configurable limit per IP+username combination
pub fn check_login_rate_limit(ip: &str, username: &str) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit(
        "login",
        &format!("{}:{}", ip, username),
        config.login_max,
        config.login_window,
    )
}

/// Check rate limit for two-factor authentication attempts
///
/// Uses configurable limit per IP address
pub fn check_two_factor_rate_limit(ip: &str) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit("two_factor", ip, config.two_factor_max, config.two_factor_window)
}

/// Check rate limit for password reset requests
///
/// Uses configurable limit per IP address
pub fn check_password_reset_rate_limit(ip: &str) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit(
        "password_reset",
        ip,
        config.password_reset_max,
        config.password_reset_window,
    )
}

/// Check rate limit for email verification resend requests
///
/// Uses configurable limit per IP address
pub fn check_email_verification_rate_limit(ip: &str) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit(
        "email_verification",
        ip,
        config.email_verification_max,
        config.email_verification_window,
    )
}

/// Check rate limit for user registration
///
/// Uses configurable limit per IP address
pub fn check_registration_rate_limit(ip: &str) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit(
        "register",
        ip,
        config.registration_max,
        config.registration_window,
    )
}

/// Check rate limit for post creation
///
/// Uses configurable limit per user. Applies to:
/// - Forum posts
/// - Profile posts
/// - Conversation messages
pub fn check_post_rate_limit(user_id: i32) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit(
        "post",
        &user_id.to_string(),
        config.post_creation_max,
        config.post_creation_window,
    )
}

/// Check rate limit for thread creation
///
/// Uses configurable limit per user
pub fn check_thread_rate_limit(user_id: i32) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit(
        "thread",
        &user_id.to_string(),
        config.thread_creation_max,
        config.thread_creation_window,
    )
}

/// Check rate limit for search queries
///
/// Uses configurable limit per IP or user
pub fn check_search_rate_limit(identifier: &str) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit("search", identifier, config.search_max, config.search_window)
}

/// Check rate limit for general API requests
///
/// Applies to: user search, URL unfurl, etc.
pub fn check_api_rate_limit(identifier: &str) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit("api", identifier, config.api_max, config.api_window)
}

/// Check rate limit for file uploads
///
/// Uses configurable limit per user
pub fn check_file_upload_rate_limit(user_id: i32) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit(
        "file_upload",
        &user_id.to_string(),
        config.file_upload_max,
        config.file_upload_window,
    )
}

/// Check rate limit for report submissions
///
/// Uses configurable limit per user
pub fn check_report_rate_limit(user_id: i32) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit(
        "report",
        &user_id.to_string(),
        config.report_max,
        config.report_window,
    )
}

/// Check rate limit for reaction toggles
///
/// Uses configurable limit per user
pub fn check_reaction_rate_limit(user_id: i32) -> Result<(), RateLimitError> {
    let config = get_rate_limit_config();
    RATE_LIMITER.check_rate_limit(
        "reaction",
        &user_id.to_string(),
        config.reaction_max,
        config.reaction_window,
    )
}

/// Record a failed login attempt for an IP address
///
/// This is separate from rate limiting - it tracks failures to determine
/// when to require CAPTCHA verification.
pub fn record_failed_login(ip: &str) {
    // We use check_rate_limit to add the timestamp, allowing up to 100 attempts
    // The actual limiting is done by check_login_rate_limit, this just tracks
    let _ = RATE_LIMITER.check_rate_limit(
        "login_failures",
        ip,
        100,                       // High limit - we're just tracking, not limiting
        Duration::from_secs(3600), // 1 hour window
    );
}

/// Get the number of failed login attempts for an IP address
///
/// Returns the count of failed login attempts within the past hour
pub fn get_failed_login_count(ip: &str) -> u32 {
    RATE_LIMITER.get_request_count("login_failures", ip, Duration::from_secs(3600))
}

/// Clear failed login attempts for an IP address
///
/// Called on successful login to reset the CAPTCHA requirement
pub fn clear_failed_logins(ip: &str) {
    RATE_LIMITER.clear_requests("login_failures", ip);
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
///         dumpster::rate_limit::RATE_LIMITER.cleanup_old_entries();
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

    #[test]
    fn test_default_rate_limit_config() {
        let config = RateLimitConfig::default();

        // Verify defaults match migration values
        assert_eq!(config.login_max, 5);
        assert_eq!(config.login_window, Duration::from_secs(300));
        assert_eq!(config.registration_max, 3);
        assert_eq!(config.post_creation_max, 10);
        assert_eq!(config.thread_creation_max, 5);
        assert_eq!(config.search_max, 30);
        assert_eq!(config.file_upload_max, 20);
    }
}
