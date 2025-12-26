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
  - "Remember me" option for 30-day extended sessions
- **Password Reset** - Secure password reset flow
  - Email-based reset with secure 64-character tokens
  - 1-hour token expiration
  - Single-use tokens (cannot be reused)
  - Success message displayed after reset
  - All sessions invalidated for security

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
- **Registration:** 3 per hour (IP)
- **Background cleanup** - Automatic cleanup every 5 minutes
- **Extension ready** - Clean architecture for Redis backend

### CAPTCHA Protection
- **Dual Provider Support** - hCaptcha and Cloudflare Turnstile
- **Registration CAPTCHA** - Required when enabled via environment variables
- **Login CAPTCHA** - Required after 3+ failed login attempts from same IP
- **Environment Configuration:**
  - `CAPTCHA_PROVIDER`: "hcaptcha" or "turnstile" (disabled if not set)
  - `CAPTCHA_SITE_KEY`: Public key for frontend widgets
  - `CAPTCHA_SECRET_KEY`: Secret key for backend verification
- **Failed Login Tracking** - 1-hour window, cleared on successful login

### Spam Detection
- **Heuristic-based content analysis** with configurable threshold
- **URL Analysis** - Flags excessive links, especially from new users
- **Repeated Characters** - Detects "aaaaaaa" style spam
- **ALL CAPS Detection** - Flags excessive capitalization
- **Spam Phrases** - Checks for common spam phrases ("click here", "buy now", etc.)
- **Emoji Spam** - Flags excessive emoji usage
- **Integrated into** post creation and thread creation

### Word Filters
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

### Security Headers
- **X-Frame-Options: DENY** - Prevents clickjacking attacks
- **X-Content-Type-Options: nosniff** - Prevents MIME type sniffing
- **Referrer-Policy: strict-origin-when-cross-origin** - Controls referrer info
- **Permissions-Policy** - Restricts geolocation, microphone, camera access

### Testing
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
- **New Posts Feed** - `/recent/posts` shows latest posts across all forums with navigation link in header
- **New Threads Feed** - `/recent/threads` shows latest threads across all forums

### Forum Features
- **Forum Statistics** - Thread and post counts displayed on forum index
- **Forum Rules Display** - Optional forum-specific rules displayed at the top of each forum in a highlighted box
- **Forum Moderators** - Display moderators assigned to each forum with profile links
- **Sub-Forums** - Hierarchical forum structure with parent/child relationships
  - Forums can be nested under parent forums
  - Sub-forums displayed with visual indentation in forum index
  - Breadcrumb navigation includes full parent chain
  - Sub-forums section displayed within parent forum view
- **Read Tracking** - Track read/unread status for forums and threads
  - Unread indicators (📂 icon, blue border) for forums with new posts
  - "Mark as Read" button to mark individual forums as read
  - "Mark All Read" button to mark all forums as read at once
  - Jump to first unread post via `/threads/{id}/unread`
  - "Unread" link in thread listings for quick navigation
- **Thread Status Badges** - Visual indicators for pinned (📌) and locked (🔒) threads
- **Thread Metadata** - Post count and view count displayed in thread headers
- **Latest Activity** - Timestamp and link to latest post in forum thread listings

### Moderation Tools
- **Thread Moderation UI** - Lock/Unlock and Pin/Unpin controls for moderators
- **Thread Move** - Move threads between forums
  - Forum selection dropdown with all available destinations
  - Optional reason field for moderation logs
  - Metadata logged includes source and destination forum IDs
- **Thread Merge** - Combine threads together
  - Moves all posts from source thread to target thread
  - Recalculates post counts and first/last post references
  - Source thread marked as merged with link to target
  - Merged threads hidden from forum listings
- **Mass Moderation Actions** - Bulk operations on users
  - Checkbox selection with "Select All" in user management
  - Actions: Ban, Unban, Verify Email, Approve, Delete
  - Configurable ban duration for mass bans
  - Confirmation dialogs before executing
- **User Warning System** - Progressive discipline with point-based warnings
  - Configurable warning points (1-10 per warning)
  - Warning expiration options (30-365 days or permanent)
  - Auto-ban when warning threshold reached (configurable)
  - View complete warning history per user
- **User Approval Queue** - Manual approval for new registrations
  - View pending users at `/admin/approval-queue`
  - Approve or reject with optional reason
  - Configurable via `require_user_approval` setting
- **Moderator Notes** - Private notes visible only to staff
  - Notes attached to user profiles
  - Add, view, and delete notes
