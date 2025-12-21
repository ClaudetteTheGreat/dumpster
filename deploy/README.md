# Ruforo Deployment Guide

This directory contains deployment configuration for running Ruforo in production.

## Quick Start (Bare Metal / VM)

### Prerequisites

- Debian 12 or Ubuntu 22.04+
- PostgreSQL 14+
- Nginx
- Node.js 20+ (for frontend build)
- ffmpeg

### Installation

1. **Run the installation script** (as root):
   ```bash
   sudo ./deploy/scripts/install.sh
   ```

2. **Configure environment**:
   ```bash
   sudo nano /opt/ruforo/.env
   ```

3. **Set up SSL with Let's Encrypt**:
   ```bash
   sudo certbot --nginx -d forum.example.com
   ```

4. **Download and deploy the latest release**:
   ```bash
   sudo /opt/ruforo/scripts/deploy.sh
   ```

5. **Enable and start services**:
   ```bash
   sudo systemctl enable ruforo ruforo-xf-chat
   sudo systemctl start ruforo
   ```

## Directory Structure

```
/opt/ruforo/
├── bin/           # Binary executables
│   ├── ruforo     # Main forum server
│   └── xf-chat    # XenForo chat compatibility server
├── public/        # Static assets
│   └── assets/    # Compiled JS/CSS
├── templates/     # Askama HTML templates
├── migrations/    # Database migrations
├── tmp/           # Temporary file uploads
├── logs/          # Application logs
├── backups/       # Database backups
├── scripts/       # Deployment scripts
└── .env           # Environment configuration
```

## Configuration

### Environment Variables

Edit `/opt/ruforo/.env`:

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection | `postgres://user:pass@localhost/ruforo` |
| `SECRET_KEY` | Session encryption key | 64-byte hex string |
| `SALT` | Password hashing salt | Random string |
| `AWS_*` | S3/MinIO configuration | See .env.example |
| `SMTP_*` | Email configuration | See .env.example |

### Nginx

The nginx configuration is in `/etc/nginx/sites-available/ruforo.conf`.

Key features:
- HTTPS with modern TLS
- Rate limiting on login/API endpoints
- WebSocket support for chat
- Security headers
- Static asset caching

### Systemd Services

- `ruforo.service` - Main forum server (port 8080)
- `ruforo-xf-chat.service` - Chat server (port 8081)

Commands:
```bash
# View status
sudo systemctl status ruforo

# View logs
sudo journalctl -u ruforo -f

# Restart
sudo systemctl restart ruforo
```

## Deployment

### Manual Deployment

```bash
# Stop services
sudo systemctl stop ruforo ruforo-xf-chat

# Copy new binaries
sudo cp ruforo /opt/ruforo/bin/
sudo cp xf-chat /opt/ruforo/bin/
sudo chown ruforo:ruforo /opt/ruforo/bin/*

# Run migrations
cd /opt/ruforo && sudo -u ruforo sqlx migrate run

# Restart services
sudo systemctl start ruforo ruforo-xf-chat
```

### Automated Deployment

Use the deploy script:
```bash
# Deploy latest release
sudo /opt/ruforo/scripts/deploy.sh

# Deploy specific version
sudo /opt/ruforo/scripts/deploy.sh v1.2.3

# Skip pre-deployment backup
sudo /opt/ruforo/scripts/deploy.sh latest --skip-backup
```

## Backups

Automatic daily backups run at 3:00 AM via cron.

Manual backup:
```bash
sudo -u ruforo /opt/ruforo/scripts/backup.sh
```

Backups are stored in `/opt/ruforo/backups/` with 30-day retention.

### Restore from Backup

```bash
# Stop the service
sudo systemctl stop ruforo

# Restore database
sudo -u postgres pg_restore -d ruforo /opt/ruforo/backups/db_YYYYMMDD_HHMMSS.dump

# Start the service
sudo systemctl start ruforo
```

## Monitoring

### Health Check

```bash
curl http://localhost:8080/
```

### Logs

```bash
# Application logs
sudo journalctl -u ruforo -f

# Nginx access logs
sudo tail -f /var/log/nginx/ruforo_access.log

# Nginx error logs
sudo tail -f /var/log/nginx/ruforo_error.log
```

## Security

### Firewall

Only ports 80 and 443 should be exposed:
```bash
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable
```

### Updates

Keep the system updated:
```bash
sudo apt update && sudo apt upgrade
```

### SSL Certificate Renewal

Certbot auto-renews certificates. Verify:
```bash
sudo certbot renew --dry-run
```

## Troubleshooting

### Service won't start

Check logs:
```bash
sudo journalctl -u ruforo -n 50 --no-pager
```

Common issues:
- Missing environment variables in `.env`
- Database connection refused
- Permission issues on directories

### 502 Bad Gateway

The backend is not responding. Check:
1. Is the service running? `systemctl status ruforo`
2. Is it bound to the correct port? `ss -tlnp | grep 8080`
3. Check application logs for errors

### Database migration fails

Ensure the database user has sufficient privileges:
```sql
GRANT ALL PRIVILEGES ON DATABASE ruforo TO ruforo;
```
