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

## Documentation

| Document | Description |
|----------|-------------|
| [Features](docs/features.md) | User interface, navigation, forum features, BBCode, keyboard shortcuts |
| [Security](docs/security.md) | Authentication, CSRF, rate limiting, CAPTCHA, spam detection |
| [Moderation](docs/moderation.md) | Thread moderation, user warnings, bans, permission groups |
| [Communication](docs/communication.md) | Notifications, private messaging, chat, RSS feeds |
| [Configuration](docs/configuration.md) | Environment variables, config file, database setup |
| [Deployment](deploy/README.md) | Production deployment with systemd, nginx, CI/CD |

## Quick Start

### Prerequisites
- Rust (latest stable)
- PostgreSQL 14+
- Node.js 18+ (for frontend assets)
- ffmpeg (for media processing)

### Development Setup

```bash
# Start local services
docker-compose up -d

# Set up environment
cp .env.example .env

# Install frontend dependencies
npm install

# Build frontend assets
npx webpack

# Run database migrations
export DATABASE_URL="postgres://postgres:postgres@localhost:5433/ruforo"
sqlx database create
sqlx migrate run

# Run the forum
cargo run --bin ruforo
```

The forum will be available at http://127.0.0.1:8080

### Running Tests

```bash
# Set up test database
export TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5433/ruforo_test"
TEST_DATABASE_URL="$TEST_DATABASE_URL" sqlx database create
TEST_DATABASE_URL="$TEST_DATABASE_URL" sqlx migrate run

# Run all tests (265+ tests)
TEST_DATABASE_URL="$TEST_DATABASE_URL" cargo test
```

## Features Overview

### Core Forum
- Forums with sub-forums and hierarchical navigation
- Threads with tags, prefixes, and polls
- Posts with BBCode formatting, reactions, and multi-quote
- Full-text search across threads and posts

### User Features
- User profiles with avatars and custom titles
- Online status tracking with privacy controls
- Private messaging and conversations
- Thread watching with email notifications
- Real-time WebSocket chat
- Dark mode and user preferences

### Moderation
- Thread lock/pin/move/merge operations
- User warnings with point system
- User and IP bans with expiration
- Report system for user-submitted reports
- Word filters with replace/block/flag actions
- Mass moderation actions for bulk operations
- Custom permission groups

### Security
- Argon2 password hashing
- Two-factor authentication (TOTP)
- Account lockout protection
- Rate limiting on all endpoints
- CAPTCHA support (hCaptcha, Turnstile)
- CSRF protection on all forms

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
