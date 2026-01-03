# Performance Optimizations

This document describes the performance optimizations implemented for the Dumpster forum, focusing on thread page load times.

## Overview

Thread page loading was optimized from ~43ms to ~32ms (26% improvement) through a combination of caching, query consolidation, and data structure optimization.

## Optimization Summary

| Phase | Total Time | Improvement | Technique |
|-------|------------|-------------|-----------|
| Baseline | 43ms | - | Initial state |
| Auth Cache | 40ms | 3ms | Moka in-memory cache |
| Combined Queries | 37ms | 3ms | Thread + forum in single query |
| Conditional Poll | 35ms | 2ms | Skip poll query when none exists |
| Single Posts Query | 32ms | 3ms | Posts + authors in one query |

## Detailed Optimizations

### 1. Authentication Cache (Moka)

**File:** `src/cache.rs`

Session-based user profile caching eliminates redundant database lookups for authenticated users.

```rust
/// Cache for authenticated user profiles
static AUTH_CACHE: Lazy<Cache<Uuid, CachedProfile>> = Lazy::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(30))  // 30s TTL
        .max_capacity(10_000)
        .build()
});

/// Negative cache for invalid sessions
static INVALID_SESSION_CACHE: Lazy<Cache<Uuid, ()>> = Lazy::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(5))   // 5s negative TTL
        .max_capacity(1_000)
        .build()
});
```

**Impact:** Auth lookup reduced from ~9ms to <1ms on cache hit.

**Invalidation Points:**
- `src/web/logout.rs` - On user logout
- `src/web/account.rs` - On profile update (if implemented)

### 2. Combined Thread + Forum Query

**File:** `src/web/thread.rs`

Replaced two separate queries (thread, forum) with a single JOIN query that also checks for poll existence.

```rust
pub struct ThreadWithForum {
    // Thread fields
    pub thread_id: i32,
    pub forum_id: i32,
    pub title: String,
    // ... other thread fields

    // Forum fields (from JOIN)
    pub forum_label: String,
    pub forum_parent_id: Option<i32>,
    // ... other forum fields

    // Poll existence check (subquery)
    pub has_poll: bool,
}
```

**SQL Pattern:**
```sql
SELECT t.*, f.label as forum_label, f.parent_id as forum_parent_id, ...,
       EXISTS(SELECT 1 FROM polls WHERE thread_id = t.id) as has_poll
FROM threads t
JOIN forums f ON f.id = t.forum_id
WHERE t.id = $1
```

**Impact:** 2 queries reduced to 1 (~5ms saved).

### 3. Conditional Poll Query

**File:** `src/web/thread.rs`

The poll query is now skipped entirely when `has_poll = false` (determined in the combined thread query).

```rust
let poll = if has_poll {
    get_poll_for_thread(thread_id, client.get_id()).await?
} else {
    None  // Skip query entirely
};
```

**Impact:** ~5ms saved on threads without polls.

### 4. Single Posts + Authors Query

**File:** `src/web/post.rs`

Replaced the two-query pattern (posts, then users) with a single JOIN query.

#### Before (2 queries):
```rust
// Query 1: Load posts
let posts = posts::Entity::find()
    .filter(...)
    .all(db).await?;

// Query 2: Load users for those posts
let users = UserProfile::get_by_ids(db, &user_ids).await?;
```

#### After (1 query):
```rust
let sql = r#"
    SELECT
        p.id, p.thread_id, p.ugc_id, p.user_id, p.position, p.created_at,
        ugc_rev.content, ugc_del.deleted_at,
        un.name as author_name,
        u.post_count as author_post_count,
        u.reputation_score as author_reputation_score,
        a.filename as author_avatar_filename,
        ...
    FROM posts p
    LEFT JOIN ugc_revisions ugc_rev ON ugc_rev.ugc_id = p.ugc_id
    LEFT JOIN ugc_deletions ugc_del ON ugc_del.id = p.ugc_id
    LEFT JOIN users u ON u.id = p.user_id
    LEFT JOIN user_names un ON un.user_id = u.id
    LEFT JOIN user_avatars ua ON ua.user_id = u.id
    LEFT JOIN attachments a ON a.id = ua.attachment_id
    WHERE p.thread_id = $1 AND p.position BETWEEN $2 AND $3
    ORDER BY p.position, p.created_at
"#;
```

**Impact:** 2 queries (16-19ms) reduced to 1 query (13-15ms).

### 5. Lightweight User Profile (UserProfileLite)

**File:** `src/user.rs`

Created a minimal user struct for post author display, reducing data transfer and deserialization overhead.

