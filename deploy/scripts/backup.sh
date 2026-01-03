#!/bin/bash
# Dumpster Forum - Backup Script
# Creates database backups and manages retention

set -e

# Configuration
DUMPSTER_HOME="/opt/dumpster"
BACKUP_DIR="$DUMPSTER_HOME/backups"
RETENTION_DAYS=30
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Load environment
source "$DUMPSTER_HOME/.env"

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

# Ensure backup directory exists
mkdir -p "$BACKUP_DIR"

log_info "Starting backup..."

# Extract database connection details from DATABASE_URL
# Format: postgres://user:password@host:port/database
DB_URL="$DATABASE_URL"
DB_USER=$(echo "$DB_URL" | sed -n 's|postgres://\([^:]*\):.*|\1|p')
DB_HOST=$(echo "$DB_URL" | sed -n 's|.*@\([^:/]*\).*|\1|p')
DB_PORT=$(echo "$DB_URL" | sed -n 's|.*:\([0-9]*\)/.*|\1|p')
DB_NAME=$(echo "$DB_URL" | sed -n 's|.*/\([^?]*\).*|\1|p')
DB_PASS=$(echo "$DB_URL" | sed -n 's|.*://[^:]*:\([^@]*\)@.*|\1|p')

# Default port if not specified
DB_PORT="${DB_PORT:-5432}"

log_info "Backing up database: $DB_NAME"

# Create database backup
BACKUP_FILE="$BACKUP_DIR/db_${TIMESTAMP}.dump"
PGPASSWORD="$DB_PASS" pg_dump \
    -h "$DB_HOST" \
    -p "$DB_PORT" \
    -U "$DB_USER" \
    -Fc \
    -f "$BACKUP_FILE" \
    "$DB_NAME"

# Verify backup was created
if [[ -f "$BACKUP_FILE" ]]; then
    BACKUP_SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
    log_info "Database backup created: $BACKUP_FILE ($BACKUP_SIZE)"
else
    log_error "Failed to create database backup"
    exit 1
fi

# Compress older backups (older than 1 day)
log_info "Compressing old backups..."
find "$BACKUP_DIR" -name "db_*.dump" -mtime +1 -exec gzip {} \; 2>/dev/null || true

# Delete old backups (older than retention period)
log_info "Cleaning up old backups (older than $RETENTION_DAYS days)..."
find "$BACKUP_DIR" -name "db_*.dump.gz" -mtime +$RETENTION_DAYS -delete 2>/dev/null || true
find "$BACKUP_DIR" -name "db_*.dump" -mtime +$RETENTION_DAYS -delete 2>/dev/null || true

# Upload to S3 if configured
if [[ -n "$BACKUP_S3_BUCKET" ]] && command -v aws &> /dev/null; then
    log_info "Uploading backup to S3..."
    aws s3 cp "$BACKUP_FILE" "s3://$BACKUP_S3_BUCKET/backups/db_${TIMESTAMP}.dump" \
        --storage-class STANDARD_IA
    log_info "Backup uploaded to S3"
fi

# List recent backups
log_info "Recent backups:"
ls -lh "$BACKUP_DIR"/db_* 2>/dev/null | tail -5

# Calculate total backup size
TOTAL_SIZE=$(du -sh "$BACKUP_DIR" | cut -f1)
log_info "Total backup directory size: $TOTAL_SIZE"

log_info "Backup completed successfully"
