-- Unfurl cache for URL metadata (title, description, image, etc.)
CREATE TABLE unfurl_cache (
    id SERIAL PRIMARY KEY,
    url_hash VARCHAR(64) NOT NULL UNIQUE,  -- SHA256 hash of URL for faster lookups
    url TEXT NOT NULL,
    title TEXT,
    description TEXT,
    image_url TEXT,
    site_name VARCHAR(255),
    favicon_url TEXT,
    fetched_at TIMESTAMP NOT NULL DEFAULT NOW(),
    error_message TEXT,  -- NULL if successful, error message if failed
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Index for fast lookups by URL hash
CREATE INDEX idx_unfurl_cache_url_hash ON unfurl_cache(url_hash);

-- Index for cleanup of old entries
CREATE INDEX idx_unfurl_cache_fetched_at ON unfurl_cache(fetched_at);
