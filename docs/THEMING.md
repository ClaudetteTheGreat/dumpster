# Theming System

## Overview

The theming system uses a **parent/child architecture** where the base (light) theme defines all design tokens, and child themes (like dark mode) only override token values. Components never contain theme-specific styles - they only reference tokens.

**Key Principle**: Dark mode works by overriding token values only. No component should have `html.dark` selectors.

## Architecture

```
resources/css/
├── tokens/
│   └── _index.scss      # All design tokens (474 lines)
├── base/
│   └── _index.scss      # Reset, typography, globals (475 lines)
├── themes/
│   └── dark.scss        # Dark theme - token overrides ONLY (265 lines)
├── components/          # Use ONLY tokens, no hardcoded colors
│   ├── _button.scss
│   ├── _input.scss
│   ├── _card.scss
│   ├── _alert.scss
│   ├── _pagination.scss
│   └── _breadcrumb.scss
├── pages/               # Use ONLY tokens, no hardcoded colors
│   ├── _forum.scss
│   ├── _thread.scss
│   └── _member.scss
└── main.scss            # Entry point with import order
```

## Design Tokens

### Token Categories

All tokens are defined in `tokens/_index.scss`:

| Category | Examples |
|----------|----------|
| **Color Primitives** | `--color-gray-50` through `--color-gray-950`, `--color-blue-*`, `--color-green-*`, etc. |
| **Surfaces** | `--surface-page`, `--surface-primary`, `--surface-secondary`, `--surface-hover`, `--surface-active` |
| **Text** | `--text-primary`, `--text-secondary`, `--text-muted`, `--text-disabled`, `--text-inverse` |
| **Borders** | `--border-default`, `--border-subtle`, `--border-strong` |
| **Accent** | `--accent-default`, `--accent-hover`, `--accent-active`, `--accent-subtle` |
| **Feedback** | `--success-*`, `--danger-*`, `--warning-*`, `--info-*` (each has default, hover, active, subtle, text, border) |
| **Inputs** | `--input-bg`, `--input-border`, `--input-border-hover`, `--input-border-focus`, `--input-placeholder` |
| **Links** | `--link-default`, `--link-hover`, `--link-visited` |
| **Typography** | `--font-sans`, `--font-mono`, `--text-xs` through `--text-4xl`, `--font-normal` through `--font-bold` |
| **Spacing** | `--space-1` through `--space-16` (4px increments) |
| **Radius** | `--radius-sm`, `--radius-md`, `--radius-lg`, `--radius-xl`, `--radius-full` |
| **Shadows** | `--shadow-xs`, `--shadow-sm`, `--shadow-md`, `--shadow-lg`, `--shadow-xl` |
| **Motion** | `--duration-fast`, `--duration-normal`, `--duration-slow`, `--ease-default`, `--ease-in`, `--ease-out` |
| **Layout** | `--container-max`, `--sidebar-width`, `--nav-height` |
| **Focus** | `--focus-ring-width`, `--focus-ring-color`, `--focus-ring-offset` |
| **Z-Index** | `--z-dropdown`, `--z-sticky`, `--z-modal`, `--z-toast` |

### Using Tokens in Components

```scss
// CORRECT - Use tokens only
.btn-primary {
    background-color: var(--accent-default);
    color: var(--text-inverse);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-4);

    &:hover {
        background-color: var(--accent-hover);
    }
}

// WRONG - Never use hardcoded colors
.btn-primary {
    background-color: #0d6efd;  // DON'T DO THIS
    color: #ffffff;              // DON'T DO THIS
}

// WRONG - Never add dark mode overrides to components
html.dark {
    .btn-primary {
        background-color: #60a5fa;  // DON'T DO THIS
    }
}
```

## Creating a New Theme

To create a new theme, create a file in `themes/` that only overrides token values:

```scss
// themes/high-contrast.scss
html.high-contrast {
    // Override color primitives
    --color-gray-50: #000000;
    --color-gray-900: #ffffff;

    // Override semantic tokens
    --surface-page: #000000;
    --surface-primary: #000000;
    --text-primary: #ffffff;
    --border-default: #ffffff;

    // Override accent colors
    --accent-default: #ffff00;
    --accent-hover: #ffff66;

    // All components automatically use these new values
}
```

Then import it in `main.scss`:

```scss
// 6. THEMES
@use 'themes/dark';
@use 'themes/high-contrast';  // Add new theme
```

## Dark Theme Implementation

The dark theme (`themes/dark.scss`) demonstrates the pattern:

```scss
html.dark {
    // Invert the gray scale
    --color-gray-50: #18181b;
    --color-gray-100: #27272a;
    // ... through --color-gray-950

    // Dark surfaces
    --surface-page: #0f0f0f;
    --surface-primary: #18181b;
    --surface-secondary: #27272a;

    // Light text on dark backgrounds
    --text-primary: #fafafa;
    --text-secondary: #d4d4d8;

    // Brighter accent for visibility
    --accent-default: #60a5fa;
    --accent-hover: #93c5fd;

    // Adjusted shadows for dark mode
    --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.3);
}
```

**Important**: The dark theme file contains NO component-specific styles - only token value overrides.

## Import Order in main.scss

```scss
// 1. DESIGN TOKENS - Must come first
@use 'tokens/index' as tokens;

// 2. BASE STYLES - Reset, typography, globals
@use 'base/index' as base;
@use 'utilities';

// 3. COMPONENTS - Reusable UI components
@use 'components/button';
@use 'components/input';
@use 'components/card';
@use 'components/alert';
@use 'components/pagination';
@use 'components/breadcrumb';

// 4. PAGE STYLES - Page-specific styles
@use 'pages/forum';
@use 'pages/thread';
@use 'pages/member';

// 5. LEGACY MODULES - Being migrated
@use 'generic';
@use 'layout';
// ...

// 6. THEMES - Token overrides LAST
@use 'themes/dark';
```

## Legacy Token Aliases

For backward compatibility, legacy token names are aliased to new semantic tokens:

```scss
// Legacy aliases (in tokens/_index.scss)
--bg-primary: var(--surface-primary);
--bg-secondary: var(--surface-secondary);
--bg-tertiary: var(--surface-tertiary);
--bg-hover: var(--surface-hover);
--link-color: var(--link-default);
--link-hover: var(--link-hover);
--success-btn: var(--success-default);
--danger-btn: var(--danger-default);
// etc.
```

These allow existing code to continue working while new code uses semantic token names.

## Migration Checklist for Components

When refactoring a component to use tokens:

1. **Replace hardcoded colors** with semantic tokens:
   - Background colors → `--surface-*`
   - Text colors → `--text-*`
   - Border colors → `--border-*`
   - Accent/brand colors → `--accent-*`
   - Status colors → `--success-*`, `--danger-*`, `--warning-*`, `--info-*`

2. **Replace hardcoded values** with tokens:
   - Spacing → `--space-*`
   - Border radius → `--radius-*`
   - Shadows → `--shadow-*`
   - Transitions → `--duration-*` and `--ease-*`

3. **Remove all `html.dark` blocks** - dark mode is handled by token overrides

4. **Test both themes** - verify component looks correct in light and dark mode

## Accessibility

- All focus states use `--focus-ring-*` tokens
- Reduced motion is respected via `prefers-reduced-motion` media query in base
- Color contrast ratios are validated at the token level
- Focus-visible is used instead of focus for keyboard navigation

## Build

```bash
npm run build  # or npx webpack
```

CSS output is in `public/assets/style.css`.
