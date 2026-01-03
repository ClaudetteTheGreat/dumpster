//! In-memory caching for frequently accessed data.
//! Uses moka for TTL-based caching with LRU eviction.

use moka::sync::Cache;
use once_cell::sync::Lazy;
use std::time::Duration;
use uuid::Uuid;

/// Cached unread counts for a user
#[derive(Clone, Debug)]
pub struct UnreadCounts {
    pub notifications: i64,
    pub messages: i64,
}

/// Cached authentication context for a session.
/// Contains minimal fields needed for request handling without hitting DB.
#[derive(Clone, Debug)]
pub struct CachedProfile {
    pub id: i32,
    pub name: String,
    pub created_at: chrono::NaiveDateTime,
    pub password_cipher: String,
    pub avatar_filename: Option<String>,
    pub avatar_height: Option<i32>,
    pub avatar_width: Option<i32>,
    pub posts_per_page: i32,
    pub post_count: i32,
    pub theme: Option<String>,
    pub theme_auto: bool,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub website_url: Option<String>,
    pub signature: Option<String>,
    pub custom_title: Option<String>,
    pub show_online: bool,
    pub reputation_score: i32,
    pub allow_profile_posts: bool,
    pub follower_count: i32,
    pub following_count: i32,
    pub default_chat_room: Option<i32>,
}

/// Cache for unread counts with 30 second TTL.
/// Key is user_id, value is UnreadCounts.
static UNREAD_COUNTS_CACHE: Lazy<Cache<i32, UnreadCounts>> = Lazy::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(30))
        .max_capacity(10_000)
        .build()
});

/// Cache for authenticated user profiles with 30 second TTL.
/// Key is session UUID, value is CachedProfile.
/// This eliminates the 9ms Profile::get_by_id() query on every request.
static AUTH_CACHE: Lazy<Cache<Uuid, CachedProfile>> = Lazy::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(30))
        .max_capacity(10_000)
        .build()
});

/// Negative cache for invalid/expired sessions to prevent DB hammering.
/// Key is session UUID that was invalid.
static INVALID_SESSION_CACHE: Lazy<Cache<Uuid, ()>> = Lazy::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(5))
        .max_capacity(1_000)
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

// =============================================================================
// Auth Cache Functions
// =============================================================================

/// Get cached profile for a session, with cache hit status.
/// Returns (Option<CachedProfile>, was_cache_hit, is_negative_cached).
pub fn get_cached_profile(session_uuid: &Uuid) -> (Option<CachedProfile>, bool, bool) {
    // Check negative cache first (invalid sessions)
    if INVALID_SESSION_CACHE.get(session_uuid).is_some() {
        return (None, false, true);
    }

    // Check auth cache
    if let Some(cached) = AUTH_CACHE.get(session_uuid) {
        return (Some(cached), true, false);
    }

    (None, false, false)
}

/// Store a profile in the auth cache.
pub fn cache_profile(session_uuid: Uuid, profile: CachedProfile) {
    AUTH_CACHE.insert(session_uuid, profile);
}

/// Mark a session as invalid (negative caching).
pub fn cache_invalid_session(session_uuid: Uuid) {
    INVALID_SESSION_CACHE.insert(session_uuid, ());
}

/// Invalidate auth cache for a specific session.
/// Call this on logout or when session is removed.
pub fn invalidate_auth_cache(session_uuid: &Uuid) {
    AUTH_CACHE.invalidate(session_uuid);
    INVALID_SESSION_CACHE.invalidate(session_uuid);
}

/// Invalidate auth cache for all sessions of a user.
/// Call this when user profile is updated (name, avatar, theme, etc.).
pub fn invalidate_auth_cache_for_user(user_id: i32) {
    // We can't efficiently iterate the cache by user_id, so we just
    // let TTL expire the stale entries. This is acceptable because:
    // 1. TTL is short (30s)
    // 2. Profile updates are infrequent
    // 3. Stale data is only cosmetic (name, avatar, theme)
    //
    // For critical changes (password, permissions), use invalidate_user_sessions()
    // which removes the session entirely.
    log::debug!("Auth cache invalidation requested for user_id={}, TTL will expire entries", user_id);
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
