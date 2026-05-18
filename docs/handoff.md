# Handoff — Active Mission

**Last updated:** 2026-05-13 (maintenance session — security hardening, landing page, cert renewal; v0.4 implementation not started)
**Current phase:** v0.4 reframe — spec set finalized, ready for implementation. This session was maintenance, not v0.4 code.
**Branch:** `main` at `e1e1bf3` — pushed to `origin/main`.
**Tag:** `v0.4.0` is the current shipped version (tagged and released). openclaw-vault submodule at `a5a46ad`.

---

## RUN THIS NEXT — read the spec set, then start at PR-1

The next session's job is to **implement v0.4**. The 8 specs in [`docs/specs/v0.4-shell-tenant-reframe/`](specs/v0.4-shell-tenant-reframe/) are the canonical source. Read them in order:

1. [`README.md`](specs/v0.4-shell-tenant-reframe/README.md) — vision, reading order, cross-spec invariants, implementation sequence
2. [`00-architectural-reframe.md`](specs/v0.4-shell-tenant-reframe/00-architectural-reframe.md) — the umbrella; phases, refactor/rebuild table, scoping
3. [`01-state-machine.md`](specs/v0.4-shell-tenant-reframe/01-state-machine.md) — `BootstrapState × TenantState`, marker files, watchdog refactor
4. [`02-bootstrap-service.md`](specs/v0.4-shell-tenant-reframe/02-bootstrap-service.md) — background pipeline, Podman sidecar, post-bootstrap auto-activation, single-instance plugin
5. [`03-activation-flow.md`](specs/v0.4-shell-tenant-reframe/03-activation-flow.md) — JIT modal, Anthropic ping, Telegram polling-handoff
6. [`04-stop-and-recovery-ux.md`](specs/v0.4-shell-tenant-reframe/04-stop-and-recovery-ux.md) — one-button Stop on `pause_perimeter`, recovery card, failure taxonomy
7. [`05-bot-first-message.md`](specs/v0.4-shell-tenant-reframe/05-bot-first-message.md) — tutorial in Telegram via inline keyboard (submodule PR)
8. [`06-migration.md`](specs/v0.4-shell-tenant-reframe/06-migration.md) — existing-install detection + live-ping verification
9. [`07-container-name-cleanup.md`](specs/v0.4-shell-tenant-reframe/07-container-name-cleanup.md) — small precondition PR

Then start with **PR-1: container-name cleanup** ([`07`](specs/v0.4-shell-tenant-reframe/07-container-name-cleanup.md)). It's small, low-risk, and unblocks isolated testing for everything else.

The implementation sequence is in the README; each PR has a natural review checkpoint before the next begins.

---

## What landed this session (2026-05-13)

Maintenance session — no v0.4 feature code. All changes on `main`, no feature branches.

### Security: skill clearance enforcement (openclaw-vault)

ADR-0003 claimed "agent rejects unscanned skills" — this was never actually implemented. `install-skill.sh` had a soft y/N bypass prompt. Two changes close the gap:

- **`install-skill.sh`** (`ecb3269`, `a5a46ad`): bypass prompt replaced with hard `exit 1`. A forge clearance report is now required to install any skill. Auto-detects `clearance-report.json` from the skill directory when the path is the forge export dir (which is the GUI call path — `component.yml` passes only `skill_path`). Writes a `.trust` file (`VERIFY_HASH=sha256:...`) into the container after successful install.
- **`entrypoint.sh`** (`ecb3269`): step 5.5 added before OpenClaw starts — iterates every installed skill directory, checks `.trust` exists, verifies `VERIFY_HASH` matches current `sha256sum` of `SKILL.md`. Any failure aborts container startup. Skills dropped in without going through `install-skill.sh` can never reach the agent.

Both commits pushed to openclaw-vault remote (`a5a46ad`). Submodule reference updated in opentrapp (`2c189bc`).

### Landing page

- Version pill updated: `v0.3.0` → `v0.4.0 — Shellfish Reframe release`
- SmartScreen/Gatekeeper bypass guide added as a `<details>` block in the download section
- SignPath Foundation credit line added (required by their OSS terms)
- Full audit of skill scanner claims against actual clawhub-forge source code — four locations corrected: "before it runs" → "pre-install only", `16` → `17` injection signatures, "SKILL.md only" → "every file in the bundle", added honest limitation ("pipeline is not automatic; patterns unknown to scanner can still slip through")
- **4-gate vetting pipeline SVG diagram** added to the Ecosystem section — shows the full clawhub-forge sequence (untrusted skill → Lint → Scan → Verify → Test → Clearance → Vault) with red fail rail. Rendered and verified in browser.
- Deployed to Hetzner

### Supporting docs

