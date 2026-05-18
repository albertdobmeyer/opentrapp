# Visual Assets Plan

> **Brand Identity section is superseded (2026-05-04).** The "logo-mark / logo-mark-mono / logo-wordmark" set this document plans was retired before it shipped. The canonical brand mark is now the **FontLogo banner** family in [`logos/`](../../../logos/), with the green→blue gradient variant at `logos/OpenTrApp-FontLogo-Gradient.png` deployed across the README, landing page, and social preview. The deprecated golden-shield-with-claws design referenced in earlier drafts is retired entirely. See [`memory/brand_colors.md`](../../../../) and [`scripts/render-gradient-banner.py`](../../../scripts/render-gradient-banner.py) for the current brand pipeline. Sections B–D below (illustrations, status, empty states) are still in scope.

**Prerequisite reading:** `01-vision-and-personas.md`, `02-design-system.md`
**Purpose:** Inventory all visual assets needed for the rebuild. Sourcing strategy, licensing, file organization.

---

## Why Visual Assets Matter

Karen is a visual thinker. A friendly illustration next to a success message conveys "everything is good" faster than a sentence. A concerning but calm illustration on an error screen tells her the app understands her situation. Screenshots showing what her assistant just did are more informative than any paragraph.

**For user mode, every screen should have at least one custom visual.** Dev mode uses flat icons (Lucide) for speed and density.

**No stock corporate illustrations** (disembodied floating laptops, smiling office workers). The style is warm, slightly whimsical — pictographic, not realistic — using the brand palette.

---

## Sourcing Strategy

### Tier 1: Free, customizable (default choice)

