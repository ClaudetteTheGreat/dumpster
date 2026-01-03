#!/bin/bash
# Dumpster Forum - Installation Script
# Run as root on a fresh Debian/Ubuntu server

set -e

# Configuration
DUMPSTER_USER="dumpster"
DUMPSTER_HOME="/opt/dumpster"
DUMPSTER_REPO="https://github.com/yourorg/dumpster"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    log_error "This script must be run as root"
    exit 1
fi

log_info "Starting Dumpster installation..."

# Update system
log_info "Updating system packages..."
apt-get update
apt-get upgrade -y

# Install dependencies
log_info "Installing dependencies..."
apt-get install -y \
    curl \
    wget \
    git \
    nginx \
    postgresql \
    postgresql-contrib \
    ffmpeg \
    certbot \
    python3-certbot-nginx \
    build-essential \
    pkg-config \
    libssl-dev \
    unzip

# Install Node.js (for frontend build)
log_info "Installing Node.js..."
if ! command -v node &> /dev/null; then
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
    apt-get install -y nodejs
fi

# Create dumpster user
log_info "Creating dumpster user..."
if ! id "$DUMPSTER_USER" &>/dev/null; then
    useradd -r -m -d "$DUMPSTER_HOME" -s /bin/bash "$DUMPSTER_USER"
fi

# Create directory structure
log_info "Creating directory structure..."
mkdir -p "$DUMPSTER_HOME"/{bin,public,templates,migrations,tmp,logs,backups}
chown -R "$DUMPSTER_USER:$DUMPSTER_USER" "$DUMPSTER_HOME"

# Set up PostgreSQL
log_info "Setting up PostgreSQL..."
sudo -u postgres psql -c "CREATE USER dumpster WITH PASSWORD 'changeme';" 2>/dev/null || log_warn "User dumpster already exists"
sudo -u postgres psql -c "CREATE DATABASE dumpster OWNER dumpster;" 2>/dev/null || log_warn "Database dumpster already exists"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE dumpster TO dumpster;"

# Copy systemd service files
log_info "Installing systemd services..."
cp deploy/systemd/dumpster.service /etc/systemd/system/
cp deploy/systemd/dumpster-xf-chat.service /etc/systemd/system/
systemctl daemon-reload

# Copy nginx configuration
log_info "Installing nginx configuration..."
cp deploy/nginx/dumpster.conf /etc/nginx/sites-available/
ln -sf /etc/nginx/sites-available/dumpster.conf /etc/nginx/sites-enabled/

# Remove default nginx site
rm -f /etc/nginx/sites-enabled/default

# Create .env template
log_info "Creating environment template..."
if [[ ! -f "$DUMPSTER_HOME/.env" ]]; then
    cat > "$DUMPSTER_HOME/.env" << 'EOF'
# Dumpster Configuration
# Edit this file with your production values

APP_NAME=dumpster

# Generate with: openssl rand -hex 32
SALT=CHANGE_ME_GENERATE_RANDOM_STRING
SECRET_KEY=CHANGE_ME_GENERATE_64_BYTE_HEX_STRING

# Database
DATABASE_URL=postgres://dumpster:changeme@localhost/dumpster

# S3 Storage (MinIO or AWS)
AWS_REGION_NAME=us-east-1
AWS_BUCKET_NAME=dumpster
AWS_API_ENDPOINT=https://s3.example.com
AWS_PUBLIC_URL=https://cdn.example.com
AWS_ACCESS_KEY_ID=your_access_key
AWS_SECRET_ACCESS_KEY=your_secret_key

# File paths
DIR_TMP=/opt/dumpster/tmp

# Session
SESSION_TIME=1440

# SMTP (Email)
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=noreply@example.com
SMTP_PASSWORD=your_smtp_password
SMTP_FROM_EMAIL=noreply@example.com
SMTP_FROM_NAME=Dumpster Forum
SMTP_USE_TLS=true
SMTP_MOCK=false

# Chat
CHAT_ASSET_DIR=/opt/dumpster/public/assets
CHAT_WS_BIND=127.0.0.1:8080
CHAT_WS_URL=wss://forum.example.com/chat.ws

# Logging
RUST_LOG=info
EOF
    chown "$DUMPSTER_USER:$DUMPSTER_USER" "$DUMPSTER_HOME/.env"
    chmod 600 "$DUMPSTER_HOME/.env"
fi

# Set up log rotation
log_info "Setting up log rotation..."
cat > /etc/logrotate.d/dumpster << 'EOF'
/opt/dumpster/logs/*.log {
    daily
    missingok
    rotate 14
    compress
    delaycompress
    notifempty
    create 0640 dumpster dumpster
    sharedscripts
    postrotate
        systemctl reload dumpster >/dev/null 2>&1 || true
    endscript
}
EOF

# Set up backup cron job
log_info "Setting up backup cron job..."
cat > /etc/cron.d/dumpster-backup << 'EOF'
# Daily database backup at 3:00 AM
0 3 * * * dumpster /opt/dumpster/scripts/backup.sh >> /opt/dumpster/logs/backup.log 2>&1
EOF

# Create scripts directory and copy scripts
mkdir -p "$DUMPSTER_HOME/scripts"
cp deploy/scripts/backup.sh "$DUMPSTER_HOME/scripts/"
cp deploy/scripts/deploy.sh "$DUMPSTER_HOME/scripts/"
chmod +x "$DUMPSTER_HOME/scripts/"*.sh
chown -R "$DUMPSTER_USER:$DUMPSTER_USER" "$DUMPSTER_HOME/scripts"

log_info "Installation complete!"
echo ""
echo "Next steps:"
echo "  1. Edit $DUMPSTER_HOME/.env with your production values"
echo "  2. Update /etc/nginx/sites-available/dumpster.conf with your domain"
echo "  3. Run: certbot --nginx -d forum.example.com"
echo "  4. Copy binaries to $DUMPSTER_HOME/bin/"
echo "  5. Copy public assets to $DUMPSTER_HOME/public/"
echo "  6. Copy templates to $DUMPSTER_HOME/templates/"
echo "  7. Run migrations: sudo -u dumpster $DUMPSTER_HOME/bin/sqlx migrate run"
echo "  8. Start services: systemctl enable --now dumpster"
echo "  9. Verify: systemctl status dumpster"
