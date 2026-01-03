# Configuration

This document covers the configuration options for the forum.

## Configuration File (`config.toml`)

Ruforo supports a TOML configuration file with layered priority:

1. **Environment variables** (`RUFORO_*` prefix) - highest priority
2. **Config file** (`config.toml`) - optional, for non-secret settings
3. **Default values** - sensible secure defaults

Copy `config.toml.example` to `config.toml` and customize as needed.

## Configuration Sections

| Section | Description |
|---------|-------------|
| `[site]` | Site name, description, base URL |
| `[captcha]` | CAPTCHA provider (hcaptcha/turnstile), site key, failed login threshold |
| `[security]` | Max failed logins, lockout duration, session timeout, remember me duration |
| `[rate_limit]` | Login attempts, registration limits, posts/threads per minute |
| `[limits]` | Posts per page, max upload size, post length limits |
| `[email]` | SMTP host, port, TLS, from address |
| `[storage]` | Storage backend (local/s3), paths, S3 settings |
| `[spam]` | Spam threshold, max URLs, first post URL blocking |

## Environment Variable Override

All config values can be overridden with environment variables using the `RUFORO_` prefix:

```bash
# Override site name
RUFORO_SITE_NAME=MyForum

# Override CAPTCHA provider
RUFORO_CAPTCHA_PROVIDER=turnstile

# Override rate limits
RUFORO_RATE_LIMIT_LOGIN_MAX_ATTEMPTS=10
```

## Secrets

Keep secrets in environment variables (not config file):

| Variable | Description |
|----------|-------------|
| `RUFORO_CAPTCHA_SECRET_KEY` | CAPTCHA verification secret |
| `RUFORO_EMAIL_SMTP_PASSWORD` | SMTP password |
| `RUFORO_STORAGE_S3_ACCESS_KEY` | S3 access key |
| `RUFORO_STORAGE_S3_SECRET_KEY` | S3 secret key |
| `DATABASE_URL` | Database connection string (no prefix) |
| `SECRET_KEY` | Session signing key (64+ bytes) |

## Environment Setup

### Required Environment Variables

```bash
# Database
DATABASE_URL=postgres://user:password@localhost:5432/ruforo

# Session security
SECRET_KEY=your-64-byte-secret-key-here
```

### Optional Environment Variables

```bash
# S3 Storage
AWS_ACCESS_KEY_ID=your-access-key
AWS_SECRET_ACCESS_KEY=your-secret-key
AWS_REGION=us-east-1
S3_BUCKET=ruforo-uploads
S3_ENDPOINT=https://s3.amazonaws.com

# CAPTCHA (disabled if not set)
CAPTCHA_PROVIDER=hcaptcha
CAPTCHA_SITE_KEY=your-site-key
CAPTCHA_SECRET_KEY=your-secret-key

# Email (for notifications and password reset)
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=noreply@example.com
SMTP_PASSWORD=your-smtp-password
SMTP_FROM=noreply@example.com
```

## Storage Configuration

Ruforo supports two storage backends for file uploads:

### Local Storage (Default)

Stores files on the local filesystem. Recommended for development and simple deployments.

```toml
[storage]
backend = "local"
local_path = "./uploads"
```

Files are stored with a prefix structure: `./uploads/{hash[0:2]}/{hash[2:4]}/{filename}`

The directory is created automatically on first upload.

### S3 Storage

Stores files in S3-compatible object storage (AWS S3, MinIO, etc.).

```toml
[storage]
backend = "s3"
s3_endpoint = "http://localhost:9000"
s3_region = "us-east-1"
s3_bucket = "ruforo"
s3_public_url = "http://localhost:9000/ruforo"
```

S3 credentials should be set via environment variables:
```bash
RUFORO_STORAGE_S3_ACCESS_KEY=your-access-key
RUFORO_STORAGE_S3_SECRET_KEY=your-secret-key
```

### Migrating from S3 to Local