- **[unDraw](https://undraw.co)** — free, MIT-license SVG illustrations with single-color customization. Match brand color `#ff6b35`.
  - Great for: onboarding illustrations, empty states, feature explainers.
  - Style: flat, modern, consistent.

- **[Storyset](https://storyset.com)** — free tier with attribution (or paid for no-attribution). More animated/character-driven.
  - Great for: celebratory moments, personality-heavy screens (welcome, complete).
  - Customize color and style online, export SVG.

### Tier 2: Icons

- **[Heroicons](https://heroicons.com)** — MIT-licensed, outline/solid variants. React component library.
  - Great for: primary UI icons in user mode (nav, CTAs, status badges).
  - Style: simple, stroked, 24x24.

- **[Lucide](https://lucide.dev)** — already installed via `lucide-react`. Keep for dev mode.
  - Great for: dense dev mode icons.
  - Style: flat, minimal, 16x16.

### Tier 3: Custom SVG

- **Brand assets**: OpenTrApp logo variations, custom shield illustrations for security moments, tray icons.
- **Source**: create via Figma or directly in SVG. Commissioned work (Fiverr, dribbble) as fallback.

### Tier 4: Real screenshots

- **Where**: onboarding guides ("how to create a Telegram bot"), help FAQ answers.
- **How**: capture annotated screenshots with arrows/highlights, save as PNG/WebP.
- **Store**: `app/public/help-screenshots/`.

---

## Asset Inventory (By Category)

### A. Brand Identity

| Asset | Purpose | Size | Format | Source |
|-------|---------|------|--------|--------|
| `logo-mark.svg` | Icon-only logo mark | 256×256 | SVG | Custom (exists) |
| `logo-mark-mono.svg` | Monochrome variant for single-color contexts | 256×256 | SVG | Custom |
| `logo-wordmark.svg` | Logo + "OpenTrApp" text | 480×128 | SVG | Custom |
| `tray-icon-light.png` | Tray icon (macOS light mode) | 32×32 @2x | PNG | Custom |
| `tray-icon-dark.png` | Tray icon (macOS dark mode) | 32×32 @2x | PNG | Custom |
| `tray-icon-linux.png` | Tray icon (Linux) | 22×22 | PNG | Custom |
| `tray-icon-windows.ico` | Windows tray icon | 16, 32, 48 | ICO | Custom |

**Existing:** `app/public/logo.svg` (current) — replace with new marks.

### B. Onboarding Illustrations (Spec 07)

| Asset | Purpose | Notes |
|-------|---------|-------|
| `onboarding-welcome.svg` | Full-screen hero on welcome step | Friendly logo, subtle shield, warm gradient |
| `onboarding-connect.svg` | Illustrates API key + Telegram connection | Show phone + laptop bridging |
| `onboarding-installing.svg` | Animated install illustration | Concentric rings, pulsing (SVG `<animate>`) |
| `onboarding-ready.svg` | Celebration on complete step | Confetti, logo with thumbs up, phone showing Telegram chat |

**Source:** unDraw or Storyset (customize to brand palette). Estimated: 4 illustrations × 15 min customization = 1 hour.

### C. Status Illustrations (Spec 08 — Home Dashboard)

| Asset | Purpose | Used when |
|-------|---------|-----------|
| `status-safe.svg` | Large green shield with checkmark | Assistant running safely |
| `status-warning.svg` | Amber shield with exclamation | Needs attention (skill pending scan) |
| `status-error.svg` | Red shield with cross | Assistant can't start |
| `status-paused.svg` | Gray shield with pause icon | User-paused |
| `status-offline.svg` | Gray shield with WiFi-off | No API connectivity |

**Size:** 160×160 default; scalable to 240×240 for hero cards.
**Source:** Custom SVG from a common base template. Each variant = different color + icon overlay.

### D. Empty States

| Asset | Purpose | Screen |
|-------|---------|--------|
| `empty-activity.svg` | "No activity yet" | Security Monitor when new |
| `empty-alerts.svg` | "All quiet — no alerts" | Security Monitor no-alerts state |
| `empty-skills.svg` | "No skills installed yet" | Skills/discover (future) |
| `empty-favorites.svg` | "No favorites yet — add some from Discover" | Use-case gallery favorites |
| `empty-diagnostics.svg` | "Nothing to report" | Help → diagnostics |

**Size:** 200×200.
**Source:** unDraw (customized).

### E. Use-Case Gallery (Spec 12)

A grid of 10–15 illustrated cards:

| # | Use case | Category |
|---|----------|----------|
| 1 | "Plan a trip" | Everyday |
| 2 | "Morning briefing" | Everyday |
| 3 | "Remember to call mom" | Everyday |
| 4 | "Organize my desktop" | Everyday |
| 5 | "Summarize this article" | Work |
| 6 | "Draft an email" | Work |
| 7 | "Research a topic deeply" | Research |
| 8 | "Compare products" | Research |
| 9 | "Write me a poem" | Creative |
| 10 | "Brainstorm ideas" | Creative |
| 11 | "Translate a sentence" | Work |
| 12 | "Check local weather" | Everyday |
| 13 | "Plan a meal" | Everyday |
| 14 | "Track what I've been doing" | Work |
| 15 | "Random fact of the day" | Creative |

**Style:** Square SVG, 240×240, consistent illustration style (use Storyset for character-driven scenes).
**Icon variants:** Each also needs a smaller monochrome icon (32×32) for list views.

**Source:** 15 illustrations × 10 min each = 2.5 hours. Significant but doable. If time-constrained, ship with 6 and expand later.

### F. Help & FAQ Illustrations (Spec 11)

FAQ categories each get a category icon + illustration:

| Category | Illustration | Icon |
|----------|--------------|------|
| "Getting started" | Laptop with logo mark | 🏁 rocket |
| "Keys & connections" | Key + Telegram logo bridging | 🔑 key |
| "Security" | Shield with lock | 🛡️ shield |
| "Troubleshooting" | Wrench + question mark | 🔧 wrench |
| "Privacy" | Lock + eye (crossed out) | 🔒 lock |
| "Updates & billing" | Upward arrow + coin | 💳 card |

**Source:** Heroicons for icons. unDraw for illustrations. Total: 6 sets × 5 min = 30 min.

### G. Help Screenshots

Real screenshots for step-by-step guides:

| Topic | Screenshots needed |
|-------|-------------------|
| "How to create a Telegram bot" | 6-8 screenshots (BotFather flow) |
| "How to get an Anthropic API key" | 5-6 screenshots (console.anthropic.com) |
| "How to set up a spending limit" | 3-4 screenshots (console.anthropic.com billing) |
| "How to uninstall OpenTrApp" | 3 screenshots per OS × 3 OS = 9 screenshots |

**Source:** Capture manually, annotate with arrows using Figma or Excalidraw. Total: ~30 screenshots × 2 min = 1 hour.

**Format:** WebP @ 1440px wide, PNG fallback.
**Storage:** `app/public/help-screenshots/{topic}/01.webp` etc.

### H. Dev Mode Icons

All from `lucide-react` (already installed). No new assets needed. Reference in specs by Lucide name:

```tsx
import { Terminal, Shield, Network, FileCode, GitBranch, Activity } from 'lucide-react';
```

### I. Micro-animations

Small motion assets (Lottie or CSS-based):

| Asset | Purpose | Format |
|-------|---------|--------|
| Spinner (indeterminate) | Default loading | CSS keyframes |
| Progress bar (determinate) | Setup wizard steps | CSS |
| Confetti burst | Success celebrations | Canvas or SVG particles |
| Typewriter effect | Optional for "tip of the day" | CSS / JS |
| Pulse glow | Hero status indicator (subtle) | CSS @keyframes |

No Lottie JSON files for v0.2.0 (keeps bundle small). Use pure CSS/SVG.

---

## File Organization

```
app/public/
├── logo-mark.svg
├── logo-wordmark.svg
├── tray-icon-light.png
├── tray-icon-dark.png
├── tray-icon-linux.png
├── tray-icon-windows.ico
├── illustrations/
│   ├── onboarding/
│   │   ├── welcome.svg
│   │   ├── connect.svg
│   │   ├── installing.svg
│   │   └── ready.svg
│   ├── status/
│   │   ├── safe.svg
│   │   ├── warning.svg
│   │   ├── error.svg
│   │   ├── paused.svg
│   │   └── offline.svg
│   ├── empty-states/
│   │   ├── activity.svg
│   │   ├── alerts.svg
│   │   ├── skills.svg
│   │   ├── favorites.svg
│   │   └── diagnostics.svg
│   ├── use-cases/
│   │   ├── plan-a-trip.svg
│   │   ├── morning-briefing.svg
│   │   ├── ... (15 total)
│   └── help/
│       ├── getting-started.svg
│       ├── keys-connections.svg
│       ├── security.svg
│       ├── troubleshooting.svg
│       ├── privacy.svg
│       └── updates-billing.svg
└── help-screenshots/
    ├── telegram-bot/
    │   ├── 01-open-botfather.webp
    │   └── ... (8 total)
    ├── anthropic-key/
    │   └── ... (6 total)
    ├── spending-limit/
    │   └── ... (4 total)
    └── uninstall/
        ├── macos/
        ├── linux/
        └── windows/
```

Estimated total new assets: **~60 SVG illustrations + ~30 screenshots + 6 tray/logo variants**.

---

## Licensing & Attribution

| Source | License | Attribution |
|--------|---------|-------------|
| unDraw | CC0 / MIT | None required |
| Storyset (free tier) | Freepik free | Credit in About screen |
| Heroicons | MIT | None required |
| Lucide | ISC | None required |
| Custom | OpenTrApp repo license (MIT) | Own work |

Add attribution to `app/src/pages/About.tsx` (part of Settings or Help → About):

> Illustrations by [unDraw](https://undraw.co) and [Storyset](https://storyset.com). Icons by [Heroicons](https://heroicons.com) and [Lucide](https://lucide.dev). Thank you to the open source community.

---

## SVG Optimization

All SVG assets should be:

1. **Optimized with SVGO** — remove comments, metadata, unnecessary precision.
2. **Inlined when small** (<4KB) — use `<img src>` for larger, inline for micro-icons.
3. **Themeable** — use `currentColor` where possible so CSS can override fill.
4. **Accessible** — include `<title>` element for screen readers.

Build step (optional): add `svgo` to `devDependencies` and run on `prebuild`.

---

## Accessibility

- All illustrations: alt text describing the illustration's meaning, not its contents ("Illustration of your assistant running safely", not "Green shield with logo").
- Icons as buttons: always paired with `aria-label`.
- Status illustrations: paired with text (never image-only).
- Dark mode contrast: SVG fill colors must meet WCAG AA on the dark background (4.5:1 for text, 3:1 for graphics).

---

## Priority / Phasing

### MVP (must have for launch)

- [ ] Tray icons (4 variants)
- [ ] Brand logo (3 variants)
- [ ] Onboarding illustrations (4)
- [ ] Status illustrations (5)
- [ ] 3 empty-state illustrations (activity, alerts, diagnostics)
- [ ] FAQ category icons (6)
- [ ] 3 key help screenshot series (Telegram bot, Anthropic key, spending limit)

**Total MVP: ~25 assets. ~4-5 hours to source and customize.**

### Post-launch polish

- [ ] Use-case gallery illustrations (15)
- [ ] Remaining empty states (2)
- [ ] Uninstall screenshots (9)
- [ ] Lottie animations (optional, later)

---

## Implementation Notes

### React component wrapper

Create `app/src/components/Illustration.tsx`:

```tsx
interface IllustrationProps {
  name: string;          // e.g. "status/safe" or "onboarding/welcome"
  size?: number;         // default 160
  className?: string;
  alt: string;           // required for a11y
}

export default function Illustration({ name, size = 160, className, alt }: IllustrationProps) {
  return (
    <img
      src={`/illustrations/${name}.svg`}
      width={size}
      height={size}
      alt={alt}
      className={className}
      loading="lazy"
    />
  );
}
```

### Pre-load critical illustrations

Add `<link rel="preload">` in `app/index.html` for:
- `/illustrations/status/safe.svg`
- `/illustrations/onboarding/welcome.svg`

### Fallbacks

If a named illustration is missing, render a placeholder:

```tsx
<div className="w-40 h-40 rounded-lg bg-surface flex items-center justify-center">
  <span className="text-muted text-xs">Illustration</span>
</div>
```

Log a warning in console (dev mode only).

---

## Deliverable Checklist

When implementation starts, the implementer should:

- [ ] Create the `app/public/illustrations/` directory tree
- [ ] Source MVP assets from unDraw/Storyset/Heroicons
- [ ] Run all SVGs through SVGO
- [ ] Replace `app/public/logo.svg` (Vite placeholder) with new brand mark
- [ ] Create the `<Illustration>` component
- [ ] Add pre-load hints for 2 critical illustrations
- [ ] Add attribution to About screen

Post-launch, fill in the "nice to have" assets as product usage reveals gaps.

---

## Next

Read `05-automation-strategy.md` to understand what the app should auto-detect, auto-heal, and auto-configure to keep Karen from having to make decisions.
