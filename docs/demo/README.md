# Landing-page demo recording

**Document status:** Active scaffold; the recorded asset itself is queued for a future session.
**Created:** 2026-05-04
**Roadmap reference:** [`../roadmap-post-launch.md`](../roadmap-post-launch.md) §7.

This directory holds the assets for the ≤30-second demo video that will appear on the [lobster-trapp.com](https://lobster-trapp.com) landing page hero area. The video itself is not yet committed; this README is the shooting script and the conversion recipe so the next maintainer-session can produce it without re-deriving the plan.

When the assets land, this directory should contain:

- `demo.mp4` — H.264, 1280×720, ≤30 seconds, ≤8 MB target
- `demo.gif` — animated GIF fallback, ≤5 MB target
- `demo-poster.png` — single still frame for the `<video>` `poster` attribute
- (optional) `demo.webm` — VP9 fallback for browsers without H.264 (most modern browsers handle H.264, so this is a "nice to have")

The asset filenames are referenced from the `<video>` block already stubbed in [`../index.html`](../index.html); changing them requires updating that file as well.

---

## Shooting script (≤30 seconds)

The video walks the wizard end-to-end, then cuts to the first Telegram chat. Roughly four scenes; total runtime ≤30 s; no narration (the landing page provides the prose context). Captions appear as on-screen text, not voice-over.

| t (s) | Duration | Scene | What's on screen | On-screen caption |
|------:|---------:|-------|------------------|-------------------|
| 0–3 | 3 s | Wizard `Welcome` | Lobster-TrApp window opens; the `Welcome` page is visible with the *Next* button. | "Open Lobster-TrApp." |
| 3–8 | 5 s | Wizard `System Check` | Click *Next*; the `System Check` page runs through its row of checkmarks (Podman/Docker found, network reachable, .env writable, etc.) | "It checks your system." |
| 8–14 | 6 s | Wizard `Connect Your Accounts` | Click *Next*; the credentials form appears. Paste an Anthropic key (already on clipboard). Paste a Telegram bot token. Click *Save*. | "Paste two credentials." |
| 14–22 | 8 s | Wizard `Setting Up Your Assistant` | The progress block animates: `Pulling images → Starting containers → Verifying perimeter → Validating key`. All four steps tick green. The `Complete` screen briefly appears. | "It builds the perimeter." |
| 22–30 | 8 s | Phone screen with Telegram | Cut to a phone screen mock or a real phone capture. The user sends "Hello" to their bot; the bot replies. | "Talk to it from anywhere." |

**Style notes.**

- No music. The website's tone is restrained; a soundtrack would clash.
- No real credentials on screen. The Anthropic key shown is a freshly-issued throwaway key with a $1 spending cap, revoked immediately after the recording. The Telegram token shown is from a dedicated demo bot.
- The cursor is visible. Demo videos that hide the cursor read as marketing rather than software documentation.
- Captions use the same monospace font as the website's nav-logo (`Courier New`-ish system stack).
- The `Welcome` window starts at `1280×800` — the natural size of the desktop window. The recording is captured at that size, then exported at 1280×720 by cropping the OS window-chrome.

---

## Recording recipe

Recommended environment: a clean macOS or Linux machine, *not* the maintainer's daily-driver dev machine. Reason: the daily-driver machine accumulates state (test API keys in `.env`, paired Telegram counterparts, side-loaded skills) that is not appropriate for a public demo.

### Tools

| Tool | Purpose | Install |
|------|---------|---------|
| OBS Studio | Desktop capture (window-mode capture of the Lobster-TrApp window, plus Telegram on phone via screen-mirror) | `brew install --cask obs` (macOS) / `apt install obs-studio` (Ubuntu) |
| ffmpeg | MP4 → GIF conversion; cropping; trimming | `brew install ffmpeg` / `apt install ffmpeg` |
| Telegram Desktop *or* a real phone with screen-mirror | Final-scene capture | platform-native |
| imagemagick (`magick`) | Poster-frame extraction (alternative: `ffmpeg -ss`) | `brew install imagemagick` / `apt install imagemagick` |

### OBS scene setup

Two scenes (one per camera angle):

1. **Scene "wizard"** — single capture of the Lobster-TrApp window in window-capture mode (not display-capture, to avoid menu-bar leakage). Sized 1280×800. Frame the entire window with a 16-pixel margin.
2. **Scene "telegram"** — single capture of either the Telegram Desktop window or a phone-mirror feed. Same 1280×720 frame.

OBS profile settings: 30 fps, H.264 with the *Indistinguishable Quality* preset, output to `~/Movies/lobster-demo.mkv` (record as MKV; convert to MP4 in post — MKV is more recoverable if OBS is killed mid-record).

### Recording

```bash
# 1. Quit any other applications.
# 2. Start OBS, scene "wizard", begin recording.
# 3. Walk through scenes 1–4 of the shooting script (wizard).
# 4. Switch to scene "telegram", record scene 5.
# 5. Stop recording.
```

If a scene goes badly, stop and re-record from the start. Editing splices in OBS is more work than re-recording.

### Conversion

```bash
# Re-encode MKV → MP4, trim to the final cut-length, downsize to 720p:
ffmpeg -i ~/Movies/lobster-demo.mkv \
  -ss 00:00:00 -t 00:00:30 \
  -vf "scale=1280:720:flags=lanczos" \
  -c:v libx264 -preset slow -crf 22 \
  -c:a aac -b:a 128k \
  -movflags +faststart \
  docs/demo/demo.mp4

# Generate the GIF fallback (≤5 MB target):
ffmpeg -i docs/demo/demo.mp4 \
  -vf "fps=12,scale=720:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse" \
  -loop 0 \
  docs/demo/demo.gif

# Extract the poster frame (~3 seconds in, the "Open Lobster-TrApp" caption):
ffmpeg -ss 00:00:03 -i docs/demo/demo.mp4 -frames:v 1 -q:v 2 docs/demo/demo-poster.png

# Optional WebM (VP9) for browsers without H.264:
ffmpeg -i docs/demo/demo.mp4 \
  -c:v libvpx-vp9 -b:v 1M -crf 32 \
  -c:a libopus -b:a 96k \
  docs/demo/demo.webm
```

Verify the final-asset sizes after conversion:

```bash
ls -la docs/demo/demo.mp4 docs/demo/demo.gif docs/demo/demo-poster.png
```

Targets (above which the asset should be re-encoded with tighter parameters):

| Asset | Soft cap | Hard cap |
|-------|---------:|---------:|
| `demo.mp4` | 5 MB | 8 MB |
| `demo.gif` | 3 MB | 5 MB |
| `demo-poster.png` | 200 KB | 500 KB |

### Pre-publish checklist

- [ ] No real credentials visible at any frame (scrub the entire timeline)
- [ ] No real personal data (the Telegram username and the bot username are demo-only)
- [ ] Captions readable on a 13" laptop screen at 50 % zoom
- [ ] First and last frames look clean for the social-preview (Twitter / Mastodon / OG image)
- [ ] Audio track absent or silent (verify with `ffprobe demo.mp4 | grep Audio`)
- [ ] File sizes within the soft caps above
- [ ] Tested in Chrome, Firefox, and Safari (the `<video>` element renders in all three; the `<picture>` GIF fallback renders for browsers without `<video>`)

---

## Embedding in the landing page

The landing page at [`../index.html`](../index.html) already includes a stubbed `<video>` block in the hero area, currently commented out. When the assets are committed to this directory, un-comment the block and verify locally with:

```bash
# From the repository root
python3 -m http.server -d docs 8080
# Open http://localhost:8080 and verify the hero video plays
```

Deploy to Hetzner per the standard pattern documented in [`../../README.md`](../../README.md) (the pattern that pushes `docs/index.html` and the asset directories to `/var/www/lobster-trapp.com/`).

Verify the deployed assets:

```bash
curl -I https://lobster-trapp.com/demo/demo.mp4
curl -I https://lobster-trapp.com/demo/demo.gif
```

The two `curl -I` calls should return 200 OK with `Content-Type: video/mp4` and `Content-Type: image/gif` respectively.

---

## Why this is a separate session

The recording cannot be produced inside this repository's CI: it requires a clean recording environment, a fresh API key, a fresh Telegram bot, and a human to walk through the wizard and react naturally on camera. It also benefits from being recorded against a stable v0.3.x release (no in-flight changes that would mid-take rebrand the wizard).

The plan above is intentionally precise so the recording session is mechanical: pre-stage credentials, follow the shooting script, run the conversion commands, commit. Estimated session time: half a day, dominated by re-takes to get a clean recording.
