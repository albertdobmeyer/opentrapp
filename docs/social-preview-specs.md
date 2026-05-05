# Social Preview Image Specs

GitHub social preview images for all 4 repos in the OpenClaw ecosystem.

## Dimensions

- **Size**: 1280 x 640 pixels
- **Format**: PNG (final) or SVG (placeholder)
- **Safe area**: Keep text within 960 x 480 centered region (GitHub crops edges on some views)

## Per-Repo Specs

The parent repo (lobster-trapp) intentionally breaks the centered-text-on-solid-color pattern of the submodules — it's the orchestrator and the brand surface. The submodules use a flat per-component accent color so they read as parts of an ecosystem.

| Repo | Background | Accent | Tagline |
|------|-----------|--------|---------|
| lobster-trapp | Slate gradient `#0f172a → #1e293b` + brand glow | LobsterTrApp-Green `#009966` → LobsterTrApp-Blue `#0EA5E9` | A safer way to run OpenClaw on your own computer. |
| openclaw-vault | `#dc2626` (red-600) | `#fef2f2` (red-50) | API keys never enter the container |
| clawhub-forge | `#3b82f6` (blue-500) | `#eff6ff` (blue-50) | 87-pattern offline security scanner |
| moltbook-pioneer | `#a855f7` (purple-500) | `#faf5ff` (purple-50) | Safe reconnaissance for the Moltbook agent network |

## Layout

The submodules share a flat layout:

1. Full-bleed background color
2. Repo name in monospace font (large, centered)
3. Tagline below in regular weight
4. `albertdobmeyer` footer at bottom

The lobster-trapp card uses a different layout — see [`social-preview/lobster-trapp-spec.md`](social-preview/lobster-trapp-spec.md) for the full design brief.

## Current Status

| Repo | Status | Path |
|------|--------|------|
| lobster-trapp | **Production PNG (gradient banner)** | `docs/social-preview/lobster-trapp.png` (rendered by `scripts/render-social-preview.py`) |
| openclaw-vault | **Production PNG** | `docs/social-preview.png` (in vault repo) |
| clawhub-forge | SVG placeholder | `docs/social-preview/clawhub-forge.svg` |
| moltbook-pioneer | SVG placeholder | `docs/social-preview/moltbook-pioneer.svg` |

## Uploading

Go to each repo's Settings > General > Social preview > Edit > Upload an image.
