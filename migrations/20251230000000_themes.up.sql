-- Create themes table for database-backed theming
CREATE TABLE themes (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    is_system BOOLEAN NOT NULL DEFAULT FALSE,
    is_dark BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    display_order INT NOT NULL DEFAULT 0,
    css_variables TEXT,
    css_custom TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by INT REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_themes_slug ON themes(slug);
CREATE INDEX idx_themes_active ON themes(is_active) WHERE is_active = TRUE;
CREATE INDEX idx_themes_display_order ON themes(display_order);

-- Seed system themes
INSERT INTO themes (slug, name, description, is_system, is_dark, display_order) VALUES
('light', 'Light', 'Default light theme', TRUE, FALSE, 0),
('dark', 'Dark', 'Dark theme with gray backgrounds', TRUE, TRUE, 1),
('oled-dark', 'OLED Dark', 'Pure black backgrounds for OLED displays', TRUE, TRUE, 2);

-- OLED Dark CSS variables (pure black backgrounds)
UPDATE themes SET css_variables =
'--background-color: #000000;
--bg-primary: #000000;
--bg-secondary: #0a0a0a;
--bg-tertiary: #111111;
--bg-hover: #1a1a1a;
--bg-active: #1f1f1f;
--bg-header: #000000;
--bg-code: #0a0a0a;
--bg-modal: #050505;
--bg-input: #0a0a0a;
--bg-video: #000000;
--border-primary: #1a1a1a;
--border-secondary: #222222;
--border-tertiary: #2a2a2a;
--border-color: #1a1a1a;
--nav-background: #000000;
--code-bg: #0a0a0a;
--code-border: #1a1a1a;
--code-header-bg: #050505;
--inline-code-bg: #0a0a0a;
--inline-code-border: #1a1a1a;
--table-header-bg: #0a0a0a;
--table-border: #1a1a1a;
--table-hover-bg: #111111;
--reaction-bg: #111111;
--reaction-border: #1a1a1a;
--reaction-hover-bg: #1a1a1a;
--reaction-picker-bg: #0a0a0a;
--mq-indicator-bg: #000000;
--btn-secondary-bg: transparent;
--btn-secondary-border: #333;
--btn-secondary-hover-bg: #1a1a1a;'
WHERE slug = 'oled-dark';
