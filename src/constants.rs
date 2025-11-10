/// Application-wide constants
///
/// This module contains constants used throughout the application.

/// Maximum length for post content in characters
/// Regular users are limited to this length to prevent abuse and
/// excessive database/storage usage.
pub const MAX_POST_LENGTH: usize = 50_000;

/// Maximum length for moderator posts in characters
/// Moderators can create longer posts for announcements and documentation.
pub const MAX_POST_LENGTH_MODERATOR: usize = 100_000;
