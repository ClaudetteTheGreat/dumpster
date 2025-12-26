-- Remove IP bans table
DROP INDEX IF EXISTS idx_ip_bans_active;
DROP INDEX IF EXISTS idx_ip_bans_expires;
DROP INDEX IF EXISTS idx_ip_bans_ip;
DROP TABLE IF EXISTS ip_bans;
