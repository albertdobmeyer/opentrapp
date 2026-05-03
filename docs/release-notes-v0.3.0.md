# Lobster-TrApp v0.3.0 — Release Notes

**Tagged:** 2026-05-02 (commit `7ebdd8b`; subsequently `75dbccb` after a release-build correction)
**Container baseline:** four-container perimeter as in v0.2.0; default Split Shell.
**Target audience:** non-technical end users — the first build judged ready for that audience.

## Summary

v0.2.0 shipped the security architecture in functional but UI-incomplete form. v0.3.0 closes the user-experience gap: every previously-placeholder surface in the desktop application is now implemented; the application itself owns the perimeter lifecycle; backend status aggregation drives a stateful Home view. The security model is unchanged.

The work was carried out as eight sequenced passes ("Delightful Sloth" UX-coherence phase) over approximately two weeks. The pre-ship audit is at [`docs/specs/2026-05-02-pass-8-preship-walk.md`](specs/2026-05-02-pass-8-preship-walk.md).

## Changes since v0.2.0

### Implemented user-mode pages

| Page | v0.2.0 | v0.3.0 |
|---|---|---|
| Home | Placeholder ("Coming in Phase E.2.2") | Seven-state hero card driven by a 60-second backend status aggregator; Security and Spending tiles; proactive-alerts banner; daily use-case tip |
| Discover | Placeholder | 19-entry use-case gallery with category filters, search, favourites, and Telegram deep-links with prefilled prompts |
| Preferences | Placeholder | Five sections — keys (with auto-restart on rotation), notifications (with OS permission gate), startup (`tauri-plugin-autostart` wired through), re-run setup, advanced mode |
| Security Monitor | Placeholder | Friendlier placeholder; substantive view deferred to Pass 9 |
| Help | Placeholder | Friendlier placeholder; substantive view deferred to Pass 9 |

### Lifecycle ownership

The desktop application now owns the lifetime of the four-container perimeter:

- Application start triggers `compose up -d` on a background thread.
- Graceful exit (window close, tray Quit, SIGTERM, SIGINT) triggers `compose down` synchronously, with a 30-second ceiling.
- Following SIGKILL, orphan containers are detected and stopped on the next launch via a PID-file mechanism (`RunGuard`).
- A new `paused_by_user` state allows the user to suspend the perimeter without exiting the application; the state survives application restart via the `~/.lobster-trapp/paused` marker file.

Seven distinct termination paths were validated as cleanly tearing down the perimeter.

### Backend status aggregation

A new module evaluates an `AssistantStatus` enum (`ok` / `starting` / `recovering` / `error_perimeter` / `error_key` / `paused_by_user` / `not_setup`) on a 60-second interval. Both the Home hero state machine and the proactive-alerts banner subscribe to it. Anthropic-key validity is probed via the free `/v1/models` endpoint (not the billable `/v1/messages`); the result is cached for five minutes and invalidated immediately on key rotation.

### Auto-restart on key rotation

Saving a new Anthropic key in Preferences now triggers a perimeter restart with a sticky-toast progress sequence (typical duration ~10 seconds). v0.2.0 required the user to manually restart the application after a key change.

### Removed: admin-API spending integration

A mid-phase scope review on 2026-05-02 removed an in-progress feature that read live spending data from the Anthropic Admin API. The admin-API key carries full organisation-administration scope (workspace management, billing modification, rate-limit control); requiring a non-technical user to place such a credential on disk to display numbers already presented in the Anthropic Console was judged out of scope.

The review crystallised a "deserve-to-exist" test: does a proposed feature duplicate functionality the user already has elsewhere (Anthropic Console for spending and usage; Telegram for chat history), or does it address a problem specific to running OpenClaw safely on a personal machine? Pass 8 applied the same test to every shipped surface; no surface was flagged for removal.

The Spending tile is now a single deep-link to `console.anthropic.com/cost`. The Activity tile (which would have duplicated Telegram chat history) was removed entirely.

## Verification

