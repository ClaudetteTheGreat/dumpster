# Dumpster Deployment Guide

This directory contains deployment configuration for running Dumpster in production.

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
   sudo nano /opt/dumpster/.env
   ```

3. **Set up SSL with Let's Encrypt**:
   ```bash
   sudo certbot --nginx -d forum.example.com
   ```

4. **Download and deploy the latest release**:
   ```bash
   sudo /opt/dumpster/scripts/deploy.sh
   ```

5. **Enable and start services**:
   ```bash
   sudo systemctl enable dumpster dumpster-xf-chat
   sudo systemctl start dumpster
   ```

## Directory Structure

```
/opt/dumpster/
├── bin/           # Binary executables
│   ├── dumpster   # Main forum server
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

Edit `/opt/dumpster/.env`:

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection | `postgres://user:pass@localhost/dumpster` |
| `SECRET_KEY` | Session encryption key | 64-byte hex string |
| `SALT` | Password hashing salt | Random string |
| `AWS_*` | S3/MinIO configuration | See .env.example |
| `SMTP_*` | Email configuration | See .env.example |

### Nginx

The nginx configuration is in `/etc/nginx/sites-available/dumpster.conf`.

Key features:
- HTTPS with modern TLS
- Rate limiting on login/API endpoints
- WebSocket support for chat
- Security headers
- Static asset caching

### Systemd Services

- `dumpster.service` - Main forum server (port 8080)
- `dumpster-xf-chat.service` - Chat server (port 8081)

Commands:
```bash
# View status
sudo systemctl status dumpster

# View logs
sudo journalctl -u dumpster -f

# Restart
sudo systemctl restart dumpster
```

## Deployment

### Manual Deployment

```bash
# Stop services
sudo systemctl stop dumpster dumpster-xf-chat

# Copy new binaries
sudo cp dumpster /opt/dumpster/bin/
sudo cp xf-chat /opt/dumpster/bin/
sudo chown dumpster:dumpster /opt/dumpster/bin/*

# Run migrations
cd /opt/dumpster && sudo -u dumpster sqlx migrate run

# Restart services
sudo systemctl start dumpster dumpster-xf-chat
```

### Automated Deployment

Use the deploy script:
```bash
# Deploy latest release
sudo /opt/dumpster/scripts/deploy.sh

# Deploy specific version
sudo /opt/dumpster/scripts/deploy.sh v1.2.3

# Skip pre-deployment backup
sudo /opt/dumpster/scripts/deploy.sh latest --skip-backup
```

## Backups

Automatic daily backups run at 3:00 AM via cron.

Manual backup:
```bash
sudo -u dumpster /opt/dumpster/scripts/backup.sh
```

Backups are stored in `/opt/dumpster/backups/` with 30-day retention.

### Restore from Backup

```bash
# Stop the service
sudo systemctl stop dumpster

# Restore database
sudo -u postgres pg_restore -d dumpster /opt/dumpster/backups/db_YYYYMMDD_HHMMSS.dump

# Start the service
sudo systemctl start dumpster
```

## Monitoring

### Health Check

```bash
curl http://localhost:8080/
```

### Logs

```bash
# Application logs
sudo journalctl -u dumpster -f

# Nginx access logs
sudo tail -f /var/log/nginx/dumpster_access.log

# Nginx error logs
sudo tail -f /var/log/nginx/dumpster_error.log
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
sudo journalctl -u dumpster -n 50 --no-pager
```

Common issues:
- Missing environment variables in `.env`
- Database connection refused
- Permission issues on directories

### 502 Bad Gateway

The backend is not responding. Check:
1. Is the service running? `systemctl status dumpster`
2. Is it bound to the correct port? `ss -tlnp | grep 8080`
3. Check application logs for errors

### Database migration fails

Ensure the database user has sufficient privileges:
```sql
GRANT ALL PRIVILEGES ON DATABASE dumpster TO dumpster;
```

## CI/CD with GitHub Actions

The repository includes GitHub Actions workflows for continuous integration and releases.

### Workflows

| Workflow | Trigger | Description |
|----------|---------|-------------|
| `ci.yml` | Push/PR to master | Format check, Clippy, Build, Test, Frontend build |
| `release.yml` | Tag push (v*) | Build release binaries, create GitHub Release |

### CI Pipeline

On every push and pull request:
1. **Format Check** - `cargo fmt --check`
2. **Clippy** - Lint with `-D warnings`
3. **Build** - Compile all targets
4. **Test** - Run tests with PostgreSQL service container
5. **Frontend** - Build JS/CSS assets with npm

### Release Process

To create a new release:
```bash
# Tag a new version
git tag v1.0.0
git push origin v1.0.0
```

The release workflow will:
1. Build optimized release binaries
2. Build frontend assets
3. Create a tarball with binaries, templates, and migrations
4. Publish a GitHub Release with the artifact

### Downloading Releases

```bash
# Download latest release
curl -L -o dumpster.tar.gz \
  https://github.com/yourorg/dumpster/releases/latest/download/dumpster-linux-x86_64.tar.gz

# Extract to /opt/dumpster
sudo tar -xzf dumpster.tar.gz -C /opt/dumpster/
```
