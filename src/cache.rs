//! In-memory caching for frequently accessed data.
//! Uses moka for TTL-based caching with LRU eviction.

use moka::sync::Cache;
use once_cell::sync::Lazy;
use std::time::Duration;

/// Cached unread counts for a user
#[derive(Clone, Debug)]
pub struct UnreadCounts {
    pub notifications: i64,
    pub messages: i64,
}

/// Cache for unread counts with 30 second TTL.
/// Key is user_id, value is UnreadCounts.
static UNREAD_COUNTS_CACHE: Lazy<Cache<i32, UnreadCounts>> = Lazy::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(30))
        .max_capacity(10_000)
        .build()
});

/// Get unread counts for a user, using cache if available.
/// Falls back to database query if cache miss.
pub async fn get_unread_counts(user_id: i32) -> UnreadCounts {
    get_unread_counts_with_status(user_id).await.0
}

/// Get unread counts for a user with cache hit status.
/// Returns (UnreadCounts, was_cache_hit).
pub async fn get_unread_counts_with_status(user_id: i32) -> (UnreadCounts, bool) {
    // Check cache first
    if let Some(cached) = UNREAD_COUNTS_CACHE.get(&user_id) {
        return (cached, true);
    }

    // Cache miss - query database
    let notifications = crate::notifications::count_unread_notifications(user_id)
        .await
        .unwrap_or(0);
    let messages = crate::conversations::count_unread_conversations(user_id)
        .await
        .unwrap_or(0);

    let counts = UnreadCounts {
        notifications,
        messages,
    };

    // Store in cache
    UNREAD_COUNTS_CACHE.insert(user_id, counts.clone());

    (counts, false)
}

/// Invalidate unread counts cache for a user.
/// Call this when creating new notifications or messages.
pub fn invalidate_unread_counts(user_id: i32) {
    UNREAD_COUNTS_CACHE.invalidate(&user_id);
}

/// Invalidate only notification count (triggers full reload on next request).
/// Convenience alias for invalidate_unread_counts.
pub fn invalidate_notification_count(user_id: i32) {
    invalidate_unread_counts(user_id);
}

/// Invalidate only message count (triggers full reload on next request).
/// Convenience alias for invalidate_unread_counts.
pub fn invalidate_message_count(user_id: i32) {
    invalidate_unread_counts(user_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let counts = UnreadCounts {
            notifications: 5,
            messages: 3,
        };
        UNREAD_COUNTS_CACHE.insert(999, counts.clone());

        let cached = UNREAD_COUNTS_CACHE.get(&999);
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.notifications, 5);
        assert_eq!(cached.messages, 3);

        // Clean up
        UNREAD_COUNTS_CACHE.invalidate(&999);
    }

    #[test]
    fn test_cache_invalidation() {
        let counts = UnreadCounts {
            notifications: 10,
            messages: 20,
        };
        UNREAD_COUNTS_CACHE.insert(998, counts);

        // Verify it's there
        assert!(UNREAD_COUNTS_CACHE.get(&998).is_some());

        // Invalidate
        invalidate_unread_counts(998);

        // Should be gone
        assert!(UNREAD_COUNTS_CACHE.get(&998).is_none());
    }
}
