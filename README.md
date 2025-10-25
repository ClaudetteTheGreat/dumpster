# _Unnamed Web Forum Project_
(Formerly ruforo, formerly Sneedforo, formerly Chuckforo, formerly XenForo)

PROJECT_NAME is a traditional web forum built in Rust.

## Stack
 - Rust
   - Actix-Web
   - Askama for templating
   - SeaQL (sqlx) for ORM
 - Postgres
 - S3
 - NPM
   - SWC for asset compilation
   - SCSS for stylesheets
   - Vanilla JS

## Aspirations
 - Minimal bloat.
 - No-JS, Tor compatability.
 - Unit tested.
 - Event driven WebSocket subscriptions.
 - Total replacement for XenForo.

## Security Features

### Authentication & Authorization
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

### CSRF Protection
- **Session-based CSRF tokens** on all state-changing operations
- **Protected endpoints:**
  - Login and 2FA verification
  - Post creation, editing, deletion
  - Thread creation and replies
  - Account operations (avatar upload)
- **Template integration** - Automatic token generation per session

### Rate Limiting
- **Sliding window rate limiting** using DashMap (in-memory)
- **Login attempts:** 5 per 5 minutes (IP + username)
- **2FA attempts:** 5 per 5 minutes (IP)
- **Post creation:** 10 per minute (user ID)
- **Thread creation:** 5 per 5 minutes (user ID)
- **Background cleanup** - Automatic cleanup every 5 minutes
- **Extension ready** - Clean architecture for Redis backend

### Testing
- **28 integration tests** covering:
  - 6 account lockout tests
  - 7 input validation tests
  - 5 two-factor authentication tests
  - 3 CSRF protection tests
  - 7 rate limiting tests
- **Test infrastructure** - Comprehensive test utilities and fixtures

### Additional Security
- **Permission system** - Bitflag-based permissions with group hierarchy
- **Authorization helpers** - `require_login()`, `require_permission()`, `can_modify()`, `require_ownership()`
- **Soft deletion** - Content is marked deleted, not removed (UGC system)
- **SQL injection prevention** - Using SeaORM with parameterized queries
- **XSS prevention** - Template auto-escaping via Askama

## Environment
 - Example `.env` file
   + NOTE: AWS variables will likely be migrated to DB
 - PostgreSQL
   + Required. Database agnosticism not planned.
 - S3 Storage
   + Any S3-compatible storage API for attachments.
   + Suggested to use [MinIO](https://min.io/) (FOSS + Self-Hosted)
 - node and webpack
   + Install [npm](https://nodejs.org/en/download/).
   + Run `npm install` from the root directory to install node dependencies.
   + Run `npx webpack` from the root directory to deploy browser-friendly resource files.
   + _webpack will be replaced with SWC when SASS compilation is available._

### WebM Validation Notes
 - https://www.webmproject.org/docs/container/
 - VP8
 - VP9
 - AV1
 - OPUS
 - VORBIS

## Contributions
### Code Guidelines
 - We use [rustfmt](https://github.com/rust-lang/rustfmt).
 - `cargo clippy` whenever possible.
 - Try to eliminate warnings.

### Database Guidelines
 - Any data which would apply to two types of content (i.e. posts, chat messages, profile posts) should interact with the `ugc` tables, not individual content type tables.
 - Usernames should be referenced by `user_id,created_at DESC` from `user_name`. User rows can be deleted, but a historical reference for their name will be added to this table. This complies with [GDPR software requirements](https://gdpr.eu/right-to-be-forgotten).