- `docs/code-signing-policy.md` — new file required for SignPath Foundation application
- `docs/release-notes-v0.4.0.md` — v0.4.0 release notes
- `app/src-tauri/tauri.conf.json` — version bumped `0.3.2` → `0.4.0`

### Infrastructure

- **Dependabot alerts dismissed**: glib `RUSTSEC-2024-0429` (unsoundness in `VariantStrIter`, locked by Tauri GTK3 stack, no update path) and a second transitive alert — both marked `tolerable_risk`.
- **Cloudflare Origin Certificate expired** (May 12, 17:49 UTC — the day before this session). Replaced with a Let's Encrypt certificate issued via certbot DNS challenge using the token in `/etc/letsencrypt/cloudflare.ini`. Auto-renews every 90 days (cron registered by certbot). nginx updated: `ssl_certificate/ssl_certificate_key` now point to `/etc/letsencrypt/live/opentrapp.com/{fullchain.pem,privkey.pem}`. Site verified live (HTTP 200, content confirmed).
- **SignPath Foundation application submitted** — awaiting approval email at `albertkdobmeyer@gmail.com`. When approved: set up project in SignPath dashboard, add `SIGNPATH_API_TOKEN` and `SIGNPATH_ORGANIZATION_ID` GitHub secrets, restructure the Windows CI job to use post-build signing (see plan in `~/.claude/plans/ethereal-wiggling-rocket.md`), then update `bundle.windows.certificateThumbprint` in `tauri.conf.json`.

### Architecture audit

Investigated whether the 4-container architecture was an accidental Claude drift or deliberate design. Finding: it was deliberate, documented in ADR-0006 (April 2026) driven by a real security gap. vault-pioneer is intentionally parked (Moltbook acquired by Meta, API unstable since April 2026) — kept in compose.yml for completeness, same as moltbook-pioneer submodule.

---

## What was decided this session (the product calls)

1. **Stop button: one button, not two.** "Stop your assistant" / "Resume" — both share the existing `pause_perimeter` primitive (`compose stop`, all volumes preserved). Never `nuclear-kill` or `hard-kill` — those wipe `vault-data` (session history) and `forge-deliveries` (installed skills).
2. **Container-name cleanup is its own PR**, lands first ([`07`](specs/v0.4-shell-tenant-reframe/07-container-name-cleanup.md)). Removes hardcoded `container_name:` lines that block `--project-name` isolation.
3. **Bot first-message tutorial as a named deliverable** ([`05`](specs/v0.4-shell-tenant-reframe/05-bot-first-message.md)). Submodule PR in openclaw-vault.
4. **Podman strategy: bootstrapper sidecar** for v0.4. Bundled Podman is the v1.0 destination. Sidecar interface designed for forward compat.
5. **Plaintext `.env` stays for v0.4**, with explicit user-facing disclosure ("Your key is stored in plain text at `~/opentrapp/.env`. We're working on encrypted storage for a future release."). OS keychain migration is a v1.0 conversation that requires redesigning the proxy's env-var injection path.
6. **Pluggable-shell scoping is honest**: architecture earns the language; v0.4 ships only the OpenClaw tenant. "This makes future tenants cheap" — never "multi-tenant today."

---

## Verified facts that the specs depend on

The verification pass in this session ran four parallel investigations + targeted file reads. Findings the implementing agent should treat as established:

