# Design System

**Prerequisite reading:** `01-vision-and-personas.md`
**Purpose:** Codify the visual and interaction language for both modes. Color, typography, spacing, components, motion.

---

## Philosophy

The design system draws from:
- **Apple HIG** (spacious, quiet, trust-building — for user mode)
- **Material 3** (density, consistency, system-level patterns — for dev mode)

Both modes share the same **foundations** (color primitives, typography family, spacing scale) but apply them with different **defaults** (type sizes, spacing density, radii, shadows).

**Dark mode first.** Light mode is not a v0.2.0 goal. The app's aesthetic is "cozy night sky" — deep blue-blacks, warm accent colors, confident contrast.

---

## Color System

### Primitive Tokens

Defined as CSS custom properties in `app/src/styles/globals.css`, referenced by Tailwind via `theme.extend.colors` in `app/tailwind.config.js`.

```css
/* Neutrals */
--color-neutral-950: #030712;   /* bg-deep */
--color-neutral-900: #0b1120;   /* bg-base */
--color-neutral-850: #111827;   /* bg-surface */
--color-neutral-800: #1f2937;   /* bg-raised */
--color-neutral-700: #374151;   /* border */
--color-neutral-600: #4b5563;   /* border-strong */
--color-neutral-500: #6b7280;   /* text-muted */
--color-neutral-400: #9ca3af;   /* text-secondary */
--color-neutral-300: #d1d5db;   /* text-primary */
--color-neutral-200: #e5e7eb;   /* text-strong */
--color-neutral-100: #f3f4f6;   /* text-emphasis */
--color-neutral-50:  #f9fafb;   /* text-inverse */

/* Brand — Logo red-orange */
--color-primary-50:  #fff5f0;
--color-primary-200: #ffcbb0;
--color-primary-400: #ff8a5c;
--color-primary-500: #ff6b35;   /* brand */
--color-primary-600: #e55527;   /* brand-hover */
--color-primary-700: #b84520;

/* Accent — Calm blue (trust) */
--color-info-500:    #3b82f6;
--color-info-600:    #2563eb;
--color-info-400:    #60a5fa;

/* Semantic */
--color-success-500: #10b981;
--color-success-400: #34d399;
--color-warning-500: #f59e0b;
--color-warning-400: #fbbf24;
--color-danger-500:  #ef4444;
--color-danger-400:  #f87171;
```

### Semantic Token Aliases

Rather than naming colors by value, name them by **role**:

```css
/* Surfaces */
--bg-app:         var(--color-neutral-950);
--bg-base:        var(--color-neutral-900);
--bg-surface:     var(--color-neutral-850);
--bg-raised:      var(--color-neutral-800);
--bg-overlay:     rgba(3, 7, 18, 0.92);

/* Text */
--text-primary:   var(--color-neutral-200);
--text-secondary: var(--color-neutral-400);
--text-muted:     var(--color-neutral-500);
--text-inverse:   var(--color-neutral-900);

/* Borders */
--border-subtle:  var(--color-neutral-800);
--border-default: var(--color-neutral-700);
--border-strong:  var(--color-neutral-600);

/* Interactive */
--accent-primary: var(--color-primary-500);
--accent-primary-hover: var(--color-primary-600);
--accent-secondary: var(--color-info-500);
--accent-secondary-hover: var(--color-info-600);

/* Status */
--status-safe:    var(--color-success-500);
--status-warning: var(--color-warning-500);
--status-danger:  var(--color-danger-500);
--status-info:    var(--color-info-500);
```

### Mode-Specific Color Usage

- **User mode** emphasizes `--color-info-400` (soft blue) and `--color-success-500` (gentle green) with generous use of `--bg-surface` and `--bg-raised` for card elevation. Brand `--color-primary-500` appears sparingly, for hero CTAs.
- **Dev mode** emphasizes `--text-primary` on `--bg-app` (high contrast), uses `--color-info-500` for active states, and shows `--status-warning` / `--status-danger` prominently for alerts. Brand color rare.

---

## Typography

### Font Stack

```css
--font-sans: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Inter", system-ui, sans-serif;
--font-display: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Inter", system-ui, sans-serif;
--font-mono: ui-monospace, "SF Mono", "Cascadia Code", "Menlo", monospace;
```

Install `Inter` via `@fontsource-variable/inter` for Linux fallback. No external CDN fonts.

### Type Scale

