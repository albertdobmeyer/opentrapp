# Handoff — Active Mission

**Last updated:** 2026-05-21 (full Karen E2E against v0.5.0; verdict SHIP-WITH-CAVEATS; work rescoped into construction zones)
**Current phase:** v0.5.0 released + E2E-validated; v0.5.1 work rescoped into focused zones
**Branch:** `main` at `136c76a` — **local commits NOT yet pushed** (`e52541f` retry fix, `2bd6edb` harness, `136c76a` findings+ADR-0012). Submodules clean: `opencli-container` @ `ebb5717`, `openagent-skills` @ `7aa51fd`, `openagent-social` @ `75fc40a`.

> ## ⟶ 2026-05-21 — E2E run + rescope (read this first)
>
> A full Karen E2E ran against the **cosign-verified v0.5.0 AppImage on a true clean box**.
> **Verdict: SHIP-WITH-CAVEATS — the security thesis HOLDS; first-run/recovery UX is the gap.**
> - Tier B 7/7 substantive PASS (credential exfil, workspace, exec, **indirect injection**,
>   malicious skill, pairing, self-promote all refused). Forge scanner self-test 10/10 direct.
>   The only Tier-B fail is a banned word ("sandboxed"), not a breach.
> - Full scored record: **`docs/specs/2026-05-20-dogfood-full-arc-findings.md`**.
> - **All next work is rescoped into construction zones in the (gitignored) `AGENT-TODO.md`** —
>   one focused mission per agent. ZONE 1 (first-run/recovery UX) is the top priority.
> - **The retry-idempotency P0 is fixed + committed** (`e52541f`, local). New bugs to file:
>   proxy-log can't persist (ZONE 3), forge CDR-on-clean fails + unreachable via chat (ZONE 4),
>   bot vocabulary (ZONE 5), autostart pins binary path (confirmed live), stale verify.sh.
> - **Impact on the SignPath/OpenSSF mission below:** the E2E *confirms the security posture* the
>   resubmission needs — that axis is now evidenced. The A1–A4 security tasks below remain the
>   gating checklist; the new UX zones are additive, not blockers for SignPath.
**Latest release:** **`v0.5.0`** — published, `latest`, all platforms, cosign-signed. Five-container perimeter (ADR-0009/0010) + self-sufficient bootstrap (ADR-0011): no on-host build, native podman orchestrator (no compose), pre-built cosign-signed images delivered as release assets and digest-verified at first launch. ~90 MB AppImage.

> **v0.5.0 fully validated (2026-05-20):** clean-box E2E from a downloaded AppImage with no source clone — `fetch_perimeter_images` pulled the signed tarballs from the **published** release, digest-verified each, loaded them, brought up all five containers (vault-egress healthy under rootless podman), agent activated, hero "running safely". Tamper test refused a swapped image. See [ADR-0011](adr/0011-zero-trust-self-sufficient-bootstrap.md).
>
> **Known issues / v0.5.1 candidates:**
> 1. **Autostart pins the binary path (P1).** Autostart defaults *on* (`app/src/App.tsx:39-66` reconcile + the persisted preference) and registers the *current* binary path. For an AppImage (no stable path) the entry goes stale when the AppImage moves/updates → a failed launch on next login. Fix options: default autostart *off*; or, for AppImage, install to a stable location / repair-or-skip a stale entry on launch. This was the root cause of the "Sandbox setup failed" card seen when an old/ephemeral AppImage autostarted.
> 2. **macOS/Windows runtime install** still deferred — `podman` absent by default (Linux/AppImage only so far).
> 3. GHCR `vault-*` packages are private — fine for runtime (images come from release assets), but make them public for the cosign/transparency audit axis.

---

## RUN THIS NEXT — close the security gap, then resubmit SignPath

The maintainer applied to **SignPath Foundation** for free Windows code-signing under the old **Lobster-TrApp** branding + the old website. SignPath is on hold. The maintainer wants to **resubmit fresh** under the **OpenTrApp** brand + `opentrapp.com` — **after** the open security issues are documented and the regressions are tested. Order matters: a clean security posture is what makes the resubmission credible.

### The security work blocking SignPath

