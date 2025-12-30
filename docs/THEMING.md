# Theming Feature - Planning Document

## Executive Summary

**Feasibility: EXCELLENT (85% Complete)**

Dark/light theming infrastructure already exists and works. Extending to support custom themes requires consolidating hardcoded colors into CSS variables.

## Current State

### Existing Infrastructure

#### User Preference Storage
- `users.theme` column (VARCHAR(20), DEFAULT 'light')
- Supported values: `light`, `dark`, `auto` (system preference)
- CHECK constraint enforces valid values
- Migration: `20251123000000_user_theme_preference.up.sql`

#### Account Settings UI
- Theme selector at `/account` preferences
- Options: Light, Dark, Auto (follow system)
- PATCH endpoint validates and saves preference

#### Template Integration
- Base template (`container/public.html`) sets `data-theme` attribute
- JavaScript applies `html.dark` class based on preference
- Auto mode listens for system preference changes via `matchMedia`

#### CSS Architecture
- **17 SCSS files** totaling ~4,114 lines
- **CSS Custom Properties** defined in `var.scss`:
  - `:root` - light mode defaults
  - `body.style-dark` - dark mode overrides
- **Dedicated dark mode file** (`dark-mode.scss`, 768 lines)
- Variables: `--background-color`, `--text-color`, `--border-color`, etc.

### Known Issues

| Issue | Location | Impact |
|-------|----------|--------|
| Class selector mismatch | `var.scss` uses `body.style-dark`, `dark-mode.scss` uses `html.dark` | Styles may not apply correctly |
| Hardcoded colors | `dark-mode.scss` has ~100+ hardcoded hex values | Cannot easily add new themes |
| Navigation not themed | `nav.scss` uses `#333`, `white` | Nav doesn't respond to theme |
| Minimal SCSS variables | `_variables.scss` only 6 lines | No central color palette |

## Implementation Options

### Option A: Fix Current System (Recommended First)

Consolidate the existing light/dark implementation:
- Standardize on `html.dark` selector
- Move hardcoded colors to CSS variables
- Ensure all components use variables

**Effort**: 4-6 hours
**Risk**: Low

### Option B: Admin-Defined Custom Themes

Allow admins to create custom color themes:
- Store theme definitions in database
- Generate CSS dynamically or at build time
- Users select from available themes

**Effort**: 2-3 days
**Risk**: Medium (CSS generation complexity)

### Option C: User Custom Colors

Allow users to customize individual colors:
- Color picker UI for each variable
- Store as JSON in user preferences
- Apply via inline styles or CSS variables

**Effort**: 3-4 days
**Risk**: Medium-High (performance, CSS conflicts)

## Recommended Implementation

### Phase 1: Consolidate Existing System

1. **Fix selector inconsistency**
   - Change `body.style-dark` to `html.dark` in `var.scss`
   - Or vice versa (standardize on one)

2. **Extract hardcoded colors**
   - Audit `dark-mode.scss` for hardcoded values
   - Create CSS variables for each unique color
   - Replace hardcoded values with `var(--name)`

3. **Theme navigation**
   - Update `nav.scss` to use CSS variables

### Phase 2: Expand Theme Options (Optional)

1. **Add more built-in themes**
   - High contrast
   - Sepia/warm
   - OLED dark (pure black)

2. **Database-backed themes**
   - `themes` table with color definitions
   - Admin UI to create/edit themes
   - User preference references theme ID

## Files to Modify

### Phase 1 (Consolidation)

| File | Changes |
|------|---------|
| `resources/css/var.scss` | Change `body.style-dark` to `html.dark` |
| `resources/css/dark-mode.scss` | Replace hardcoded colors with variables |
| `resources/css/nav.scss` | Use CSS variables for colors |
| `resources/css/_variables.scss` | Expand SCSS variable definitions |

### Phase 2 (Custom Themes)

| File | Changes |
|------|---------|
| `migrations/YYYYMMDD_themes.up.sql` | Create themes table |
| `src/orm/themes.rs` | Theme entity model |
| `src/web/admin.rs` | Theme CRUD endpoints |
| `templates/admin/themes.html` | Theme management UI |
| `src/orm/users.rs` | Change theme column to reference themes |
| `templates/container/public.html` | Load theme colors dynamically |

## CSS Variable Inventory

### Currently Defined (var.scss)

```scss
--background-color
--border-color
--text-color
--text-muted
--input-background
--button-text-color
```

### Needed for Full Theming

```scss
// Backgrounds
--bg-primary
--bg-secondary
--bg-tertiary
--bg-input
--bg-code
--bg-quote

// Text
--text-primary
--text-secondary
--text-muted
--text-link

// Borders
--border-primary
--border-secondary
--border-input

// Accents
--accent-primary
--accent-secondary
--accent-success
--accent-danger
--accent-warning

// Components
--btn-primary-bg
--btn-primary-text
--btn-secondary-bg
--btn-secondary-text
--nav-bg
--nav-text
--nav-hover
--modal-bg
--modal-border
--post-bg
--post-border
```

## Complexity Assessment

| Task | Effort | Risk |
|------|--------|------|
| Fix selector inconsistency | 1 hour | Very Low |
| Extract colors from dark-mode.scss | 3-4 hours | Low |
| Theme navigation | 30 min | Very Low |
| Add built-in themes | 2-3 hours each | Low |
| Database-backed themes | 1-2 days | Medium |
| User custom colors | 2-3 days | Medium-High |

## Recommendation

**Start with Phase 1** - The current dark mode works but has technical debt. Consolidating it will:

1. Make the codebase cleaner
2. Make future theme additions trivial
3. Be low risk with immediate benefits

**Phase 2 is optional** - Only implement if there's demand for custom themes. The infrastructure from Phase 1 makes it straightforward.

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

- Test theme switching without page reload
- Test auto mode with system preference changes
- Test CSS variable fallbacks for older browsers
- Test high contrast ratios for accessibility (WCAG AA minimum)
- Test all UI components in each theme
