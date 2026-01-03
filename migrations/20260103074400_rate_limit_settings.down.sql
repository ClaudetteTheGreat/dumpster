-- Remove rate limit settings
DELETE FROM settings WHERE key LIKE 'rate_limit.%';