- **`pause_perimeter`** at [`commands/lifecycle.rs:87-119`](../app/src-tauri/src/commands/lifecycle.rs) is `compose stop` against the root compose.yml: stops all 4 containers, preserves all volumes, persists via `~/.opentrapp/paused`. Verified data preservation via reading `kill.sh` + `lifecycle.rs`.
- **`hard-kill` and `nuclear-kill`** wipe `vault-data` and the agent image. Confirmed in [`components/openclaw-vault/scripts/kill.sh:30-49,71-72`](../components/openclaw-vault/scripts/kill.sh).
- **`vault-proxy` reads `ANTHROPIC_API_KEY` per request** at [`vault-proxy.py:176-181`](../components/openclaw-vault/proxy/vault-proxy.py); never gates startup; warns if absent.
- **`SIGHUP` reloads the allowlist only**, not env vars ([`vault-proxy.py:49`](../components/openclaw-vault/proxy/vault-proxy.py)). To pick up new keys, the proxy needs `compose up -d --force-recreate vault-proxy`.
- **OpenClaw uses grammY long-polling**, not webhooks. Telegram allows one consumer per token; the wizard's test-message must use the documented handoff sequence (`deleteWebhook → getMe → poll → sendMessage → confirm-by-offset → release`) before vault-agent starts polling.
- **OpenClaw boots cleanly with empty `TELEGRAM_BOT_TOKEN`** — Telegram is silently disabled. Verified in [`components/openclaw-vault/docs/phase1-findings.md:134`](../components/openclaw-vault/docs/phase1-findings.md).
- **`podman-compose 1.0.6` skips build-only services on `compose pull`** unless `--force-local`. Three of our four services are `build:` stanzas. Bootstrap pipeline needs `compose build` AND `compose pull` as separate steps.
- **macOS and Windows additionally require `podman machine init && podman machine start`** after the OS install. No upstream-supported Linux rootless tarball exists.
- **`api.anthropic.com` is on the proxy allowlist** ([`components/openclaw-vault/proxy/allowlist.txt:4`](../components/openclaw-vault/proxy/allowlist.txt)).
- **Modal-over-home is feasible without routing rewrite.** `Setup.tsx` is at `/setup` route gated by `<Navigate>` redirect at `App.tsx:114`; the wizard step components are pure presentational. The connection-step blocks at `ConnectStep.tsx:149-169` (Anthropic) and `:171-191` (Telegram) get reused inside a new `<ActivationModal>`.
- **`tauri-plugin-shell` is already present** ([`Cargo.toml:13`](../app/src-tauri/Cargo.toml)); sidecar wiring is ~3 line additions to `tauri.conf.json` + `capabilities/default.json`.
- **`tauri-plugin-single-instance` is NOT yet configured** — must add (`~10 LOC`, register first per docs).
- **The `first-run-setup` workflow at [`config/orchestrator-workflows.yml:45`](../config/orchestrator-workflows.yml)** is dead vocabulary — defined but never invoked. The wizard reimplements bootstrap imperatively in [`pipeline-steps.ts`](../app/src/components/wizard/install-step/pipeline-steps.ts). The new bootstrap subsystem replaces this imperative path.

---

## Empirical gaps still open

Three things the implementing agent must validate before each merge:

1. **The 3-container partial bring-up has not yet run live.** [`07`](specs/v0.4-shell-tenant-reframe/07-container-name-cleanup.md) is the precondition; once `container_name:` lines are removed, the dryrun is `podman compose --project-name opentrapp-dryrun up -d vault-proxy vault-forge vault-pioneer` against a fresh test env. Reasoning supports it (one-directional `depends_on`, sleep-infinity daemons, independent networks) but no live confirmation yet.
2. **Windows MSI silent-install flags** (`/quiet /qn /norestart`) are standard but **NOT documented by Podman**. Empirically validate on a Windows VM during PR-3 (bootstrap service).
3. **macOS `.pkg` silent install command** (`installer -pkg ... -target /`) is extrapolated from `installer(8)`, not Podman-documented. Verify on a clean macOS VM during PR-3.

---

## Gotchas inherited from prior work

1. **Always run `make dogfood-fresh-sessions` before re-testing prompt changes.** OpenClaw's session transcripts at `/home/vault/.openclaw/agents/main/sessions/*.jsonl` cache prior responses; the model self-mimics them. Documented in [`tests/dogfood/CHECKLIST.md`](../tests/dogfood/CHECKLIST.md) §0a. Especially relevant for PR-6 (bot first-message tutorial) — the tutorial behaviour depends on the bot reading fresh CONSTRAINTS.md and not regurgitating cached "I don't know what to say first" responses.
2. **Cloudflare auto-injects a bot-management `<script>`** before `</body>` on every response from `opentrapp.com`. Any byte-level diff between the live HTML and the local `docs/index.html` will show false-positive divergence. Use `ssh hetzner sha256sum` (per the runbook §4a) for sync checks.
3. **Submodule changes need separate PRs** in their respective repos. The pattern: branch in submodule → commit + push to submodule's GitHub → merge submodule PR → bump submodule reference in parent → parent PR. Used three times in the previous session; PR-6 (bot tutorial) needs this discipline again.
4. **`HUMAN-TODO.md` §4 is sensitive** (adversarial registry-staging recipe). Don't stage, commit, or push that file. Operator-only.
5. **The bot is in `vault-agent` and cannot IPC to `vault-forge`.** The user-bridge model is the architectural correction recorded in PR #43's spec rewrite. Don't recommend bot-direct forge calls without acknowledging the IPC plumbing that would require.
6. **Hetzner deploys are out-of-band from app releases.** Marketing site at `opentrapp.com` ships when `docs/index.html` changes. Use [`docs/deploying-the-landing-page.md`](deploying-the-landing-page.md). `RELEASING.md` covers app tag-and-build separately.
7. **The maintainer's GitHub handle is `albertdobmeyer`** (current). The legacy `gitgoodordietrying` is deprecated.

---

## Outstanding operator queue (`HUMAN-TODO.md`, local-only)

These items are unchanged from the previous handoff. They're operator-driven; the agent assists by pasting commands, verifying output, summarising findings — but cannot drive these autonomously. They sit alongside v0.4 work and don't block it.

