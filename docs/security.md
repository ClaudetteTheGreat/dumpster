# Security Features

This document covers the security features implemented in the forum.

## Authentication & Authorization

- **Argon2 Password Hashing** - Industry-standard password encryption with salt
- **Two-Factor Authentication (2FA)** - TOTP-based 2FA with Google Authenticator support
  - Two-step login flow: username/password → TOTP verification
  - Pending auth state management
- **Account Lockout** - 5 failed login attempts = 15 minute lockout
  - Automatic unlock on expiration
  - Reset counter on successful login
- **Input Validation** - Comprehensive form validation using validator crate
  - Username: 1-255 characters, trimmed
  - Password: 1-1000 characters
  - TOTP: exactly 6 digits
- **Session Management** - Secure cookie-based sessions via actix-session
  - Session invalidation on password reset
  - All active sessions terminated when password is reset for security
  - "Remember me" option for 30-day extended sessions
- **Password Reset** - Secure password reset flow
  - Email-based reset with secure 64-character tokens
  - 1-hour token expiration
  - Single-use tokens (cannot be reused)
  - Success message displayed after reset
  - All sessions invalidated for security

## CSRF Protection

- **Session-based CSRF tokens** on all state-changing operations
- **Protected endpoints:**
  - Login and 2FA verification
  - Post creation, editing, deletion
  - Thread creation and replies
  - Account operations (avatar upload)
- **Template integration** - Automatic token generation per session

## Rate Limiting

- **Sliding window rate limiting** using DashMap (in-memory)
- **Login attempts:** 5 per 5 minutes (IP + username)
- **2FA attempts:** 5 per 5 minutes (IP)
- **Post creation:** 10 per minute (user ID)
- **Thread creation:** 5 per 5 minutes (user ID)
- **Registration:** 3 per hour (IP)
- **Background cleanup** - Automatic cleanup every 5 minutes
- **Extension ready** - Clean architecture for Redis backend

## CAPTCHA Protection

- **Dual Provider Support** - hCaptcha and Cloudflare Turnstile
- **Registration CAPTCHA** - Required when enabled via environment variables
- **Login CAPTCHA** - Required after 3+ failed login attempts from same IP
- **Environment Configuration:**
  - `CAPTCHA_PROVIDER`: "hcaptcha" or "turnstile" (disabled if not set)
  - `CAPTCHA_SITE_KEY`: Public key for frontend widgets
  - `CAPTCHA_SECRET_KEY`: Secret key for backend verification
- **Failed Login Tracking** - 1-hour window, cleared on successful login

## Spam Detection

- **Heuristic-based content analysis** with configurable threshold
- **URL Analysis** - Flags excessive links, especially from new users
- **Repeated Characters** - Detects "aaaaaaa" style spam
- **ALL CAPS Detection** - Flags excessive capitalization
- **Spam Phrases** - Checks for common spam phrases ("click here", "buy now", etc.)
- **Emoji Spam** - Flags excessive emoji usage
- **Integrated into** post creation and thread creation

## Word Filters

- **Admin-configurable content filters** for automatic moderation
- **Filter Actions:**
  - **Replace** - Substitute matched words with alternatives (e.g., "Solana" → "Salona")
  - **Block** - Reject content containing specific words entirely
  - **Flag** - Allow content but mark it for moderator review
- **Matching Options:**
  - Regular expression support for complex patterns
  - Case-sensitive or case-insensitive matching
  - Whole-word only or partial matching within words
  - Enable/disable individual filters without deletion
- **Case Preservation** - Replacements preserve original case (WORD→REPLACEMENT, Word→Replacement, word→replacement)
- **Admin Panel** - Full CRUD interface at `/admin/word-filters`
- **Integrated into** thread creation (title and content) and post replies
- **Efficient Caching** - Compiled regex patterns cached in memory, reloaded on filter changes

## Security Headers

- **X-Frame-Options: DENY** - Prevents clickjacking attacks
- **X-Content-Type-Options: nosniff** - Prevents MIME type sniffing
- **Referrer-Policy: strict-origin-when-cross-origin** - Controls referrer info
- **Permissions-Policy** - Restricts geolocation, microphone, camera access

## Additional Security Measures

- **Permission system** - Bitflag-based permissions with group hierarchy
- **Authorization helpers** - `require_login()`, `require_permission()`, `can_modify()`, `require_ownership()`
- **Soft deletion** - Content is marked deleted, not removed (UGC system)
- **SQL injection prevention** - Using SeaORM with parameterized queries
- **XSS prevention** - Template auto-escaping via Askama
- **IP address tracking** - IP tracking for all posts and threads for moderation purposes
- **Post size limits** - 50,000 character limit for posts (100,000 for moderators) to prevent abuse

## Testing

- **255+ tests** covering:
  - 6 account lockout tests
  - 7 input validation tests
  - 5 two-factor authentication tests
  - 3 CSRF protection tests
  - 9 notification tests
  - 7 notification preferences tests
  - 9 email verification tests
  - 10 password reset tests
  - 6 moderation tests (lock/pin/unpin)
  - 8 deletion types tests (normal/permanent/legal hold)
  - 8 IP ban tests
  - 8 search tests
  - 10 conversation/PM tests
  - 9 thread watching tests
  - 6 post reactions tests
  - 7 report system tests
  - 6 user ban tests
  - 5 RSS feed tests
  - 7 thread move/merge tests
  - 10 user profile tests
  - 12 word filter tests
  - 78 unit tests (BBCode, spam detection, rate limiting, etc.)
- **Test infrastructure** - Comprehensive test utilities and fixtures
- **Test database** - Isolated test database with full migration support