```
/* Font sizes (mobile-first, scale up at md:) */
--text-xs:    0.75rem   /* 12px — captions, hints */
--text-sm:    0.875rem  /* 14px — body-small, metadata */
--text-base:  1rem      /* 16px — body */
--text-lg:    1.125rem  /* 18px — emphasized body */
--text-xl:    1.25rem   /* 20px — section headings */
--text-2xl:   1.5rem    /* 24px — page titles */
--text-3xl:   1.875rem  /* 30px — hero titles */
--text-4xl:   2.25rem   /* 36px — welcome screen */
--text-5xl:   3rem      /* 48px — celebratory moments */

/* Line heights */
--leading-tight:   1.2     /* display */
--leading-snug:    1.35    /* headings */
--leading-normal:  1.5     /* body */
--leading-relaxed: 1.65    /* prose */

/* Weights */
--font-regular:  400
--font-medium:   500
--font-semibold: 600
--font-bold:     700
```

### Mode-Specific Type Defaults

- **User mode**: body is `text-base` / `leading-relaxed`. Headings are `font-semibold`, use `--font-display`. Generous spacing.
- **Dev mode**: body is `text-sm` / `leading-normal`. Tables and data use `text-xs`. Monospace for any code/path/ID.

### Heading Hierarchy (User Mode)

| Element | Size | Weight | Example |
|---------|------|--------|---------|
| Hero (welcome, success) | 4xl | bold | "Your Assistant is Ready!" |
| Page title | 3xl | bold | "Dashboard" |
| Section title | 2xl | semibold | "Your Assistant" |
| Card title | lg | semibold | "Safe & Running" |
| Body | base | regular | "Your assistant is..." |
| Caption | sm | regular | "Last checked 2 min ago" |

### Heading Hierarchy (Dev Mode)

| Element | Size | Weight | Example |
|---------|------|--------|---------|
| Page title | xl | semibold | "openclaw-vault" |
| Panel title | base | semibold | "Commands" |
| Label | xs | medium (uppercase) | "STATUS" |
| Body | sm | regular | "Container running, exit 0" |
| Caption / code | xs | regular mono | `0.1.0 · runtime` |

---

## Spacing

```
--space-0:   0
--space-1:   0.25rem  /*  4px */
--space-2:   0.5rem   /*  8px */
--space-3:   0.75rem  /* 12px */
--space-4:   1rem     /* 16px */
--space-5:   1.25rem  /* 20px */
--space-6:   1.5rem   /* 24px */
--space-8:   2rem     /* 32px */
--space-10:  2.5rem   /* 40px */
--space-12:  3rem     /* 48px */
--space-16:  4rem     /* 64px */
--space-20:  5rem     /* 80px */
```

### Mode-Specific Spacing

- **User mode defaults**: sections use `space-8` to `space-12` between elements; cards have `space-6` internal padding; grid gutters are `space-6`.
- **Dev mode defaults**: sections use `space-3` to `space-4`; panels have `space-4` internal padding; data tables have `space-2` cell padding.

---

## Border Radius

```
--radius-none:  0
--radius-sm:    0.25rem   /* 4px — inputs, tags */
--radius-md:    0.5rem    /* 8px — buttons */
--radius-lg:    0.75rem   /* 12px — cards (dev mode) */
--radius-xl:    1rem      /* 16px — cards (user mode) */
--radius-2xl:   1.5rem    /* 24px — hero cards */
--radius-full:  9999px    /* pills, avatars */
```

User mode uses `lg` and `xl` widely. Dev mode uses `sm` and `md`.

---

## Shadows (Elevation)

```
--shadow-xs:  0 1px 2px 0 rgba(0, 0, 0, 0.2);
--shadow-sm:  0 2px 4px 0 rgba(0, 0, 0, 0.25);
--shadow-md:  0 4px 8px -2px rgba(0, 0, 0, 0.3), 0 2px 4px -2px rgba(0, 0, 0, 0.2);
--shadow-lg:  0 10px 20px -5px rgba(0, 0, 0, 0.35), 0 4px 6px -4px rgba(0, 0, 0, 0.25);
--shadow-xl:  0 20px 40px -10px rgba(0, 0, 0, 0.4), 0 8px 12px -6px rgba(0, 0, 0, 0.3);
--shadow-glow: 0 0 24px rgba(255, 107, 53, 0.15);  /* brand accent glow */
```

- **User mode**: cards use `shadow-md` or `shadow-lg`. Hero cards can use `shadow-xl` + `shadow-glow`.
- **Dev mode**: cards use `shadow-xs`. Panels are flat with `border-subtle`.

---

## Components

### Buttons

**Variants (by intent):**

