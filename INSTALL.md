# Ruforo Installation Guide

This guide covers installing and running Ruforo, a Rust-based web forum.

## Prerequisites

### Required Software

- **Rust** (1.70+) - https://rustup.rs/
- **Node.js** (18+) - https://nodejs.org/
- **PostgreSQL** (14+) - https://www.postgresql.org/
- **FFmpeg development libraries** - Required for media processing

### Installing System Dependencies

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install -y \
    pkg-config \
    libssl-dev \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libavfilter-dev \
    libavdevice-dev \
    libswscale-dev \
    libswresample-dev
```

#### Fedora/RHEL
```bash
sudo dnf install -y \
    pkg-config \
    openssl-devel \
    ffmpeg-devel
```

#### macOS
```bash
brew install pkg-config openssl ffmpeg
```

#### Arch Linux
```bash
sudo pacman -S pkg-config openssl ffmpeg
```

## Quick Start (Development)

```bash
# 1. Clone the repository
git clone https://github.com/your-org/ruforo.git
cd ruforo

# 2. Start PostgreSQL (using Docker)
docker-compose up -d postgres

# 3. Set up environment
cp .env.example .env
cp config.toml.example config.toml

# 4. Install sqlx-cli and set up database
cargo install sqlx-cli --no-default-features --features postgres
export DATABASE_URL="postgres://postgres:postgres@localhost:5433/ruforo"
sqlx database create
sqlx migrate run

# 5. Build frontend assets
npm install
npm run build

# 6. Run the server
cargo run --bin ruforo
```

The forum will be available at http://localhost:8080

## Detailed Installation

### Step 1: Database Setup

#### Option A: Docker (Recommended for Development)
```bash
docker-compose up -d postgres
```
This starts PostgreSQL on port 5433 with credentials `postgres:postgres`.

#### Option B: Native PostgreSQL
```bash
# Create database and user
sudo -u postgres psql
CREATE USER ruforo WITH PASSWORD 'your_password';
CREATE DATABASE ruforo OWNER ruforo;
\q
```

### Step 2: Environment Configuration

Copy the example files:
```bash
cp .env.example .env
cp config.toml.example config.toml
```

Edit `.env` with your settings:
```bash
# Required
DATABASE_URL=postgres://postgres:postgres@localhost:5433/ruforo
SECRET_KEY=generate_a_64_byte_secure_random_string
SALT=generate_a_secure_salt_string

# Email (optional for development)
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=noreply@example.com
SMTP_PASSWORD=your_smtp_password
SMTP_FROM_EMAIL=noreply@example.com

# Base URL for email links
BASE_URL=http://localhost:8080
```

Generate secure keys:
```bash
# Generate SECRET_KEY (64 bytes)
openssl rand -base64 64 | tr -d '\n'

# Generate SALT
openssl rand -base64 32 | tr -d '\n'
```

### Step 3: Database Migrations

Install the sqlx CLI tool:
```bash
cargo install sqlx-cli --no-default-features --features postgres
```

Run migrations:
```bash
export DATABASE_URL="postgres://postgres:postgres@localhost:5433/ruforo"
sqlx database create
sqlx migrate run
```

Verify migrations:
```bash
sqlx migrate info
```

### Step 4: Build Frontend Assets

```bash
npm install
npm run build
```

This compiles JavaScript and CSS to `public/assets/`.

### Step 5: Build and Run

#### Development
```bash
cargo run --bin ruforo
```

#### Production
```bash
cargo build --release
./target/release/ruforo
```

## Configuration Reference

### config.toml

The `config.toml` file contains application settings. Key sections:

```toml
[site]
name = "Ruforo"
description = "A forum built in Rust"
base_url = "http://localhost:8080"

[storage]
# "local" for filesystem, "s3" for S3-compatible storage
backend = "local"
local_path = "./uploads"

[security]
max_failed_logins = 5
lockout_duration_minutes = 15
session_timeout_minutes = 1440

[captcha]
# "hcaptcha", "turnstile", or "" to disable
provider = ""
site_key = ""
# Set secret via RUFORO_CAPTCHA_SECRET_KEY env var
```

### Environment Variables

Environment variables override config file values. Use the `RUFORO_` prefix:

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | PostgreSQL connection string |
| `SECRET_KEY` | Session encryption key (64 bytes) |
| `SALT` | Password hashing salt |
| `RUFORO_CAPTCHA_SECRET_KEY` | CAPTCHA secret key |
| `RUFORO_EMAIL_SMTP_PASSWORD` | SMTP password |
| `RUFORO_STORAGE_S3_ACCESS_KEY` | S3 access key |
| `RUFORO_STORAGE_S3_SECRET_KEY` | S3 secret key |

## File Storage

### Local Storage (Default)
Files are stored in `./uploads/` by default. No additional setup required.

```toml
[storage]
backend = "local"
local_path = "./uploads"
```

### S3-Compatible Storage
For production, use S3 or a compatible service (MinIO, DigitalOcean Spaces, etc.):

```toml
[storage]
backend = "s3"
s3_endpoint = "https://s3.amazonaws.com"
s3_region = "us-east-1"
s3_bucket = "your-bucket"
s3_public_url = "https://your-bucket.s3.amazonaws.com"
```

Set credentials via environment:
```bash
export RUFORO_STORAGE_S3_ACCESS_KEY="your-access-key"
export RUFORO_STORAGE_S3_SECRET_KEY="your-secret-key"
```

#### Development with MinIO
```bash
docker-compose up -d minio
```
MinIO console: http://localhost:9001 (minioadmin/minioadmin)

## Running Tests

```bash
# Set up test database
export TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5433/ruforo_test"
sqlx database create
sqlx migrate run

# Run tests
cargo test
```

## Production Deployment

### Recommended Setup

1. **Reverse Proxy**: Use nginx or Caddy in front of Ruforo
2. **Process Manager**: Use systemd or supervisor
3. **Database**: Use managed PostgreSQL or secure your installation
4. **Storage**: Use S3 or compatible object storage
5. **TLS**: Enable HTTPS via reverse proxy

### Example systemd Service

```ini
[Unit]
Description=Ruforo Forum
After=network.target postgresql.service

[Service]
Type=simple
User=ruforo
WorkingDirectory=/opt/ruforo
Environment=DATABASE_URL=postgres://ruforo:password@localhost/ruforo
Environment=SECRET_KEY=your_secret_key
Environment=RUST_LOG=info
ExecStart=/opt/ruforo/target/release/ruforo
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### Example nginx Configuration

```nginx
server {
    listen 443 ssl http2;
    server_name forum.example.com;

    ssl_certificate /etc/letsencrypt/live/forum.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/forum.example.com/privkey.pem;

    client_max_body_size 50M;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /static/ {
        alias /opt/ruforo/public/;
        expires 30d;
    }
}
```

## Troubleshooting

### FFmpeg linking errors
Ensure FFmpeg development libraries are installed:
```bash
pkg-config --libs libavcodec libavformat
```

### Database connection errors
Verify PostgreSQL is running and credentials are correct:
```bash
psql $DATABASE_URL -c "SELECT 1"
```

### Migration errors
Check migration status and re-run if needed:
```bash
sqlx migrate info
sqlx migrate run
```

### Permission denied on uploads
Ensure the uploads directory is writable:
```bash
mkdir -p uploads
chmod 755 uploads
```

## Additional Resources

- [CLAUDE.md](./CLAUDE.md) - Development commands and architecture overview
- [migrations/](./migrations/) - Database migration files
- [config.toml.example](./config.toml.example) - Full configuration reference
