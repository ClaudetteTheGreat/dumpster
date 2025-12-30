# Theming Feature - Planning Document

## Executive Summary

**Status: Phase 1 COMPLETE**

Dark/light theming infrastructure is now fully consolidated. All hardcoded colors have been extracted to CSS variables, enabling easy theme customization.

## Completed Work (Phase 1)

### ✅ Fixed Selector Inconsistency
- Changed `body.style-dark` to `html.dark` in `var.scss`
- All dark mode styles now use consistent `html.dark` selector
- Commit: `268811a`

### ✅ Extracted Hardcoded Colors from dark-mode.scss
- Created 61 semantic CSS variables organized by category
- Replaced 100+ hardcoded hex values with `var()` references
- Variables organized into: backgrounds, text, borders, accents, status colors, shadows
- Commit: `e3a0391`

### ✅ Updated nav.scss for Theming
- Added `--nav-background`, `--nav-text`, `--nav-text-hover`, `--nav-hover-background`
- Navigation now responds to light/dark mode
- Commit: `fe002ce`

### ✅ Updated _variables.scss and thread.scss
- Removed unused SCSS color variables (kept only `$padding`)
- Added 40+ component-specific CSS variables to `var.scss`
- Updated `thread.scss` to use CSS variables (~50 hardcoded values eliminated)
- Commit: `efdac3e`

### ✅ Cleaned Up Redundant Overrides
- Removed 220 lines of redundant component overrides from `dark-mode.scss`
- Components now themed via centralized variables in `var.scss`
- Reduced compiled CSS by ~4KB
- Commit: `86d877d`

## Current Architecture

### CSS Variable Organization

**var.scss** (`:root` and `html.dark`):
- Base variables: `--background-color`, `--text-color`, `--border-color`
- Navigation: `--nav-background`, `--nav-text`, `--nav-hover-background`
- Buttons: `--btn-secondary-*`, `--btn-quote-*`
- Multi-quote: `--mq-indicator-*`, `--mq-insert-*`, `--mq-clear-*`
- Code blocks: `--code-bg`, `--code-border`, `--inline-code-*`
- Mentions: `--mention-text`, `--mention-bg`
- Tables: `--table-border`, `--table-header-bg`, `--table-hover-bg`
- Reactions: `--reaction-bg`, `--reaction-border`, `--reaction-picker-*`
- Signature: `--signature-text`

**dark-mode.scss** (`html.dark`):
- Color palette variables (61 total)
- Base/layout styles not covered by component variables
- Forms, blockquotes, messages, modals, toolbar, etc.

### File Summary

| File | Purpose | Lines |
|------|---------|-------|
| `var.scss` | Light/dark CSS variables for components | ~175 |
| `dark-mode.scss` | Dark mode palette + base styles | ~700 |
| `nav.scss` | Navigation using CSS variables | ~55 |
| `thread.scss` | Thread/post styles using CSS variables | ~545 |
| `_variables.scss` | SCSS spacing variables only | 5 |

## Phase 2: Expand Theme Options (Optional)

If there's demand for custom themes, the infrastructure is now ready:

### Option A: Add Built-in Themes
- High contrast
- Sepia/warm
- OLED dark (pure black)

Simply create additional variable sets in `var.scss`:
```scss
html.high-contrast {
    --bg-primary: #000;
    --text-primary: #fff;
    // ...
}
```

### Option B: Database-Backed Themes
- `themes` table with color definitions
- Admin UI to create/edit themes
- User preference references theme ID

### Files to Modify (Phase 2)

| File | Changes |
|------|---------|
| `migrations/YYYYMMDD_themes.up.sql` | Create themes table |
| `src/orm/themes.rs` | Theme entity model |
| `src/web/admin.rs` | Theme CRUD endpoints |
| `templates/admin/themes.html` | Theme management UI |
| `src/orm/users.rs` | Change theme column to reference themes |
| `templates/container/public.html` | Load theme colors dynamically |

## CSS Variable Reference

### Defined in var.scss (both light and dark)

```scss
// Base
--background-color, --border-color, --text-color, --text-muted
--input-background, --scrollbar-thumb

// Navigation
--nav-background, --nav-text, --nav-text-hover, --nav-hover-background

// Buttons
--btn-secondary-bg, --btn-secondary-border, --btn-secondary-text
--btn-secondary-hover-bg, --btn-secondary-hover-border, --btn-secondary-hover-text
--btn-quote-hover-bg, --btn-quote-hover-border, --btn-quote-hover-text
--btn-quote-selected-bg, --btn-quote-selected-border

// Multi-quote
--mq-indicator-bg, --mq-indicator-text
--mq-insert-bg, --mq-insert-hover-bg
--mq-clear-bg, --mq-clear-hover-bg

// Code
--code-bg, --code-border, --code-header-bg, --code-text-muted
--code-copy-hover-bg, --code-copy-success
--inline-code-bg, --inline-code-border

// Mentions
--mention-text, --mention-bg, --mention-hover-bg

// Tables
--table-border, --table-header-bg, --table-hover-bg

// Reactions
--reaction-bg, --reaction-border, --reaction-hover-bg, --reaction-hover-border
--reaction-active-bg, --reaction-active-border, --reaction-count-text
--reaction-picker-bg, --reaction-picker-shadow
--reaction-option-hover-bg, --reaction-option-active-bg, --reaction-option-active-border

// Other
--video-bg, --signature-text
```

### Defined in dark-mode.scss (dark mode only)

```scss
// Background hierarchy
--bg-primary, --bg-secondary, --bg-tertiary, --bg-hover, --bg-active
--bg-header, --bg-code, --bg-modal, --bg-input, --bg-video

// Text hierarchy
--text-primary, --text-secondary, --text-muted, --text-hint, --text-faint
--text-light, --text-white

// Border hierarchy
--border-primary, --border-secondary, --border-tertiary, --border-light

// Accents
--link-color, --link-hover
--accent-selected-bg, --accent-selected-border, --accent-active-bg
--accent-focus, --accent-focus-dark, --accent-mention, --accent-mention-bg

// Status colors
--success-text, --success-bg, --success-border, --success-btn, --success-btn-hover
--danger-text, --danger-bg, --danger-border, --danger-btn, --danger-accent
--warning-text, --warning-bg, --warning-border
--notification-badge-bg

// Shadows
--shadow-dropdown, --shadow-heavy, --shadow-light, --shadow-focus
--overlay-light, --overlay-medium
```

## Database Schema (Phase 2)

If custom themes are needed:

```sql
CREATE TABLE themes (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    description TEXT,
    is_default BOOLEAN DEFAULT FALSE,
    is_dark BOOLEAN DEFAULT FALSE,
    colors JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- colors JSONB example:
-- {
--   "bg-primary": "#ffffff",
--   "bg-secondary": "#f5f5f5",
--   "text-primary": "#333333",
--   ...
-- }
```

## Testing Considerations

- ✅ Theme switching works without page reload
- ✅ Auto mode responds to system preference changes
- Test CSS variable fallbacks for older browsers
- Test high contrast ratios for accessibility (WCAG AA minimum)
- Test all UI components in each theme
