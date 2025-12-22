//! Application-wide constants
//!
//! This module contains constants used throughout the application.

/// Maximum length for post content in characters
/// Regular users are limited to this length to prevent abuse and
/// excessive database/storage usage.
pub const MAX_POST_LENGTH: usize = 50_000;

/// Maximum length for moderator posts in characters
/// Moderators can create longer posts for announcements and documentation.
pub const MAX_POST_LENGTH_MODERATOR: usize = 100_000;

/// Whether users can view their own deleted posts
/// When false, deleted posts are hidden from everyone including the author
/// When true, authors can still see their own deleted posts
/// This affects the can_read_post permission check
pub const ALLOW_VIEW_OWN_DELETED: bool = false;

/// Default username displayed for unauthenticated users
/// This string will be replaced with localized versions when i18n is implemented
pub const GUEST_USERNAME: &str = "Guest";
