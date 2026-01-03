# Dumpster

A traditional web forum built in Rust.

## Stack

- **Backend**: Rust with Actix-Web
- **Database**: PostgreSQL with SeaORM
- **Templates**: Askama
- **Storage**: S3-compatible (MinIO for development)
- **Frontend**: Webpack + SWC for JS/SCSS compilation

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
# Start local services (PostgreSQL on 5433, MinIO on 9000/9001)
docker-compose up -d

# Set up environment
cp .env.example .env

# Install frontend dependencies and build assets
npm install
npm run build

# Set database URL
export DATABASE_URL="postgres://postgres:postgres@localhost:5433/dumpster"

# Create database and run migrations
sqlx database create
sqlx migrate run

# Run the forum (binds to 0.0.0.0:8080)
cargo run --bin dumpster
```

The forum will be available at http://localhost:8080

### Running Tests

```bash
# Set up test database
export TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5433/dumpster_test"
sqlx database create
sqlx migrate run

# Run all tests
cargo test
```

## Features Overview

### Core Forum

- Forums with sub-forums and hierarchical navigation
- Threads with tags, prefixes, and polls
- Posts with BBCode formatting, reactions, and multi-quote
- Full-text search across threads and posts
- Activity feeds (personal, global, per-user)

### User Features

- User profiles with avatars and custom titles
- Reputation system based on post reactions
- Online status tracking with privacy controls
- Private messaging and conversations
- Thread watching with email notifications
- Real-time WebSocket chat
- Dark mode and theme support
- User following system

### Moderation

- Thread lock/pin/move/merge operations
- User warnings with point system
- User and IP bans with expiration
- Report system for user-submitted reports
- Word filters with replace/block/flag actions
- Mass moderation actions for bulk operations
- Custom permission groups with forum-specific overrides

### Security

- Argon2 password hashing
- Two-factor authentication (TOTP)
- Account lockout protection
- Configurable rate limiting on all endpoints
- CAPTCHA support (hCaptcha, Turnstile)
- CSRF protection on all forms
- Spam detection with heuristic analysis

## Development

### Code Guidelines

- We use [rustfmt](https://github.com/rust-lang/rustfmt) for formatting
- Run `cargo clippy` before commits
- Try to eliminate warnings

### Database Guidelines

- Any data which would apply to multiple content types (posts, chat messages, profile posts) should use the `ugc` tables
- Usernames are referenced via `user_name` table with `(user_id, created_at DESC)`. User rows can be deleted while preserving historical username references (GDPR compliant)

### WebM Validation

Supported codecs:
- Video: VP8, VP9, AV1
- Audio: Opus, Vorbis
