# Handoff: Landing Page + Domain Publishing

**Date:** 2026-04-09
**Context:** Phases F-J of the v4 finalization roadmap are complete. The lobster-trapp repo is public. The three component repos (openclaw-vault, clawhub-forge, moltbook-pioneer) are private.

---

## Current State

### What's Done
- **Phases F-J implemented** on `main` branch — test infrastructure, card-grid renderer, cross-platform bundles, setup wizard E2E, forge polish, security audit, README polish, landing page
- **Username migration** — all references updated from `gitgoodordietrying` to `albertdobmeyer` across all 4 repos
- **Updater ceremony** — signing keypair generated, `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` stored in GitHub Secrets
- **Test release tag** — `v0.1.0-rc.1` pushed, CI should be building release binaries
- **Landing page** — `docs/index.html` with hero image `docs/hero.png`, OS auto-detection, download funnel design
- **Domain** — `lobster-trapp.com` purchased on Cloudflare

### What's Uncommitted
Two files in `docs/` are staged but not committed:
- `docs/index.html` — updated landing page (download funnel with hero image, OS detection)
- `docs/hero.png` — isometric security architecture illustration (new file)

### What Needs To Happen

#### 1. Perfect the Landing Page
The landing page at `docs/index.html` is a single-file download funnel. The user wants to review and refine it before publishing. Current layout:

1. **Hero** — logo, "Your API keys *never* enter the container" headline, auto-detected OS download button, trust badges
2. **Hero image** — `docs/hero.png` isometric architecture visualization
3. **Feature row** — Contain / Scan / Monitor (3 cards)
4. **Footer** — author, source link, tech stack

Design decisions already made:
- Dark theme (`#020617` background) — chosen by user from 3 options
- Single-page funnel — not a marketing site
- OS auto-detection via JS `navigator.userAgent` — customizes button text and format info
- Download links all point to `https://github.com/albertdobmeyer/lobster-trapp/releases/latest`
- No links to private component repos (vault/forge/pioneer are private)
- OG meta tags set for social sharing with hero.png as og:image

Things the user may want to iterate on:
- Hero image sizing/placement
- Copy refinement
- Mobile responsiveness
- Any additional sections or removal of existing ones

#### 2. Set Up Cloudflare Pages
The user bought `lobster-trapp.com` on Cloudflare. The landing page needs to be published there.

**Recommended approach: Cloudflare Pages**
1. Go to Cloudflare dashboard → Pages → Create a project
2. Connect to GitHub repo `albertdobmeyer/lobster-trapp`
3. Build settings:
   - Build command: (none — static HTML)
   - Build output directory: `docs`
   - Root directory: `/`
4. Set custom domain: `lobster-trapp.com` and `www.lobster-trapp.com`
5. Cloudflare auto-provisions SSL

Alternative: could also use `wrangler` CLI if the user prefers command-line setup.

#### 3. Verify RC Release & Tag v0.1.0
- Check CI status for `v0.1.0-rc.1` at https://github.com/albertdobmeyer/lobster-trapp/actions
- If binaries built successfully: tag `v0.1.0` for real release
- If CI failed: diagnose, fix, tag `v0.1.0-rc.2`

```bash
# After verifying rc.1 succeeded:
git tag -a v0.1.0 -m "v0.1.0 — first public release"
git push origin v0.1.0
```

#### 4. Post-Launch
- Verify download links on lobster-trapp.com resolve to actual binaries
- Test the auto-updater endpoint URL in `tauri.conf.json` (points to GitHub releases `latest.json`)
- Consider enabling GitHub Pages as a fallback/redirect if Cloudflare Pages has issues

---

## Key Files

| Purpose | Path |
|---------|------|
| Landing page | `docs/index.html` |
| Hero image | `docs/hero.png` |
| Tauri config (updater endpoint) | `app/src-tauri/tauri.conf.json` |
| Updater ceremony script | `scripts/setup-updater.sh` |
| Private signing key (LOCAL ONLY) | `~/.tauri/lobster-trapp.key` |
| CI workflow | `.github/workflows/ci.yml` |
| Master roadmap | `docs/roadmap-v4-finalization.md` |

## Repo Visibility

| Repo | Visibility | Why |
|------|-----------|-----|
| `albertdobmeyer/lobster-trapp` | **Public** | The product — users download from here |
| `albertdobmeyer/openclaw-vault` | Private | Component submodule — not needed by end users |
| `albertdobmeyer/clawhub-forge` | Private | Component submodule — not needed by end users |
| `albertdobmeyer/moltbook-pioneer` | Private | Component submodule — not needed by end users |

The README explains that submodules are private and only needed for development. End users download installers from GitHub Releases.