- **Permission-Based Display** - Moderation tools only visible to users with appropriate permissions
- **Moderation Logging** - All moderation actions logged with reason in `mod_log` table
- **CSRF-Protected Actions** - All moderation operations protected against CSRF attacks

### Permission Groups
- **Custom Permission Groups** - Create and manage user groups at `/admin/groups`
  - Create custom groups with specific permissions
  - Edit group names and permissions
  - Delete custom groups (system groups protected)
  - View member count for each group
- **System Groups** - Built-in groups that cannot be deleted
  - Guests - Read-only access for unauthenticated users
  - Registered Users - Basic permissions for logged-in users
  - Moderators - Content moderation permissions
  - Administrators - Full system access
- **Permission Values** - Granular permission control
  - Yes - Grant the permission
  - No - Deny the permission
  - Never - Permanent deny (cannot be overridden by other groups)
  - Default - Inherit from other groups

### User Information Display
- **Thread Starter Badge** - "OP" badge displayed next to original poster's name
- **User Post Counts** - Total post count shown in message sidebar
- **Join Date Display** - User registration date shown as "Joined: Mon YYYY"
- **User Avatars** - Avatar display with multiple size options (S/M/L)
  - Drag-and-drop upload with image preview
  - Client-side file type and size validation
  - Support for JPEG, PNG, GIF, and WebP formats
- **Custom Title** - User-defined title displayed under username in posts (100 character limit)

### Thread Features
- **Thread Prefixes** - Categorize threads with prefixes like [SOLVED], [QUESTION], [DISCUSSION] displayed as badges
- **Thread Tags** - Add tags during thread creation for categorization and discoverability
  - Comma-separated tag input with auto-slug generation
  - Colored badge display in listings and thread views
  - Forum-specific or global tag scope
  - Automatic tag use count tracking
  - Filter threads by tag via `?tag=slug` query parameter
  - Active filter indicator with clear button
- **Watch Threads** - Subscribe to threads for notifications on new posts
- **Deleted Post Handling** - Placeholder display for deleted posts with deletion timestamp
- **Post History** - Track post edits with revision history
- **Attachments** - File upload support with S3 storage integration
- **Thread Polls** - Create polls when starting threads
  - Single or multiple choice voting with configurable max choices
  - Optional vote changing after initial vote
  - Results visibility before/after voting
  - Optional poll closing date
  - Real-time vote count display with percentage bars
  - Full dark mode support
- **Similar Threads** - Discover related content based on shared tags
  - Displays up to 5 similar threads at the bottom of thread view
  - Ranked by number of matching tags, then recency
  - Shows post count and number of tags in common
- **Post Reactions** - React to posts with emoji reactions (like, thanks, funny, informative, agree, disagree)
  - Toggle reactions on/off with single click
  - Real-time reaction count updates
  - Visual indication of user's own reactions
  - Database-backed with automatic count triggers
- **Quote Reply** - Click Quote button on any post to insert quoted content into reply
  - Inserts `[quote=username]content[/quote]` BBCode
  - Scrolls to and focuses the reply textarea
- **Multi-Quote** - Queue multiple posts to quote at once
  - Click +Quote to add posts to queue (persists across pages via localStorage)
  - Floating indicator shows number of selected quotes
  - "Insert Quotes" inserts all queued quotes at once
  - "Clear" removes all queued quotes
  - Button toggles to -Quote when post is in queue
- **Draft Auto-Save** - Prevent data loss with automatic draft saving
  - Automatically saves post content to localStorage every 2 seconds
  - Restores draft when returning to page (with "Draft restored" indicator)
  - Clears draft on successful form submission
  - Works for thread replies, new threads, and conversations
  - Drafts expire after 7 days
  - "Clear draft" button to discard saved content
- **Quick Reply** - Reply button in thread header for fast access
  - Smooth scroll to reply form
  - Auto-focus on textarea
- **Report Post** - Report posts for moderation review
  - Modal dialog with reason selection (spam, harassment, off-topic, illegal content, misinformation, other)
  - Optional details field (required for "Other" reason)
  - Duplicate report prevention
  - Admin panel for reviewing and managing reports at `/admin/reports`
- **@Mentions** - Tag users in posts with `@username`
  - Autocomplete dropdown while typing
  - Clickable mention links to user profiles
  - Automatic notifications to mentioned users
  - Skips mentions in code blocks and URLs
