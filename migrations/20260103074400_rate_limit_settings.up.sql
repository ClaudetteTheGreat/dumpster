-- Rate limit settings (category: rate_limits)
-- Format: rate_limit.{action}.max_requests and rate_limit.{action}.window_seconds

INSERT INTO settings (key, value, value_type, description, category, is_public) VALUES
-- Authentication (individual - critical)
('rate_limit.login.max_requests', '5', 'int', 'Maximum login attempts per window', 'rate_limits', FALSE),
('rate_limit.login.window_seconds', '300', 'int', 'Login rate limit window in seconds (5 min)', 'rate_limits', FALSE),
('rate_limit.two_factor.max_requests', '5', 'int', 'Maximum 2FA attempts per window', 'rate_limits', FALSE),
('rate_limit.two_factor.window_seconds', '300', 'int', '2FA rate limit window in seconds (5 min)', 'rate_limits', FALSE),
('rate_limit.password_reset.max_requests', '3', 'int', 'Maximum password reset requests per window', 'rate_limits', FALSE),
('rate_limit.password_reset.window_seconds', '3600', 'int', 'Password reset window in seconds (1 hour)', 'rate_limits', FALSE),
('rate_limit.email_verification.max_requests', '3', 'int', 'Maximum email verification resends per window', 'rate_limits', FALSE),
('rate_limit.email_verification.window_seconds', '3600', 'int', 'Email verification window in seconds (1 hour)', 'rate_limits', FALSE),

-- Account creation (individual)
('rate_limit.registration.max_requests', '3', 'int', 'Maximum registrations per window', 'rate_limits', FALSE),
('rate_limit.registration.window_seconds', '3600', 'int', 'Registration window in seconds (1 hour)', 'rate_limits', FALSE),

-- Content creation (grouped)
('rate_limit.post_creation.max_requests', '10', 'int', 'Maximum posts/profile posts/messages per window', 'rate_limits', FALSE),
('rate_limit.post_creation.window_seconds', '60', 'int', 'Post creation window in seconds (1 min)', 'rate_limits', FALSE),
('rate_limit.thread_creation.max_requests', '5', 'int', 'Maximum threads per window', 'rate_limits', FALSE),
('rate_limit.thread_creation.window_seconds', '300', 'int', 'Thread creation window in seconds (5 min)', 'rate_limits', FALSE),

-- Search & API (grouped)
('rate_limit.search.max_requests', '30', 'int', 'Maximum search queries per window', 'rate_limits', FALSE),
('rate_limit.search.window_seconds', '60', 'int', 'Search window in seconds (1 min)', 'rate_limits', FALSE),
('rate_limit.api.max_requests', '60', 'int', 'Maximum API requests (user search, unfurl) per window', 'rate_limits', FALSE),
('rate_limit.api.window_seconds', '60', 'int', 'API request window in seconds (1 min)', 'rate_limits', FALSE),

-- File uploads (individual)
('rate_limit.file_upload.max_requests', '20', 'int', 'Maximum file uploads per window', 'rate_limits', FALSE),
('rate_limit.file_upload.window_seconds', '60', 'int', 'File upload window in seconds (1 min)', 'rate_limits', FALSE),

-- Reports (individual)
('rate_limit.report.max_requests', '5', 'int', 'Maximum reports per window', 'rate_limits', FALSE),
('rate_limit.report.window_seconds', '300', 'int', 'Report window in seconds (5 min)', 'rate_limits', FALSE),

-- Reactions (individual)
('rate_limit.reaction.max_requests', '30', 'int', 'Maximum reaction toggles per window', 'rate_limits', FALSE),
('rate_limit.reaction.window_seconds', '60', 'int', 'Reaction window in seconds (1 min)', 'rate_limits', FALSE)

ON CONFLICT (key) DO NOTHING;
