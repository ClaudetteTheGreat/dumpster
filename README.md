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
  - Session invalidation on password reset
  - All active sessions terminated when password is reset for security

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
- **44 integration tests** covering:
  - 6 account lockout tests
  - 7 input validation tests
  - 5 two-factor authentication tests
  - 3 CSRF protection tests
  - 7 rate limiting tests
  - 9 notification tests
  - 7 notification preferences tests
- **Test infrastructure** - Comprehensive test utilities and fixtures
- **Test database** - Isolated test database with full migration support

### Additional Security
- **Permission system** - Bitflag-based permissions with group hierarchy
- **Authorization helpers** - `require_login()`, `require_permission()`, `can_modify()`, `require_ownership()`
- **Soft deletion** - Content is marked deleted, not removed (UGC system)
- **SQL injection prevention** - Using SeaORM with parameterized queries
- **XSS prevention** - Template auto-escaping via Askama
- **IP address tracking** - IP tracking for all posts and threads for moderation purposes
- **Post size limits** - 50,000 character limit for posts (100,000 for moderators) to prevent abuse

## User Interface & Experience

### Navigation & Discoverability
- **Breadcrumb Navigation** - Hierarchical navigation (Home → Forums → Forum → Thread)
- **Latest Post Navigation** - Quick jump to most recent post from thread header and forum listings
- **Enhanced Pagination** - Previous/Next buttons, current page highlighting, smart ellipsis (1 2 3 ... 8 [9] 10 ... 15)
- **Jump to Post** - Direct linking to specific posts with `/threads/{id}/post-{post_id}`

### Forum Features
- **Forum Statistics** - Thread and post counts displayed on forum index
- **Forum Rules Display** - Optional forum-specific rules displayed at the top of each forum in a highlighted box
- **Thread Status Badges** - Visual indicators for pinned (📌) and locked (🔒) threads
- **Thread Metadata** - Post count and view count displayed in thread headers
- **Latest Activity** - Timestamp and link to latest post in forum thread listings

### Moderation Tools
- **Thread Moderation UI** - Lock/Unlock and Pin/Unpin controls for moderators
- **Permission-Based Display** - Moderation tools only visible to users with appropriate permissions
- **Moderation Logging** - All moderation actions logged with reason in `mod_log` table
- **CSRF-Protected Actions** - All moderation operations protected against CSRF attacks

### User Information Display
- **Thread Starter Badge** - "OP" badge displayed next to original poster's name
- **User Post Counts** - Total post count shown in message sidebar
- **Join Date Display** - User registration date shown as "Joined: Mon YYYY"
- **User Avatars** - Avatar display with multiple size options (S/M/L)

### Thread Features
- **Thread Prefixes** - Categorize threads with prefixes like [SOLVED], [QUESTION], [DISCUSSION] displayed as badges
- **Watch Threads** - Subscribe to threads for notifications on new posts
- **Deleted Post Handling** - Placeholder display for deleted posts with deletion timestamp
- **Post History** - Track post edits with revision history
- **Attachments** - File upload support with S3 storage integration

### Responsive Design
- All UI components are mobile-friendly with appropriate breakpoints
- Statistics and metadata hidden on mobile for cleaner layout
- Touch-friendly button sizes and spacing

### User Preferences & Customization
- **Dark Mode** - Toggle between light, dark, and auto (system preference) themes
  - Persistent theme preference stored per user
  - Real-time theme switching without page reload
  - Comprehensive dark mode styling for all UI components
  - Auto mode respects operating system dark mode preference
- **Posts Per Page** - Configurable pagination (10, 25, 50, or 100 posts per page)

## Communication & Notifications

### Notification System
- **In-App Notifications** - Real-time notifications for user mentions, thread replies, and watched threads
- **Notification Types** - Mention, Reply, Thread Watch, Private Message, Quote, Moderation Action
- **Notification Preferences** - Per-type configuration for in-app and email delivery
- **Read/Unread Tracking** - Mark individual or all notifications as read
- **Notification Center** - Centralized view of all user notifications

### Private Messaging
- **Direct Messages** - Send private messages between users
- **Conversation Threads** - Organized message threads with participants
- **Read Status** - Track read/unread status for messages
- **Participant Management** - Multi-user conversations support

### Thread Watching
- **Subscribe to Threads** - Get notified when someone replies to watched threads
- **Notification on Reply** - Configurable notifications for thread activity
- **Manage Subscriptions** - View and manage all watched threads

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