1. Tier C1 — first-launch wizard screenshot in `not_setup` state (becomes Tier C1' after v0.4: launch-button screenshot in `(ShellReady, Absent)`)
2. Tier D1 + D2 — graceful window-close and tray-Quit termination paths
3. Live re-run of Tier A4 — bot's hand-off behaviour. Run `make dogfood-fresh-sessions` first.
4. Adversarial skill staging for Tier B5 — needs ClawHub publishing credentials
5. Demo recording — 60-second discovery → install → use loop. **Now blocked on v0.4 shipping** (the demo can't be recorded against the wizard-as-entry UX since the new flow is fundamentally different)
6. OpenSSF Best Practices Badge submission — form pre-filled at [`docs/openssf-best-practices-application.md`](openssf-best-practices-application.md)

---

## Working state at session end

```
$ git log --oneline -5
e1e1bf3 feat(landing): add 4-gate skill vetting pipeline diagram to ecosystem section
2c189bc chore(submodule): update openclaw-vault — auto-detect clearance report fix
4be54fa chore(submodule): update openclaw-vault to clearance-enforcement commit
9699cf4 docs(landing): correct skill scanner claims to match implementation
5785dd9 chore(release): bump version to 0.4.0, add release notes and v0.4 specs

$ git submodule status
 a5a46ad  components/openclaw-vault    (heads/main)
 911b677  components/clawhub-forge     (heads/main)
 52b3db2  components/moltbook-pioneer  (heads/main, parked)
```

Working tree clean. All five test gates green on `main` (last checked 2026-05-11 pre-release).

### Pending (not blocking v0.4)

- SignPath approval — check `albertkdobmeyer@gmail.com`. See plan at `~/.claude/plans/ethereal-wiggling-rocket.md` for the full CI integration steps once approved.
- Dead Cloudflare API token in `/root/.secrets/certbot/cloudflare.ini` on Hetzner — worth regenerating next time you're in the Cloudflare dashboard.

---

## Verification approach for v0.4

Each spec has its own verification section. End-to-end coverage:

- **Unit:** Rust tests in `app/src-tauri/src/lifecycle.rs` and the new bootstrap module. Frontend unit tests in `app/src/components/HeroStatusCard.test.tsx`.
- **Integration:** test environments with Podman pre-installed, simulating partial states. **Precondition: PR-1 ([container-name cleanup](specs/v0.4-shell-tenant-reframe/07-container-name-cleanup.md)) must land first** so `--project-name` isolation works.
- **E2E:** Playwright in `app/e2e/` covers phase-transition sequences, activation, recovery paths, stop button.
- **Manual smoke:** clean VMs per OS at least once per release cut — macOS Sequoia, Windows 11, Ubuntu 24.04. Includes admin-prompt flow + `podman machine init`.
- **Dogfood:** new Tier-A6 (bot first-message tutorial) and Tier-A4-extended (full launch → activation → bot reply) added to [`tests/dogfood/CHECKLIST.md`](../tests/dogfood/CHECKLIST.md) when the relevant PRs land.

---

## Memory pressure caveat (still applies)

Maintainer's dev machine is a 2017 Lenovo IdeaPad with 7.2 GB RAM. Heavy parallel operations swap. Per maintainer's CLAUDE.md, max two Claude Code sessions simultaneously (one terminal, one Cursor). Stop dev servers and Ollama models between demos; check `free -h` periodically; if swap > 500 MB, stop everything non-essential before continuing.

The CI runs all heavy work; nothing in the v0.4 spec set requires the maintainer's machine to be the bottleneck.

---

## Cross-doc reference graph (orientation)

- **v0.4 specs:** [`docs/specs/v0.4-shell-tenant-reframe/`](specs/v0.4-shell-tenant-reframe/)
- **v0.4 design north-star:** [`docs/specs/v0.4-shell-tenant-reframe/_source-karen-from-hr.md`](specs/v0.4-shell-tenant-reframe/_source-karen-from-hr.md)
- **Architecture (this repository):** [`docs/trifecta.md`](trifecta.md), [`docs/whitepaper.md`](whitepaper.md), [`docs/diagrams.md`](diagrams.md), [`docs/adr/`](adr/)
- **Threat model:** [`docs/threat-model.md`](threat-model.md), [`docs/why-not-x.md`](why-not-x.md)
- **Releasing:** [`RELEASING.md`](../RELEASING.md), [`docs/deploying-the-landing-page.md`](deploying-the-landing-page.md)
- **Skill-installation policy:** [`docs/specs/2026-05-06-skill-installation-policy.md`](specs/2026-05-06-skill-installation-policy.md) — Option B accepted, user-bridge model
- **Dogfood test rig:** [`tests/dogfood/README.md`](../tests/dogfood/README.md), [`tests/dogfood/CHECKLIST.md`](../tests/dogfood/CHECKLIST.md)
