-- IP bans for blocking specific IP addresses or ranges
CREATE TABLE ip_bans (
    id SERIAL PRIMARY KEY,
    ip_address INET NOT NULL,
    banned_by INT REFERENCES users(id) ON DELETE SET NULL,
    reason TEXT NOT NULL,
    expires_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    is_permanent BOOLEAN DEFAULT FALSE,
    -- Optional: ban entire subnet (e.g., /24 for IPv4)
    is_range_ban BOOLEAN DEFAULT FALSE,
    UNIQUE(ip_address)
);

-- Index for fast IP lookups
CREATE INDEX idx_ip_bans_ip ON ip_bans USING GIST (ip_address inet_ops);
-- Index for expiration checks
CREATE INDEX idx_ip_bans_expires ON ip_bans(expires_at) WHERE expires_at IS NOT NULL;
-- Index for active bans (permanent or not yet expired)
CREATE INDEX idx_ip_bans_active ON ip_bans(is_permanent) WHERE is_permanent = TRUE;
