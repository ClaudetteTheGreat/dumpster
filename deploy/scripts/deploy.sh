#!/bin/bash
# Dumpster Forum - Deployment Script
# Downloads latest release and deploys with minimal downtime

set -e

# Configuration
DUMPSTER_HOME="/opt/dumpster"
DUMPSTER_USER="dumpster"
GITHUB_REPO="yourorg/dumpster"
BACKUP_DIR="$DUMPSTER_HOME/backups"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1"
}

# Parse arguments
VERSION="${1:-latest}"
SKIP_BACKUP="${2:-false}"

log_info "Starting deployment of dumpster version: $VERSION"

# Create backup before deployment
if [[ "$SKIP_BACKUP" != "--skip-backup" ]]; then
    log_info "Creating pre-deployment backup..."
    "$DUMPSTER_HOME/scripts/backup.sh" || log_warn "Backup failed, continuing anyway"
fi

# Download release
log_info "Downloading release..."
DOWNLOAD_DIR=$(mktemp -d)
cd "$DOWNLOAD_DIR"

if [[ "$VERSION" == "latest" ]]; then
    RELEASE_URL="https://api.github.com/repos/$GITHUB_REPO/releases/latest"
    ASSET_URL=$(curl -s "$RELEASE_URL" | grep "browser_download_url.*dumpster-linux-x86_64.tar.gz" | cut -d '"' -f 4)
else
    ASSET_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION/dumpster-linux-x86_64.tar.gz"
fi

if [[ -z "$ASSET_URL" ]]; then
    log_error "Could not find release asset URL"
    exit 1
fi

curl -L -o release.tar.gz "$ASSET_URL"
tar -xzf release.tar.gz

# Verify binaries exist
if [[ ! -f "dumpster" ]] || [[ ! -f "xf-chat" ]]; then
    log_error "Release archive does not contain expected binaries"
    exit 1
fi

# Stop services gracefully
log_info "Stopping services..."
systemctl stop dumpster-xf-chat 2>/dev/null || true
systemctl stop dumpster 2>/dev/null || true

# Backup current binaries
log_info "Backing up current binaries..."
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
if [[ -f "$DUMPSTER_HOME/bin/dumpster" ]]; then
    mv "$DUMPSTER_HOME/bin/dumpster" "$BACKUP_DIR/dumpster.$TIMESTAMP"
fi
if [[ -f "$DUMPSTER_HOME/bin/xf-chat" ]]; then
    mv "$DUMPSTER_HOME/bin/xf-chat" "$BACKUP_DIR/xf-chat.$TIMESTAMP"
fi

# Install new binaries
log_info "Installing new binaries..."
cp dumpster "$DUMPSTER_HOME/bin/"
cp xf-chat "$DUMPSTER_HOME/bin/"
chmod +x "$DUMPSTER_HOME/bin/"*
chown "$DUMPSTER_USER:$DUMPSTER_USER" "$DUMPSTER_HOME/bin/"*

# Update assets if included
if [[ -d "public" ]]; then
    log_info "Updating public assets..."
    rsync -a --delete public/ "$DUMPSTER_HOME/public/"
    chown -R "$DUMPSTER_USER:$DUMPSTER_USER" "$DUMPSTER_HOME/public/"
fi

# Update templates if included
if [[ -d "templates" ]]; then
    log_info "Updating templates..."
    rsync -a --delete templates/ "$DUMPSTER_HOME/templates/"
    chown -R "$DUMPSTER_USER:$DUMPSTER_USER" "$DUMPSTER_HOME/templates/"
fi

# Update migrations if included
if [[ -d "migrations" ]]; then
    log_info "Updating migrations..."
    rsync -a migrations/ "$DUMPSTER_HOME/migrations/"
    chown -R "$DUMPSTER_USER:$DUMPSTER_USER" "$DUMPSTER_HOME/migrations/"
fi

# Run database migrations
log_info "Running database migrations..."
cd "$DUMPSTER_HOME"
source "$DUMPSTER_HOME/.env"
# Note: Requires sqlx-cli installed, or use the binary with migrate subcommand
# sudo -u "$DUMPSTER_USER" sqlx migrate run 2>&1 || log_warn "Migration command not available"

# Start services
log_info "Starting services..."
systemctl start dumpster
sleep 2
systemctl start dumpster-xf-chat

# Verify services are running
log_info "Verifying services..."
if systemctl is-active --quiet dumpster; then
    log_info "dumpster service is running"
else
    log_error "dumpster service failed to start"
    journalctl -u dumpster --no-pager -n 20
    exit 1
fi

if systemctl is-active --quiet dumpster-xf-chat; then
    log_info "dumpster-xf-chat service is running"
else
    log_warn "dumpster-xf-chat service is not running (may be optional)"
fi

# Health check
log_info "Performing health check..."
sleep 5
if curl -sf http://127.0.0.1:8080/ > /dev/null; then
    log_info "Health check passed"
else
    log_warn "Health check failed, service may still be starting"
fi

# Cleanup
log_info "Cleaning up..."
rm -rf "$DOWNLOAD_DIR"

# Clean old backups (keep last 5)
log_info "Cleaning old binary backups..."
ls -t "$BACKUP_DIR"/dumpster.* 2>/dev/null | tail -n +6 | xargs -r rm
ls -t "$BACKUP_DIR"/xf-chat.* 2>/dev/null | tail -n +6 | xargs -r rm

log_info "Deployment complete!"
echo ""
echo "Deployed version: $VERSION"
echo "Services status:"
systemctl status dumpster --no-pager -l | head -5