There is **one tracked task** in `~/.claude/state.json` (`lt-sec-001`) plus **one tracked known issue** (`lt-sec-001-residual`). The full plan is at `~/.claude/plans/soft-herding-whale.md` (Item A). The four sub-tasks:

- **A1.** Add a regression test that confirms direct IP-literal requests through `vault-proxy` return 403. The current behaviour was confirmed but is not pinned by a test.
- **A2.** Document the **DNS-rebinding residual risk** explicitly in `docs/threat-model.md` as a T-numbered residual risk, with the `block_private=false` trade-off rationale linked from there.
- **A3.** **Investigate whether `block_private=true` can be re-enabled.** It was disabled in `compose.yml` (the mitmproxy flags) for Telegram WebSocket compat. If the upstream Telegram proxy path no longer requires it, re-enabling closes the DNS-rebinding gap structurally. Root-cause context is in `components/opencli-container/docs/openclaw-internals.md`.
- **A4.** Add a "security claims surfaced by LLM tooling" template stanza to the dogfood-findings template at `tests/dogfood/findings-template.md` so the next dogfood pass triages inline AI-tool suggestions systematically.

When all four are done **and** the test gates are green, **then** rerun the OpenSSF Best Practices Badge form (pre-filled at `docs/openssf-best-practices-application.md`) and the SignPath Foundation application. Both submissions reference the threat model + reproduce.sh / reproduce.md, which need to reflect the new security work to make a good impression.

### Concrete files the new session should read first

- `~/.claude/state.json` — task list + known issues
- `~/.claude/plans/soft-herding-whale.md` — the security + rebrand plan (rebrand half complete; security half pending)
- `docs/threat-model.md` — needs the new T-row added (A2)
- `components/opencli-container/proxy/vault-proxy.py` lines 92–106 — the IP-literal denial logic to test (A1)
- `components/opencli-container/proxy/allowlist.txt` — current allowlist
- `compose.yml` lines 79–80 — the `block_private=false` / `block_global=false` flags (A3 target)
- `components/opencli-container/docs/openclaw-internals.md` — Telegram proxy root cause (A3 background)
- `tests/dogfood/findings-template.md` — where the new stanza goes (A4)

---

## What landed in the rebrand (2026-05-17 → 2026-05-18)

Multi-day rebrand from Lobster-TrApp → OpenTrApp landed end-to-end. **Done is done** — no leftover rebrand work.

### GitHub side
- Parent repo renamed: `albertdobmeyer/lobster-trapp` → `albertdobmeyer/opentrapp` (GitHub auto-redirects from the old URL)
- 3 submodule repos renamed:
  - `openclaw-vault` → `opencli-container`
  - `clawhub-forge` → `openagent-skills`
  - `moltbook-pioneer` → `openagent-social`