If you have existing files in S3/MinIO and want to switch to local storage:

1. Copy files from S3 to local (preserving directory structure)
2. Change config to `backend = "local"`
3. Restart the server

Files uploaded after migration will be stored locally. Existing database records will work as files are re-uploaded (deduplication checks storage, not just database).

## Development Environment

### Docker Compose Services

The `docker-compose.yml` provides local development services:

```bash
docker-compose up -d
```

| Service | Port | Credentials |
|---------|------|-------------|
| PostgreSQL | 5433 | postgres/postgres |
| MinIO (S3) | 9000/9001 | minioadmin/minioadmin |

### Example `.env` for Development

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5433/ruforo
SECRET_KEY=development-secret-key-64-bytes-minimum-required-for-security
AWS_ACCESS_KEY_ID=minioadmin
AWS_SECRET_ACCESS_KEY=minioadmin
S3_ENDPOINT=http://localhost:9000
S3_BUCKET=ruforo-uploads
```

## Test Database Setup

```bash
# Set up test database (required for running tests)
export TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5433/ruforo_test"

# Create and migrate test database
TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5433/ruforo_test" sqlx database create
TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5433/ruforo_test" sqlx migrate run

# Run tests
TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5433/ruforo_test" cargo test
```

## Database Migrations

Migrations are managed with sqlx-cli:

```bash
# Install sqlx-cli (one-time)
cargo install sqlx-cli --no-default-features --features postgres

# Create database (one-time)
sqlx database create

# Run pending migrations
sqlx migrate run

# Rollback last migration
sqlx migrate revert

# Show migration status
sqlx migrate info

# Create new migration
sqlx migrate add <name>
```

## Feature Flags

Runtime feature flags can be managed at `/admin/feature-flags`:

- Enable/disable features without code deployment
- Per-feature toggle with description
- Changes take effect immediately

## Admin Settings

Site-wide settings can be configured at `/admin/settings`:

### General Settings
- **site_name** - Name of the forum (used in meta tags)
- **site_title** - Brand name displayed in navigation header
- **site_description** - Site description for meta tags
- **footer_message** - Message displayed in site footer
- **site_url** - Base URL of the site
- **timezone** - Default timezone

### Display Settings
- **posts_per_page** - Default posts per page
- **threads_per_page** - Threads per page in forum list
- **enforce_thumbnails** - Force inserted images to use thumbnail format (default: false)
- **thumbnail_max_size** - Maximum thumbnail size in pixels (default: 150)

### Security Settings
- **registration_enabled** - Allow new user registrations
- **session_timeout_minutes** - Session timeout duration
- **max_login_attempts** - Maximum login attempts before lockout
- **lockout_duration_minutes** - Account lockout duration

### Feature Toggles
- **maintenance_mode** - Put site in maintenance mode
- **chat_enabled** - Enable real-time chat feature
- **reactions_enabled** - Enable post reactions
- **polls_enabled** - Enable thread polls
- **signatures_enabled** - Show user signatures in posts

### Storage Settings
- **max_upload_size_mb** - Maximum file upload size in MB
- **max_avatar_size_kb** - Maximum avatar file size in KB

### Chat Settings
- **chat_enabled** - Enable/disable real-time chat feature
- **chat_history_limit** - Number of messages to load when joining a room (default: 40)
- **chat_rate_limit_seconds** - Minimum seconds between messages per user (0 = disabled)
- **chat_default_room** - Default room ID to auto-join (0 = none)
- **chat_max_message_length** - Maximum chat message length in bytes (default: 1024)
- **chat_embed_youtube** - Allow YouTube video embeds in chat messages (default: true)
- **chat_image_domain_whitelist** - Comma-separated list of domains allowed to show image thumbnails
  - Use `*` to allow all domains (default)
  - Supports subdomains (e.g., `example.com` also allows `cdn.example.com`)
  - Non-whitelisted images render as clickable text links instead of thumbnails