```
.btn-primary      → brand accent, bold, filled (main CTA)
.btn-secondary    → info accent, filled (second CTA)
.btn-ghost        → transparent, text-primary on hover bg-surface
.btn-danger       → danger accent, filled (destructive)
.btn-link         → no background, text-accent-primary, underline on hover
```

**Sizes:**

```
.btn-sm    → text-xs, px-3 py-1.5, radius-sm
.btn-md    → text-sm, px-4 py-2, radius-md    (default)
.btn-lg    → text-base, px-6 py-3, radius-md
.btn-xl    → text-lg, px-8 py-4, radius-md    (hero CTAs)
```

**States:**
- Hover: darken 10%, cursor pointer
- Active: scale 0.98
- Disabled: opacity 0.5, cursor not-allowed
- Loading: show spinner + lock width

### Cards

```
.card            → bg-surface, border-subtle, radius-lg, padding-6
.card-raised     → bg-raised, shadow-md, radius-xl, padding-8
.card-hero       → bg-surface, shadow-xl, radius-2xl, padding-8 to padding-12, optional glow
.card-interactive → .card + hover:border-default + cursor-pointer + transition
.card-dev        → bg-surface, border-subtle, radius-md, padding-4 (dense)
```

### Status Pills

```
.pill-safe     → bg-success/10, text-success-400, border-success/20
.pill-warning  → bg-warning/10, text-warning-400, border-warning/20
.pill-danger   → bg-danger/10, text-danger-400, border-danger/20
.pill-info     → bg-info/10, text-info-400, border-info/20
.pill-neutral  → bg-raised, text-secondary, border-default
```

Always include an icon (shield, alert-triangle, info, check-circle).

### Inputs

- Default: `bg-base, border-default, radius-md, px-3 py-2, text-primary`
- Focus: `border-accent-primary, ring-2 ring-accent-primary/30, outline-none`
- Error: `border-danger, text-danger-foreground`
- Disabled: `opacity-0.5, cursor-not-allowed`

**Always pair an input with:**
1. A label above
2. A hint below (1 sentence explaining why/how)
3. An inline validation state

### Progress Indicators

- **Skeleton**: animated gradient (for loading placeholders)
- **Spinner**: rotating arc, `size-sm|md|lg`
- **Progress bar**: linear bar with `value-label` and percentage
- **Steps indicator**: for wizard (dots or bars, current highlighted)

### Empty States

Every list/grid/table has a paired empty state:

```
[illustration — 160-240px]
[title — text-xl semibold]
[subtitle — text-secondary]
[CTA button — primary action]
```

### Toasts & Notifications

Use existing `ToastContext`. Expand semantics:

- `success` → green, 4s auto-dismiss, check icon
- `info` → blue, 4s, info icon
- `warning` → amber, 6s, alert icon
- `error` → red, 0s (persistent), x-circle icon, always includes "Dismiss" button

---

## Motion

### Durations

```
--duration-fast:   150ms   /* hover, focus */
--duration-base:   250ms   /* enter, exit */
--duration-slow:   400ms   /* page transitions */
--duration-celebratory: 600ms  /* success confetti, hero reveal */
```

### Easing

```
--ease-out:     cubic-bezier(0.16, 1, 0.3, 1)   /* default — natural decel */
--ease-in-out:  cubic-bezier(0.4, 0, 0.2, 1)
--ease-spring:  cubic-bezier(0.34, 1.56, 0.64, 1)  /* bouncy — celebrations */
```

### Rules

1. **Respect `prefers-reduced-motion: reduce`.** Wrap animations in `@media (prefers-reduced-motion: no-preference)`.
2. **Entrance animations**: fade + translate Y (4–8px), duration-base, ease-out.
3. **Exit animations**: fade + translate Y (2–4px), duration-fast, ease-in-out.
4. **Hover**: subtle color/border change, duration-fast.
5. **Success moments**: spring easing + slight scale (1 → 1.05 → 1).
6. **No looping animations** outside of loading spinners — nothing should pulse or breathe infinitely.

### Celebrations

Reserve for meaningful milestones:
- Setup complete
- First Telegram message received
- Skill successfully installed
- Monthly security audit passed

Celebrations = scale animation + optional confetti (via small SVG particles) + sound (system notification sound, if enabled in preferences).

---

## Iconography

### User mode

- **Primary icons**: custom SVG illustrations (see `04-visual-assets-plan.md`) — 48×48, 64×64, 96×96
- **Secondary icons**: Heroicons (outline for inactive, solid for active) — 20×20, 24×24
- **Chart-like icons**: not used (no charts in user mode)

