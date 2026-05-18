# OpenTrApp — Social Preview Image Spec

Design brief for the GitHub social preview image (the card shown when the repo link is shared on social media, Slack, Discord, embeds, etc.).

The canonical asset is rendered by [`scripts/render-social-preview.py`](../../scripts/render-social-preview.py); this document describes the design intent so the script and the SVG mockup stay aligned.

---

## Dimensions & Format

- **Size**: 1280 × 640 px
- **Format**: PNG (final output)
- **Safe area**: keep all critical content within a 960 × 480 centered region — GitHub crops edges on mobile and embed views

---

## What This Image Must Communicate

In ~2 seconds of glance time, a viewer should pick up:

1. **Brand identity** — the gradient banner is the project's visual mark; recognising it across surfaces is the point
2. **The proposition** — "a safer way to run OpenClaw on your own computer"
3. **Authorship and source** — `albertdobmeyer` and the GitHub URL, in muted monospace

---

## Composition

### Layout: centered, single focal element

```
┌──────────────────────────────────────────────┐
│                                              │
│                                              │
│         ┌──────────────────────────┐         │
│         │  [shield] OpenTrApp  │         │
│         └──────────────────────────┘         │
│                                              │
│       A safer way to run OpenClaw on         │
│              your own computer.              │
│                                              │
│                                              │
│   albertdobmeyer  /  github.com/...          │
└──────────────────────────────────────────────┘
```

- **Banner**: the canonical gradient FontLogo, ~880 px wide, centered horizontally, vertically positioned around y ≈ 150 (i.e. slightly above the canvas midline so the tagline sits in the optical centre)
- **Glow**: a soft radial halo behind the banner, green at the green end of the gradient and blue at the blue end, blurred ~42 px — gives the card a premium, lit feel at thumbnail size
- **Tagline**: one line, sans-serif, slate-300, centered ~36 px below the banner
- **Footer**: `albertdobmeyer  /  github.com/albertdobmeyer/opentrapp` in Consolas, slate-500, ~56 px from the bottom edge

### Why this layout (and not the previous asymmetric one)

The earlier spec called for a left-weighted logo with right-aligned text and a separate golden-shield illustration. That design predates the unified FontLogo banner. The banner now carries both the shield and the wordmark in a single composition, so the card no longer needs a manual asymmetric layout — a single centered element reads cleaner at every embed size and matches how the same banner is used in the README and on the landing page.

---

## Color Palette

| Element | Token | Hex |
|---|---|---|
| Background — top-left | Slate 900 | `#0f172a` |
| Background — bottom-right | Slate 800 | `#1e293b` |
| Dot-grid texture | Slate 400 @ ~10 % | `#94a3b8` |
| Banner gradient — start | **OpenTrApp-Green** | `#009966` |
| Banner gradient — end | **OpenTrApp-Blue** | `#0EA5E9` |
| Shield (in banner only) | **OpenTrApp-Red** | `#CC3333` |
| Banner outline + wordmark | White | `#FFFFFF` |
| Tagline text | Slate 300 | `#cbd5e1` |
| Footer text | Slate 500 | `#64748b` |

The banner already carries the brand greens, blues, reds and whites. The card should not introduce additional accent colours; everything outside the banner stays in the slate scale. The brand red is reserved for the shield element inside the banner — do not use it elsewhere on the card.

The background is a 135° gradient from slate-900 to slate-800. A flat solid is not equivalent — the gradient adds the depth that makes the card hold up at small sizes.

---

## Typography

| Element | Font | Size | Weight |
|---|---|---|---|
| Tagline | Segoe UI / SF Pro / Inter (system sans) | 30 px | Regular |
| Footer | Consolas / JetBrains Mono / SF Mono (monospace) | 18 px | Regular |

The wordmark inside the banner is part of the banner image and is not re-typeset.

---

## Tagline

**Canonical:**

> A safer way to run OpenClaw on your own computer.

This is the same line used as the H1 on the landing page and in the `<title>` / `og:title` / `twitter:title` of `docs/index.html`. Keeping it identical across surfaces is intentional — the social card, the landing page, and search engine results should all reinforce the same one-line proposition.

Do not substitute alternates. If the project's positioning changes, update all three surfaces together.

---

## Visual Details

- **Banner source**: [`logos/OpenTrApp-FontLogo-Gradient.png`](../../logos/OpenTrApp-FontLogo-Gradient.png), the brand-gradient (green → blue) variant of the canonical FontLogo. Re-render via [`scripts/render-gradient-banner.py`](../../scripts/render-gradient-banner.py) if the source ever changes.
- **Background texture**: a faint dot grid at 32 px spacing, ~10 % alpha. Subtle — it should be barely perceptible on a phone, not a feature in its own right.
- **Glow construction**: take the banner's alpha as a mask, paint it with the same green-→-blue gradient, blur heavily, lay it under the banner. This produces a halo that visually extends the banner's brand colours into the slate.
- **No drop shadows under text**: the slate background gives enough contrast on its own; shadows muddy small-size renders.
- **No screenshots**: the desktop UI is still maturing; the card should reference the brand mark, not transient interface state.

---

## What NOT to Include

- Component or submodule names (Vault, Forge, Pioneer) — too much information for a card
- Version numbers or release codenames — go stale immediately
- GitHub stars, build badges, or other shields — already on the repo page
- QR codes, secondary URLs, or call-to-action buttons
- The deprecated golden-shield-with-claws illustration — superseded by the FontLogo banner
- More than one line of tagline text below the banner

---

## Relationship to Other Previews

| Repo | Background | Style |
|---|---|---|
| openclaw-vault | Red | Centered text, solid bg |
| clawhub-forge | Blue | Centered text, solid bg |
| moltbook-pioneer | Purple | Centered text, solid bg |
| **opentrapp** | **Slate gradient + brand glow** | **Centered banner + tagline** |

The parent repo intentionally breaks the pattern of the submodules. The slate gradient and the gradient banner signal hierarchy — this is the orchestrator and the brand surface, not a component preview.

---

## Build & Deliver

The card is rendered, not hand-edited.

```bash
python scripts/render-social-preview.py
# wrote docs/social-preview/opentrapp.png (1280x640)
```

The script is idempotent and reads from the canonical gradient banner; re-run whenever the banner or tagline changes. The companion `opentrapp.svg` mirrors the design as a vector source so it can be opened in design tools, but the PNG is the asset GitHub serves.

**Upload path**: GitHub repo → Settings → General → Social preview → Edit → upload `docs/social-preview/opentrapp.png`.