Test results at the v0.3.0 tag:

- Cargo unit tests: **56 / 56** passing
- Vitest: **175 / 175** passing
- Playwright end-to-end: **25 / 25** passing
- Orchestrator-check validation suite: **42 / 42** passing, 0 warnings
- TypeScript strict mode: clean
- Vite production build: **280 KB** total, **85 KB gzipped**
- Banned-term enforcement: **28 terms** verified absent from user-visible text by Playwright on every commit

### Karen-journey rubric scores (13-principle rubric, [`docs/specs/2026-04-20-ux-principles-rubric.md`](specs/2026-04-20-ux-principles-rubric.md))

| Journey moment | v0.2.0 | v0.3.0 |
|---|---|---|
| 1 — Discovery (landing page) | 6.2 | 6.2 (separate repository) |
| 2 — First run + wizard | 7.6 | **9.0** |
| 3 — First chat (Telegram handoff) | 8.0 | 8.0 |
| 4 — Returning use (Home) | 3.8 | **9.0** |
| 5 — Monitoring (Security) | 4.9 | **8.5** |
| 6 — Adding tools (Discover) | 4.9 | **9.0** |
| 7 — Setting changes (Preferences) | 4.9 | **8.7** |
| 8 — Crash and recovery | ~7 | **9.0** |

The cliff at moments 4–7 (all four user-mode pages at ~4.9 in v0.2.0 due to placeholder content) is closed.

## Known limitations

The following are documented as accepted limitations in v0.3.0 and queued for the post-launch Pass 9:

1. **ErrorBoundary** displays raw `error.message` to the user. Fires only on unhandled exceptions; not observed during dogfooding. Replacement copy with a "Show technical details" disclosure is the highest-priority Pass 9 task.
2. **macOS / Windows install guidance** uses the literal product name "Podman Desktop". A friendlier wrapper name will be coordinated with the landing page in Pass 9.
3. **Help** and **Security Monitor** ship as friendlier placeholders. The `generate_diagnostic_bundle` Tauri command (already implemented) will be surfaced in Help; Security Monitor will derive content from `vault-proxy/var/log/vault-proxy/requests.jsonl`.

Installer signing uses the Tauri auto-updater key only. OS-level code-signing certificates (Apple Developer ID, Authenticode) are not in place; macOS Gatekeeper and Windows SmartScreen will warn on first launch.

## Upgrading from v0.2.0

No data migration is required. Settings, `.env` files, and existing container state carry forward unchanged. The new `paused_by_user` mechanism is opt-in and defaults to off.

The `spendingLimit` field that briefly existed in `AppSettings` during the unwound admin-API integration is removed; users who saved settings during that window will retain an unused entry in `settings.json` until the next reset. No migration is provided.

## Implementation history

The eight passes of the Delightful-Sloth phase, with their resulting commits:

| Pass | Focus | Commit(s) |
|---|---|---|
| 1 / 1.5 | Dogfood walkthrough + live first-chat signal | (audit only — no code changes) |
| 2 | Aspirational UX spec | (spec only) |
| 3 | Rubric extension | (spec only) |
| 4 | Lifecycle ownership | `4e2a…` (lifecycle module + signal handlers) |
| 5 | Wizard polish | `1f879d9` |
| 6 | User-mode page rebuild | `9e5ba11` → `2ea0631` (5 commits) |
| 7 | Notifications + recovery + cleanup | `6c0c8da`, `c052601`, `9097c7a`, `c27fecc`, `6646030` |
| 8 | Pre-ship walkthrough + ship recommendation | `7ebdd8b` |
| Release-build correction | Lockfile-integrity fix + version-string alignment | `104e2c4`, `75dbccb` |

The mid-phase scope review that removed the admin-API spending feature is documented in [`memory/feedback_lobster_trapp_scope.md`](../.claude/projects/-home-albertd-Repositories-lobster-trapp/memory/feedback_lobster_trapp_scope.md) and applied to every Pass 8 surface review.
