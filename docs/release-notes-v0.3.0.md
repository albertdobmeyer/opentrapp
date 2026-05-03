# Lobster-TrApp v0.3.0 — Delightful Sloth

**Status:** Tagged 2026-05-02 at commit `7ebdd8b`
**Target audience:** non-technical users (Karen) — first build that earns this audience
**Container baseline:** unchanged 4-container perimeter from v0.2.0; Split Shell hardcoded

## TL;DR

v0.3.0 is the **first build I'm comfortable putting in front of someone non-technical.**

v0.2.0 was the hardened-but-clunky release. v0.3.0 is the same security model, presented in a way Karen can actually navigate. Every screen that was a placeholder is now a real surface. Every termination path cleanly tears down the perimeter. The app now owns its own lifecycle instead of being a control panel that pretends.

The work spanned 8 sequenced passes over ~2 weeks (the "Delightful Sloth" UX-coherence polish phase). The final pre-ship audit is at `docs/specs/2026-05-02-pass-8-preship-walk.md`.

## What changed since v0.2.0

### The Home page is real
Was: "Coming in Phase E.2.2" + a `docs/specs/...` path in monospace.
Now: a 7-state hero card driven by a backend status aggregator that polls perimeter health + key validity + .env presence every 60s. Two tiles (Security + Spending), a proactive alerts banner that's invisible 90% of the time, and a daily rotating use-case tip.

### Discover is real
A 19-entry use-case gallery with category filters, search, favourites, and one-click Telegram deep-links. Karen browses, taps "Try this", and lands in her chat with the prompt prefilled.

### Preferences is real
Five honest sections: keys (with auto-restart on rotation), notifications (with OS-permission gate), startup (actually wired through `tauri-plugin-autostart`), re-run setup, and advanced mode.

### Lifecycle ownership (the big invisible win)
Pass 4's mandate: the perimeter is alive iff the app is alive.
- App start ⇒ `compose up -d` on a background thread.
- Graceful exit (window close, tray Quit, SIGTERM, SIGINT) ⇒ `compose down` synchronously with a 30s ceiling.
- SIGKILL ⇒ orphan containers reaped by RunGuard on next launch via PID-file diff.
- New `paused_by_user` state — pause from Home; survives an app restart via `~/.lobster-trapp/paused` marker; resume from the same button.

Seven termination paths, all clean.

### Auto-restart on key rotation
Save a new Anthropic key in Preferences and watch a "Restarting your assistant…" → "Your assistant is back online" toast sequence run in ~10 seconds. No more "Restart your assistant for the change to take effect" dead-end.

### Backend status aggregator
A single `AssistantStatus` enum (ok / starting / recovering / error_perimeter / error_key / paused_by_user / not_setup) that drives both the Home hero and the proactive alerts banner. The Anthropic auth probe uses the free `/v1/models` endpoint (not billable `/v1/messages`), cached 5 minutes, key-rotation invalidates immediately.

### What I deliberately did NOT build
Mid-phase, while testing a feature that pulled spending data from the Anthropic Admin API, I rejected the entire approach: *Anthropic Console already shows spending data better than I ever could, and the admin key needed for it has full org-admin scope (workspace management, billing changes, rate limits). Asking a non-technical user to put a high-privilege credential on disk just to display numbers Anthropic already shows in a better UI is off-thesis.*

Crystallized as the **"deserve-to-exist" test**: does this feature duplicate something the user already has (Anthropic Console for spend / Telegram for chat history), or does it solve a problem unique to running OpenClaw safely on a personal machine?

Outcome: the Spending tile is now a single deep-link card that opens `console.anthropic.com/cost`. The Activity tile (would have duplicated Telegram history) was removed entirely. Pass 8's pre-ship audit applied the same test to every other surface and flagged zero for removal.

## The numbers

Test infrastructure as of v0.3.0 tag:
- **56 / 56** Rust unit tests
- **175 / 175** vitest tests
- **25 / 25** Playwright e2e tests
- **42 / 42** orchestrator-check (0 warnings)
- TypeScript strict — clean
- Vite production build — 280 KB / **85 KB gzipped**
- **28 banned terms** enforced by Playwright on every commit (no developer jargon leaks)

Karen's full journey, scored against the 13-principle rubric:

| Moment | v0.2.0 | v0.3.0 |
|---|---|---|
| 1 — Discovery (landing page) | 6.2 | 6.2 (separate repo) |
| 2 — First-run + wizard | 7.6 | **9.0** |
| 3 — First chat (Telegram handoff) | 8.0 | 8.0 |
| 4 — Returning use (Home) | **3.8** | **9.0** |
| 5 — Monitoring (Security) | **4.9** | **8.5** |
| 6 — Adding tools (Discover) | **4.9** | **9.0** |
| 7 — Setting changes (Preferences) | **4.9** | **8.7** |
| 8 — Crash & recovery | ~7 | **9.0** |

The Pass-1 cliff at moments 4–7 (all stuck at ~4.8) is closed.

## Known limitations carried forward

These are real, none are blockers, all will be addressed in the post-launch Pass 9:

1. **ErrorBoundary** still shows raw `error.message` to the user. Fires only on unhandled exceptions; not seen in months of dogfooding. Friendlier-copy upgrade is the highest-leverage Pass 9 item.
2. **macOS / Windows install copy** still says "Podman Desktop" because that IS the package name. Pass 9 coordinates with the landing page to introduce a friendlier wrapper name.
3. **Help page** is a friendlier placeholder. The `generate_diagnostic_bundle` Tauri command already exists; needs a Help-page surface.
4. **Security Monitor** is a friendlier placeholder. Pass 9 derives content from `vault-proxy/var/log/vault-proxy/requests.jsonl` ("Today your assistant tried to visit X domains, all allowed; blocked Y attempts").

## Upgrading from v0.2.0

No data migration required. Settings, .env files, and the perimeter's container state all carry forward as-is. The new `paused_by_user` flag is opt-in (defaults to off; a marker file at `~/.lobster-trapp/paused` controls it).

The `spendingLimit` field that lived in `AppSettings` for a brief Pass-7-Day-1a window is gone — pre-existing users with that field saved will simply have an unused entry in their `settings.json` until next reset. Not worth a migration.

## Acknowledgments

This release exists because of one mid-pass course-correction: the 2026-05-02 vision recheck. Without it, v0.3.0 would have shipped a Spending tile that pulled real numbers via an over-privileged admin key — clever, but off-thesis. The recheck saved the next two days of work and produced a cleaner product. The principle is now in durable memory at `feedback_lobster_trapp_scope.md` and re-applied in every Pass 8 surface review.