- **BBCode Formatting** - Rich text formatting for posts
  - **Basic Formatting**: Bold `[b]`, Italic `[i]`, Underline `[u]`, Strikethrough `[s]`, Color `[color=red]`
  - **Text Styling**: Size `[size=8-36]`, Font `[font=arial]` (whitelisted fonts only)
  - **Text Alignment**: Center `[center]`, Left `[left]`, Right `[right]`
  - **Lists**: Unordered `[list][*]`, Numbered `[list=1][*]`, Alphabetic `[list=a][*]` with nesting support
  - **Quotes**: Basic `[quote]` and attributed `[quote=username]` with "username said:" display
  - **Spoilers**: Collapsible content with `[spoiler]` and custom titles `[spoiler=title]`
  - **Code**: Preformatted code blocks `[code]` with preserved whitespace and syntax highlighting
    - Language-specific highlighting with `[code=language]` (e.g., `[code=rust]`, `[code=javascript]`)
    - 50+ languages supported with highlight.js (client-side)
    - Common aliases: js→javascript, py→python, ts→typescript, sh→bash
    - GitHub-inspired color theme with automatic dark mode support
  - **Images**: `[img]` with optional dimensions `[img=200x150]` or width-only `[img=200]`, Links `[url]` with automatic URL detection
  - **Video Embeds**: `[video]` for YouTube, Vimeo, and direct video files (.mp4, .webm, .ogg)
    - YouTube privacy-enhanced embeds via youtube-nocookie.com
    - Responsive 16:9 aspect ratio for embedded players
    - `[youtube]videoId[/youtube]` shorthand for YouTube videos
  - **Audio Embeds**: `[audio]` for audio files (.mp3, .ogg, .wav, .flac, .m4a)
  - **Media Auto-Detect**: `[media]` automatically detects and embeds YouTube, Vimeo, video, or audio based on URL
  - **Tables**: `[table][tr][td]...[/td][/tr][/table]` with header support `[th]`
    - Auto-closing cells when opening new ones
    - Validation ensures proper nesting (cells inside rows, rows inside tables)
    - Responsive styling with dark mode support
  - **URL Unfurl**: Rich link previews with `[url unfurl]https://example.com[/url]`
    - Extracts Open Graph metadata (title, description, image)
    - Shows site favicon and site name
    - 24-hour server-side cache for performance
    - Async JavaScript hydration for fast page loads
    - Responsive card layout with dark mode support
  - **Security**: HTML entity sanitization, XSS prevention at tokenizer level, dimension validation (max 2000px)
  - **BBCode Toolbar**: Visual editor toolbar for post formatting
    - One-click buttons for bold, italic, underline, strikethrough
    - Link and image insertion with prompts
    - Quote, code, and spoiler blocks
    - List creation with automatic item formatting
    - Quick color buttons (red, green, blue) and custom color picker
    - Text size controls (small/large)
  - **Post Preview**: Server-side BBCode preview before posting
    - Toggle between edit and preview modes
    - Real-time rendering via `/api/bbcode/preview` endpoint
    - Shows rendered HTML exactly as it will appear
    - Dark mode support

### Keyboard Shortcuts
- **Post Navigation**: `j`/`k` to navigate between posts (vim-style)
- **Go To Navigation**: `g` then `h` (home), `f` (forums), `n` (new posts), `w` (watched threads)
- **Quick Actions**: `r` (reply), `q` (quote focused post), `/` (focus search)
- **Help & Escape**: `?` (show shortcuts help), `Escape` (close modals, unfocus)
- **Smart Detection**: Shortcuts disabled when typing in text fields
- **Help Modal**: Press `?` to view all available shortcuts

### Responsive Design
- All UI components are mobile-friendly with appropriate breakpoints
- Statistics and metadata hidden on mobile for cleaner layout
- Touch-friendly button sizes and spacing

### Accessibility
- **Skip Links** - Keyboard users can skip to main content
- **ARIA Labels** - Screen reader support for navigation, forms, and interactive elements
- **Semantic Roles** - Proper banner, main, and contentinfo roles
- **Pagination** - Accessible page navigation with current page indication
- **Form Accessibility** - Proper labels and autocomplete attributes

### User Preferences & Customization
- **Dark Mode** - Toggle between light, dark, and auto (system preference) themes
  - Persistent theme preference stored per user
  - Real-time theme switching without page reload
  - Comprehensive dark mode styling for all UI components
  - Auto mode respects operating system dark mode preference
- **Posts Per Page** - Configurable pagination (10, 25, 50, or 100 posts per page)
- **Character Counter** - Real-time character counting for post/thread creation
  - Visual feedback (green/yellow/red) based on remaining characters
  - Automatic limit detection (50,000 for users, 100,000 for moderators)
  - Form submission prevention when over limit
  - Client-side validation before submission

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
- **Email Notifications** - Toggle per-thread email notifications for new replies
  - Visual toggle button in thread header (📧 on, 🔕 off)
  - Only sends to users with verified email addresses
  - Excludes the post author from receiving notifications