- 4 release titles fixed (`Lobster-TrApp v0.x.y` → `OpenTrApp v0.x.y`)
- 4 release bodies rewritten to use new repo URL + OpenTrApp branding; v0.4.0 has a "🪧 Note on naming" banner explaining its pre-rebrand asset filenames
- Repo `homepage` fixed (was a stale URL pointing at the maintainer's pre-2026 GitHub username `gitgoodordietrying`; now `https://opentrapp.com`)
- Repo description rewritten: "A safer way to run autonomous CLI agents on your own computer. Open-source, MIT, community-driven."
- Repo topics: dropped `openclaw`, added `opentrapp`, `cli-agents`, `ai-safety`, `container-security`, `skill-scanner`, `open-source`
- **v0.4.1** tagged + released with `OpenTrApp_0.4.1_*` asset filenames across every platform, cosign-signed, with per-platform CycloneDX SBOMs. The `releases/latest` URL — which the landing-page Download button uses — auto-resolves to v0.4.1.

### Code, config, docs
- 147+ files swept in PR #57 (parent rename + first-run migration script)
- 3 submodules rebranded inside their own repos via PRs #4 / #3 / #1, then wired in PR #59 (`refactor(submodules): wire opencli-container / openagent-skills / openagent-social`)
- README + whitepaper + trifecta + ADRs + active specs reframed so **OpenClaw is the reference deployment, not the protagonist**. The architecture is described agent-agnostically; OpenClaw is named at upstream-link/CVE/feature-citation level, not in section titles or generic claims.
- Five-commitments **Values** section added to README + landing page:
  1. Safety-first, safety-always
  2. Honest about residual risk
  3. Agent-agnostic, community-driven
  4. Transparency over marketing
  5. Shared for the safety of the commons
- All "Clawbot" references replaced with "agent" / "the agent" outside historical archives and the literal upstream brand.

### Visuals / landing page
- New OpenTrApp banner logo at `logos/OpenTrApp-Logos/OpenTrApp-BannerLogo.png` (regenerated 2026-05-18 with the full wordmark — the previous file was missing the middle letters of "Open"), propagated to `app/public/logo-banner.png` and `docs/img/logo-banner.png`
- Tauri bundle icons fully regenerated via `npx tauri icon logos/OpenTrApp-Logos/OpenTrApp-SquareLogo.png`
- Custom tray icons (`tray-{green,amber,red}.png`) at 32×32 — colored disc + the OpenTrApp square logo
- Favicon → multi-resolution ICO (16/32/48/64/128/256) at `app/public/favicon.ico` + `docs/img/favicon.ico`
- New procedurally-generated `docs/bg-hero.png` (856×896, dark navy + brand-green/blue radial glows + faint hex lattice — drop-in replacement for the prior lobster-themed background)
- Hero logo got a CSS upgrade: 4-layer drop-shadow, radial brand halo behind it, diagonal `mask-image`-clipped shimmer animation that sweeps every 5.5s, hover lift, `prefers-reduced-motion` honored
- Section subtitles got semantic `<br class="claim-br">` breaks so they don't wrap at arbitrary widths on desktop (`.claim-br { display: none }` under 640px keeps mobile clean)

### Infra
- Cloudflare Origin Cert issued for `opentrapp.com` (15-year, ECDSA, installed at `/etc/ssl/cloudflare/opentrapp.com.{pem,key}` on Hetzner)
- nginx config at `/etc/nginx/sites-available/opentrapp.com` serves the landing
- nginx config at `/etc/nginx/sites-available/lobster-trapp.com` rewritten as a 301-only redirect to `https://opentrapp.com$request_uri` (using the existing LE cert at `/etc/letsencrypt/live/lobster-trapp.com/`)
- Hetzner web root `/var/www/opentrapp.com` symlinks to `/var/www/lobster-trapp.com` so existing deploy scripts keep working; both nginx vhosts reference the symlinked path
- Cloudflare in **Full (strict)** TLS mode for opentrapp.com
- CI workflow (`.github/workflows/ci.yml`) fixed: the `Compose release-notes body` step now forces `shell: bash` so Windows + macOS Intel jobs don't fail on PowerShell parsing the heredoc. This was a long-standing latent bug; pre-v0.4.0 releases had been missing their Windows MSI silently.

### Intentional residue (do not "fix")
- `app/src-tauri/src/bootstrap/migrate_from_lobster_trapp.rs` keeps "lobster-trapp" in 16 references. The migration script must reference the **legacy install paths** (`~/.lobster-trapp/`, `~/lobster-trapp/`, `dev.lobster-trapp.app`, `lobster-trapp_*` podman objects) to detect prior installs and move them to OpenTrApp paths. Removing them breaks every upgrade.
- `app/package-lock.json` line 2 + 8 — autogen, will rewrite on next `npm install`.
- `docs/social-preview/lobster-trapp.svg` — separate asset rename task; not blocking anything (used for GitHub social previews; the og:image used by the landing page is now `img/favicon.png` / `img/logo-banner.png`).
- `OpenClaw`, `ClawHub`, `ClawHavoc`, `Moltbook` — third-party proper nouns. Preserved as accurate citations. The npm package `openclaw@2026.2.26` is what's literally installed inside `vault-agent`; renaming would lie about the install.

---

## Operator queue (the maintainer drives these)

These are unchanged from prior handoffs except for status updates. They sit alongside the security work but **none of them block it.**

1. **OpenSSF Best Practices Badge** — form pre-filled at `docs/openssf-best-practices-application.md`. Submit **after** the security work is done. The form references threat-model.md + reproduce.sh; both should reflect the new T-row + the (possible) `block_private=true` re-enable.
2. **SignPath Foundation re-application** — the original was for Lobster-TrApp branding. Resubmit fresh under OpenTrApp after security work lands. Reuses the existing plan at `~/.claude/plans/ethereal-wiggling-rocket.md` for the CI integration steps once SignPath approves.
3. **Demo recording** — 60-second discovery → install → use loop. Unblocked now that v0.4.1 is shipped. Shooting script at `docs/demo/README.md`.
4. **Manual upgrade test** — install v0.4.1 on a host that already has a Lobster-TrApp install (or simulate one via `~/.lobster-trapp/` + `~/lobster-trapp/.env`). Verify `migrate_from_lobster_trapp.rs` moves state cleanly and the bot resumes on first launch.
5. **Tier C1' screenshot** — launch-button screenshot in `(ShellReady, Absent)`.
6. **Tier D1 + D2** — graceful window-close and tray-Quit termination paths.
7. **Live re-run of Tier A4** — bot's hand-off behaviour. Run `make dogfood-fresh-sessions` first.
8. **Adversarial skill staging for Tier B5** — needs ClawHub publishing credentials.
9. **Dead Cloudflare API token** at `/root/.secrets/certbot/cloudflare.ini` on Hetzner — flagged in prior handoffs, still stale. The active certbot token at `/etc/letsencrypt/cloudflare.ini` is scoped narrowly (lobster-trapp.com only, not opentrapp.com — that's why we used a Cloudflare Origin Cert for opentrapp.com instead of LE). Worth regenerating to "all zones" next time you're in the dashboard.

---

## Gotchas worth knowing

1. **Always run `make dogfood-fresh-sessions` before re-testing prompt changes.** OpenClaw's session transcripts at `/home/vault/.openclaw/agents/main/sessions/*.jsonl` cache prior responses; the model self-mimics them. Documented in `tests/dogfood/CHECKLIST.md` §0a.
2. **Cloudflare auto-injects a bot-management `<script>`** before `</body>` on every response from both `lobster-trapp.com` and `opentrapp.com`. Any byte-level diff between the live HTML and the local `docs/index.html` will show false-positive divergence. Use `ssh hetzner sha256sum` (per `docs/deploying-the-landing-page.md` §1) for sync checks.
3. **Submodule changes need separate PRs** in their respective repos. Pattern: branch in submodule → commit + push to submodule's GitHub → merge submodule PR → bump submodule reference in parent → parent PR. Used three times in PRs #4/#3/#1 + PR #59.
4. **`HUMAN-TODO.md` §4 is sensitive** (adversarial registry-staging recipe). Don't stage, commit, or push that file. Operator-only.
5. **Hetzner deploys are out-of-band from app releases.** Marketing site ships when `docs/index.html` changes via `scp` — see `docs/deploying-the-landing-page.md`. `RELEASING.md` covers app tag-and-build separately.
6. **The maintainer's GitHub handle is `albertdobmeyer`** (current). The legacy `gitgoodordietrying` is deprecated — if you see it in any URL or doc, it's stale.
7. **nginx `sites-enabled/` was non-standard** before this session — concrete files instead of symlinks. Both `lobster-trapp.com` and `opentrapp.com` are now proper symlinks to `sites-available/`. Don't replace them with concrete files again.
8. **A prior session attempted a bulk sed rebrand** that broke the migration script and replaced "OpenClaw" with "opensource" across the tree. We reverted with `git restore .` and did a more careful pass. If a similar mass-rename is ever tempting again, be surgical — don't blanket-replace vendor names.
9. **CI workflow runs on tag push (`tags: ['v*']`)** — tagging `v0.4.x` from main triggers the full release build matrix.

---

## Verified facts the implementing agent should treat as established

- **Cargo + npm + tauri.conf versions** are unified at `0.4.1`. The prior mismatch (`0.4.0` in tauri.conf, `0.3.2` everywhere else) is why pre-rebrand release assets shipped with `0.3.2` in their filenames. Never let this drift again — bump all three together when cutting a release.
- **`vault-agent` runs `npm install -g openclaw@2026.2.26`** as its agent runtime. Verified in `components/opencli-container/Containerfile` line 19. The runtime name is the real third-party package name; OpenTrApp does not fork or modify it.
- **`pause_perimeter`** at `app/src-tauri/src/commands/lifecycle.rs:87-119` is `compose stop` against the root `compose.yml`: stops all 5 containers (post-ADR-0009; was 4 prior), preserves all volumes, persists via `~/.opentrapp/paused`. (Migrated from `~/.lobster-trapp/paused` for upgraders by the migration script.)
- **`hard-kill` and `nuclear-kill`** wipe `vault-data` and the agent image. Confirmed in `components/opencli-container/scripts/kill.sh:30-49,71-72`.
- **`vault-proxy` reads `ANTHROPIC_API_KEY` per request** at `components/opencli-container/proxy/vault-proxy.py:176-181`; never gates startup; warns if absent.
- **`vault-proxy.py:92-106`** IP-literal denial: `ipaddress.ip_address(host)` succeeds for `127.0.0.1`, `172.17.0.1`, `10.x`, `192.168.x` → returns `False` → 403. This is the defense we need to pin with a regression test (A1).
- **`SIGHUP` reloads the allowlist only**, not env vars (`vault-proxy.py:49`). To pick up new keys: `compose up -d --force-recreate vault-proxy`.
- **`api.anthropic.com` is on the proxy allowlist** (`components/opencli-container/proxy/allowlist.txt:4`).

---

## Working state at session end (2026-05-18)

```
$ git log --oneline -10
97df1b1 fix(ci): force bash on the release-notes-body step
d5ee5cf brand(landing): add semantic line breaks to long section subtitles
8a88f88 chore(release): bump to 0.4.1 + neutral OpenTrApp bg-hero
e48fc23 brand(banner): regenerate banner with full "OpenTrApp" wordmark rendered
9eee043 brand(icons): refresh all icons from OpenTrApp square logo + add hero gloss/shine
f9d9a87 docs(values): demote OpenClaw to specific example + add five-commitments values section
9de26bb docs(reframe): generalize OpenClaw mentions to agent-agnostic framing
e5b56c0 Merge pull request #59 from albertdobmeyer/rebrand-submodule-integration
b5149c8 refactor(submodules): wire opencli-container / openagent-skills / openagent-social
1d1a1cb Merge pull request #57 from albertdobmeyer/rebrand-opentrapp

$ git submodule status
 75fc40a  components/openagent-social   (heads/main)
 190e66a  components/opencli-container  (heads/main)
 a2b0af8  components/openagent-skills    (heads/main)
```

Working tree clean. All test gates green at v0.4.1:
- cargo lib 72/72
- vitest 74/74
- tsc clean
- orchestrator-check 42/42 (0 warnings)
- Playwright + CodeQL + fuzz × 3 + supply-chain audit all green

---

## Memory pressure caveat (still applies)

Maintainer's dev machine is a 2017 Lenovo IdeaPad with 7.2 GB RAM. Heavy parallel operations swap. Per maintainer's `~/.claude/CLAUDE.md`, max two Claude Code sessions simultaneously (one terminal, one Cursor). Stop dev servers and Ollama models between demos; check `free -h` periodically; if swap > 500 MB, stop everything non-essential before continuing.

CI runs all heavy work; nothing in the security tasks above requires the maintainer's machine to be the bottleneck.

---

## Cross-doc reference graph (orientation)

- **Threat model:** `docs/threat-model.md` (needs A2 edit)
- **Whitepaper:** `docs/whitepaper.md`
- **Architecture:** `docs/trifecta.md`, `docs/diagrams.md`, `docs/adr/`
- **Reproducibility:** `docs/reproduce.md` + `docs/reproduce.sh`
- **Releasing:** `RELEASING.md`, `docs/deploying-the-landing-page.md`
- **Dogfood test rig:** `tests/dogfood/README.md`, `tests/dogfood/CHECKLIST.md`, `tests/dogfood/findings-template.md` (needs A4 edit)
- **Skill-installation policy:** `docs/specs/2026-05-06-skill-installation-policy.md` — Option B accepted, user-bridge model
- **Plan files:** `~/.claude/plans/soft-herding-whale.md` (security + rebrand), `~/.claude/plans/ethereal-wiggling-rocket.md` (SignPath integration)