#### Full Profile (22 columns):
```rust
pub struct Profile {
    pub id: i32,
    pub name: String,
    pub password_cipher: String,      // Not needed for display
    pub posts_per_page: i32,          // Not needed for display
    pub theme: Option<String>,        // Not needed for display
    pub bio: Option<String>,          // Not needed for display
    // ... 16 more columns
}
```

#### Lite Profile (10 columns):
```rust
pub struct UserProfileLite {
    pub id: i32,
    pub name: String,
    pub created_at: chrono::NaiveDateTime,
    pub avatar_filename: Option<String>,
    pub avatar_height: Option<i32>,
    pub avatar_width: Option<i32>,
    pub post_count: i32,
    pub custom_title: Option<String>,
    pub reputation_score: i32,
    pub signature: Option<String>,
}
```

**Impact:** 55% reduction in columns, faster deserialization.

## Database Indexes

**Migration:** `migrations/20260103185237_performance_indexes.up.sql`

```sql
-- Posts by thread and position (thread page queries)
CREATE INDEX IF NOT EXISTS idx_posts_thread_position
    ON posts(thread_id, position);

-- Unread conversation count
CREATE INDEX IF NOT EXISTS idx_conv_participants_user_archived
    ON conversation_participants(user_id, is_archived);

-- Conversation ordering
CREATE INDEX IF NOT EXISTS idx_conversations_updated
    ON conversations(updated_at DESC);
```

## Breadcrumb Optimization

**File:** `src/web/forum.rs`

Replaced N+1 parent forum lookups with a recursive CTE.

```sql
WITH RECURSIVE ancestors AS (
    SELECT id, parent_id, label, 0 as depth
    FROM forums WHERE id = $1
    UNION ALL
    SELECT f.id, f.parent_id, f.label, a.depth + 1
    FROM forums f
    JOIN ancestors a ON f.id = a.parent_id
)
SELECT id, label, depth FROM ancestors ORDER BY depth DESC
```

**Impact:** N queries reduced to 1 for deep forum hierarchies.

## Unread Count Optimization

**File:** `src/conversations/mod.rs`

Replaced loading all conversation participants with a COUNT query.

```sql
SELECT COUNT(*) FROM conversation_participants cp
JOIN conversations c ON c.id = cp.conversation_id
WHERE cp.user_id = $1
  AND cp.is_archived = false
  AND (cp.last_read_at IS NULL OR c.updated_at > cp.last_read_at)
```

Combined with moka caching (30s TTL) for frequent access.

## Timing Display

The page footer shows timing breakdown:

```
total: 32ms | mw: 2ms (auth:6μs grp:2ms unread:0μs) | handler: 30ms
```

- **total**: Full request time
- **mw**: Middleware time (auth lookup, group check, unread count)
- **handler**: Route handler time

Handler logs show detailed breakdown:

```
Thread 4 handler: total=32870μs thread=9207μs posts=14889μs
                  attach=4448μs bread=13μs poll=1μs tags=4209μs
```

## Performance Monitoring

### Enabling SQL Logging

Set environment variable for query-level timing:

```bash
RUST_LOG=sqlx::query=info cargo run --bin dumpster
```

### Log Output Examples

```
[INFO dumpster::web::post] Posts query: single_query=14848μs (7 posts with authors)
[INFO dumpster::web::thread] Thread 4 handler: total=32870μs thread=9207μs posts=14889μs...
```

## Future Optimization Opportunities

### Low-Hanging Fruit
1. **Attachments query**: Could be combined into posts query
2. **Tags query**: Could be combined into thread query
3. **Similar threads**: Could be cached or made async

### Larger Efforts
1. **Replace SeaORM with raw sqlx**: Each SeaORM query adds ~8-9ms overhead
2. **Server-side page caching**: Cache rendered HTML for hot threads
3. **Edge caching**: CDN caching for anonymous users

## Architecture Notes

### Why SeaORM Overhead Matters

Each SeaORM query involves:
1. Connection pool checkout (~1-2ms)
2. Statement preparation (~1-2ms)
3. Query execution (<1ms for simple queries)
4. Result deserialization (~1-2ms)
5. Connection pool return (~1ms)

Total overhead: ~8-10ms per query, regardless of query complexity.

### Optimization Strategy

Given this overhead, the strategy is:
1. **Reduce query count** over reducing query complexity
2. **Use raw SQL** for critical paths where JOINs are beneficial
3. **Cache aggressively** for frequently-accessed data

### Cache Hierarchy

```
Request → Auth Cache (moka, 30s) → Session DB → Full Profile Query
        → Unread Cache (moka, 30s) → Count Query
        → Permission Cache (startup) → In-memory HashMap
```