- **Manage Subscriptions** - View and manage all watched threads

### Real-Time Chat
- **WebSocket Chat** - Real-time chat with XenForo compatibility layer
- **Chat Rooms** - Multi-room support with user activity tracking
- **Message Operations** - Send, edit, and delete chat messages
  - Soft deletion via UGC system (preserves audit trail)
  - Edit creates new revision (full history preserved)
- **User Presence** - See who's online in each room
- **Smilie Support** - Configurable emoticon replacement

### RSS Feeds
- **Latest Threads Feed** - `/feed.rss` - RSS feed of latest threads across all forums
- **Per-Forum Feeds** - `/forums/{id}/feed.rss` - RSS feed of threads in a specific forum
- **Feed Discovery** - Automatic `<link rel="alternate">` tags for feed reader detection
- **Standard RSS 2.0** - Compatible with all major feed readers

## Configuration

### Configuration File (`config.toml`)

Ruforo supports a TOML configuration file with layered priority:

1. **Environment variables** (`RUFORO_*` prefix) - highest priority
2. **Config file** (`config.toml`) - optional, for non-secret settings
3. **Default values** - sensible secure defaults

Copy `config.toml.example` to `config.toml` and customize as needed.

#### Configuration Sections

| Section | Description |
|---------|-------------|
| `[site]` | Site name, description, base URL |
| `[captcha]` | CAPTCHA provider (hcaptcha/turnstile), site key, failed login threshold |
| `[security]` | Max failed logins, lockout duration, session timeout, remember me duration |
| `[rate_limit]` | Login attempts, registration limits, posts/threads per minute |
| `[limits]` | Posts per page, max upload size, post length limits |
| `[email]` | SMTP host, port, TLS, from address |
| `[storage]` | S3 endpoint, region, bucket |
| `[spam]` | Spam threshold, max URLs, first post URL blocking |

#### Environment Variable Override

All config values can be overridden with environment variables using the `RUFORO_` prefix:

```bash
# Override site name
RUFORO_SITE_NAME=MyForum

# Override CAPTCHA provider
RUFORO_CAPTCHA_PROVIDER=turnstile

# Override rate limits
RUFORO_RATE_LIMIT_LOGIN_MAX_ATTEMPTS=10
```

#### Secrets

Keep secrets in environment variables (not config file):
- `RUFORO_CAPTCHA_SECRET_KEY` - CAPTCHA verification secret
- `RUFORO_EMAIL_SMTP_PASSWORD` - SMTP password
- `RUFORO_STORAGE_S3_ACCESS_KEY` / `RUFORO_STORAGE_S3_SECRET_KEY` - S3 credentials
- `DATABASE_URL` - Database connection string (standard, no prefix)
- `SECRET_KEY` - Session signing key (64+ bytes)

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

## Deployment

Production deployment is supported on bare metal/VMs with systemd services.

### Quick Start

```bash
# Run installation script (as root)
sudo ./deploy/scripts/install.sh

# Configure environment
sudo nano /opt/ruforo/.env

# Deploy latest release
sudo /opt/ruforo/scripts/deploy.sh
```

### Components

| Component | Description |
|-----------|-------------|
| `ruforo.service` | Main forum server (port 8080) |
| `ruforo-xf-chat.service` | XenForo-compatible chat server (port 8081) |
| `nginx` | Reverse proxy with SSL, rate limiting, WebSocket support |

### Features

- **Systemd Services** - Security-hardened with `NoNewPrivileges`, `ProtectSystem=strict`
- **Nginx Configuration** - HTTPS, rate limiting zones, WebSocket upgrade for chat
- **Deployment Scripts** - Automated install, deploy, and backup scripts
- **GitHub Actions CI/CD** - Automated testing, linting, and release builds

### Documentation

See [`deploy/README.md`](deploy/README.md) for detailed deployment instructions including:
- Installation prerequisites
- Environment configuration
- SSL certificate setup
- Backup and restore procedures
- Monitoring and troubleshooting

## Contributions
### Code Guidelines
 - We use [rustfmt](https://github.com/rust-lang/rustfmt).
 - `cargo clippy` whenever possible.
 - Try to eliminate warnings.

### Database Guidelines
 - Any data which would apply to two types of content (i.e. posts, chat messages, profile posts) should interact with the `ugc` tables, not individual content type tables.
 - Usernames should be referenced by `user_id,created_at DESC` from `user_name`. User rows can be deleted, but a historical reference for their name will be added to this table. This complies with [GDPR software requirements](https://gdpr.eu/right-to-be-forgotten).
