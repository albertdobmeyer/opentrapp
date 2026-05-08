# Deploying the landing page

The marketing site at <https://lobster-trapp.com> is a static page hosted on the project's Hetzner VPS, served by nginx behind Cloudflare. The source lives in this repository at [`docs/index.html`](index.html) (and `docs/bg-hero.png`); deploys are a manual `scp` after merging changes to `main`.

This runbook covers the deploy procedure end-to-end, including verification and rollback. It is independent of [`RELEASING.md`](../RELEASING.md) — landing-page changes ship out-of-band from app releases.

## Prerequisites

- SSH access to the Hetzner VPS via the `hetzner` alias in `~/.ssh/config` (key at `~/.ssh/hetzner_linuxlaptop`).
- A clean working tree on `main` containing the change you want to publish. Deploy from `main`, never from a feature branch — the live site should always match a merged commit.

The serve path on the server is `/var/www/lobster-trapp.com/html/`. Companion documentation for the wider server (other sites, databases, nginx layout) lives at `/root/docs/` on the VPS.

## Procedure

Run all commands from the repository root.

### 1. Confirm what's about to change

```bash
LOCAL_HASH=$(sha256sum docs/index.html | awk '{print $1}')
REMOTE_HASH=$(ssh hetzner sha256sum /var/www/lobster-trapp.com/html/index.html | awk '{print $1}')
echo "local:  $LOCAL_HASH"
echo "remote: $REMOTE_HASH"
[ "$LOCAL_HASH" = "$REMOTE_HASH" ] && echo "→ already in sync" || echo "→ DIFFER (deploy needed)"
```

If the hashes match, there is nothing to deploy. If they differ, optionally diff the two for a sanity check:

```bash
ssh hetzner cat /var/www/lobster-trapp.com/html/index.html | diff - docs/index.html | head -40
```

### 2. Back up the current live file

Always back up before overwriting. Use today's date plus a one-word reason so the file name is self-explanatory months later.

```bash
DATE=$(date -u +%Y%m%d)
ssh hetzner "cp -p /var/www/lobster-trapp.com/html/index.html \
  /var/www/lobster-trapp.com/html/index.html.bak.${DATE}-pre-<reason>"
```

Replace `<reason>` with a short tag like `deeplink`, `hero-copy`, `bg-image-swap`. Inspect existing backups with `ssh hetzner ls -la /var/www/lobster-trapp.com/html/index.html.bak.*`.

### 3. Deploy

```bash
scp docs/index.html root@hetzner:/var/www/lobster-trapp.com/html/index.html
```

If the deploy includes new image assets (e.g. `bg-hero.png`, `logo-banner.png`), upload them in the same `scp` invocation:

```bash
scp docs/index.html docs/bg-hero.png root@hetzner:/var/www/lobster-trapp.com/html/
```

### 4. Verify

Run all three checks before declaring the deploy successful.

```bash
# 4a. SHA-256 match between local and the file the server is serving from.
#     This is the authoritative sync check — bypasses Cloudflare entirely.
LOCAL_HASH=$(sha256sum docs/index.html | awk '{print $1}')
REMOTE_HASH=$(ssh hetzner sha256sum /var/www/lobster-trapp.com/html/index.html | awk '{print $1}')
[ "$LOCAL_HASH" = "$REMOTE_HASH" ] && echo "✓ sync confirmed" || echo "✗ MISMATCH"

# 4b. nginx config still valid + service still active.
ssh hetzner 'nginx -t 2>&1 && systemctl is-active nginx'

# 4c. The live site (via Cloudflare) returns 200 and contains the new
#     content. Substitute your own distinguishing substring — something
#     short and unique to *this* deploy (a function name, a new copy
#     fragment, a specific URL).
HTTP_CODE=$(curl -sS -L -o /tmp/live.html -w '%{http_code}' https://lobster-trapp.com/)
echo "HTTP $HTTP_CODE"
grep -c "<your-distinguishing-substring>" /tmp/live.html
```

`grep -c` should print `1` (or higher) and `HTTP_CODE` should be `200`.

**Why no byte-level diff against the live response.** Cloudflare auto-injects a small bot-management script (`__CF$cv$params`) just before `</body>` on every response, so the live HTML is intentionally a few hundred bytes longer than what nginx serves. A `diff /tmp/live.html docs/index.html` will always show one chunk of divergence even when the deploy is correct. The local-vs-server SHA in §4a is the authoritative check; the live-site grep in §4c is the *I-can-see-my-change-on-the-internet* check. Together they cover what a byte-level diff would, without the false positive.

Cloudflare serves the page with `cf-cache-status: DYNAMIC`, which means the edge does not cache and pulls from origin on every request. **No cache purge is required** after a deploy — visitors see the new content immediately.

If you change asset filenames or directory structure, that may not hold; in that case, purge via the Cloudflare dashboard or use the API.

## Rollback

If a deploy needs to be reverted, copy a dated backup over the live file:

```bash
ssh hetzner 'cp /var/www/lobster-trapp.com/html/index.html.bak.<YYYYMMDD>-<reason> \
  /var/www/lobster-trapp.com/html/index.html'
```

Re-run the verification block from §4 (substituting an old-content grep pattern in step 4d) to confirm the rollback took.

## Notes

- **Don't `cd` into a working directory on the server**; deploys are write-once `scp` operations. The server has no checkout of this repository — it serves what was last uploaded.
- **The `staging/` directory** under `/var/www/lobster-trapp.com/html/` is separate. It is not currently wired up to a subdomain or path; if you want a preview environment, that needs a one-time nginx and DNS change.
- **The `.bak.YYYYMMDD-*` files** accumulate over time. Periodically prune them on the server (`ssh hetzner 'ls -t /var/www/lobster-trapp.com/html/index.html.bak.* | tail -n +6 | xargs -r rm'` keeps the five most recent).
- **For larger landing-page revisions** (new sections, JS additions), open a regular PR against `main`, get CI green, merge, and *then* deploy. The repo is the source of truth; the server is downstream.