### Dev mode

- **All icons**: `lucide-react` — 14×14, 16×16, 20×20
- Consistent stroke width: 2
- Icon + label for navigation; icon-only for action buttons (with tooltips)

---

## Tailwind Implementation

Update `app/tailwind.config.js`:

```js
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        // Primitives (keyed by semantic name)
        neutral: {
          50: "#f9fafb", 100: "#f3f4f6", 200: "#e5e7eb",
          300: "#d1d5db", 400: "#9ca3af", 500: "#6b7280",
          600: "#4b5563", 700: "#374151", 800: "#1f2937",
          850: "#111827", 900: "#0b1120", 950: "#030712",
        },
        primary: {
          50: "#fff5f0", 200: "#ffcbb0", 400: "#ff8a5c",
          500: "#ff6b35", 600: "#e55527", 700: "#b84520",
        },
        // info, success, warning, danger... (same pattern)
      },
      fontFamily: {
        sans: ['-apple-system', 'BlinkMacSystemFont', '"SF Pro Text"', 'Inter', 'system-ui', 'sans-serif'],
        display: ['-apple-system', 'BlinkMacSystemFont', '"SF Pro Display"', 'Inter', 'system-ui', 'sans-serif'],
        mono: ['ui-monospace', '"SF Mono"', '"Cascadia Code"', 'Menlo', 'monospace'],
      },
      spacing: { '18': '4.5rem', '22': '5.5rem' },
      borderRadius: {
        '2xl': '1.5rem',
      },
      boxShadow: {
        xs: '0 1px 2px 0 rgba(0, 0, 0, 0.2)',
        glow: '0 0 24px rgba(255, 107, 53, 0.15)',
      },
      animation: {
        'fade-in': 'fadeIn 250ms cubic-bezier(0.16, 1, 0.3, 1)',
        'slide-up': 'slideUp 250ms cubic-bezier(0.16, 1, 0.3, 1)',
        'celebrate': 'celebrate 600ms cubic-bezier(0.34, 1.56, 0.64, 1)',
      },
      keyframes: {
        fadeIn: { from: { opacity: '0' }, to: { opacity: '1' } },
        slideUp: {
          from: { opacity: '0', transform: 'translateY(8px)' },
          to: { opacity: '1', transform: 'translateY(0)' },
        },
        celebrate: {
          '0%': { transform: 'scale(0.95)', opacity: '0' },
          '50%': { transform: 'scale(1.05)', opacity: '1' },
          '100%': { transform: 'scale(1)', opacity: '1' },
        },
      },
    },
  },
}
```

---

## Vocabulary Rules

Expand the banned-terms list in `app/e2e/user-facing.spec.ts` to include everything the UX rubric caught plus:

```ts
const USER_MODE_BANNED = [
  // Existing (19 terms)
  "OpenClaw Orchestrator", "OpenClaw Vault", "ClawHub Forge", "Moltbook Pioneer",
  "MoltBook Pioneer", "container_runtime", "component.yml", "compose.yml",
  "seccomp", "MITRE ATT&CK", "proxy", "manifest", "Monorepo", "monorepo",
  "health probes", "configure components", "Checking prerequisites",
  "submodule", "Submodule",
  // Additions for this rebuild
  "exit code", "stderr", "stdout", "container", "podman", "docker",
  "API key" /* prefer "key" or "password" */,
  "port", "daemon", "endpoint", "allowlist" /* prefer "trusted sites" */,
  "seccomp", "JSON", "YAML", "env file", ".env",
  "bash", "shell", "CLI", "terminal", "IPC",
  "cargo", "rust", "tauri", "react", "vite", "npm",
  "stack trace", "error: ", "Error code",
  "component", "manifest", "schema",
];
```

**Developer mode has no banned terms** — it must be allowed to use precise technical vocabulary.

---

## Usage Rules for Implementers

1. **Never hardcode a color hex.** Use the semantic token (`bg-surface`, `text-primary`, etc.) via Tailwind classes.
2. **Use mode-aware defaults.** A `<Card>` component should accept a `mode` prop (or infer from context) and apply either user-mode or dev-mode defaults.
3. **Always pair icon + text** unless the icon is a universal symbol (close, chevron, back). Never icon-only without a tooltip.
4. **Animate only on user interaction.** Don't auto-animate on page load; use entrance animations only for new content arriving.
5. **Test with `prefers-reduced-motion`.** Simulate in DevTools; verify no animation plays.

---

## Next

Read `03-information-architecture.md` to see how these components assemble into screens and modes.
