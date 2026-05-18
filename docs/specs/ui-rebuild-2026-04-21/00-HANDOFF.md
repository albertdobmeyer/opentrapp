# Handoff — UI/UX Rebuild: Non-Technical + Developer Mode Split

**Date:** 2026-04-21
**Author:** Albert + Claude Opus (planning session)
**Status:** Planning complete. Ready for implementation in a new instance.

---

## Mission in One Sentence

**Split the OpenTrApp UI into two distinct, purpose-built modes inside the same Tauri app: an Apple/Google-quality non-technical GUI (default) and a dense, information-rich developer dashboard (toggleable), so that each audience gets an interface optimized for their needs without compromise.**

---

## Why This Exists

The current UI mixes two user stories into one interface. Every polish pass has felt sluggish because we keep reconciling opposing needs:

- **Karen** (non-technical end user) needs simplicity, automation, and reassurance. She found us via Google, downloaded the installer, and wants an AI assistant she can talk to on Telegram. She doesn't want to think about OpenTrApp at all. It should feel like Windows Defender or a VPN app — invisible by default, reassuring when checked, actionable when something needs her attention.

- **The developer** found us on GitHub as four public repos. They're interested in the security model, forkability, customization. They want to inspect every component, see every log, customize every config. The current UI is closer to what they need, but the non-technical reframing has been eroding it.

The resolution is **not** to compromise. It's to build both interfaces, cleanly separated, inside the same Tauri app.

---

## Read These Docs First (In Order)

**Do not skip this.** Implementation without context produces the same kind of mixed-audience mush we're trying to fix.

1. **`01-vision-and-personas.md`** — Who Karen is, who the developer is, the product identity as "invisible security wrapper". Understand the audiences before touching code.
2. **`02-design-system.md`** — Visual language for both modes. Color, typography, spacing, components, motion. The two modes share foundations but apply them differently.
3. **`03-information-architecture.md`** — Routing, navigation, mode toggle, system tray. How the two modes coexist.
4. **`04-visual-assets-plan.md`** — 30+ illustrations and icons needed. Sourcing strategy (unDraw, Storyset, Heroicons, custom SVG).
5. **`05-automation-strategy.md`** — What the app auto-detects, auto-heals, auto-configures. "As few clicks as possible."
6. **`06-failure-ux-strategy.md`** — Self-heal → contact-support flow. No stack traces in user view.

Then read the per-screen specs as you implement each screen:

- **User mode:** `user-mode/07-onboarding.md` → `08-home-dashboard.md` → `09-security-monitor.md` → `10-preferences.md` → `11-help-and-support.md` → `12-use-case-gallery.md`
- **Developer mode:** `developer-mode/13-dev-dashboard-overview.md` → `14-component-operations.md`

---

## What's Been Done (Previous Sessions)

| Phase | Status | Artifact |
|-------|--------|----------|
| Phase A — Documentation alignment | ✅ | `docs/specs/2026-04-19-product-identity-spec.md`, updated CLAUDE.md, GLOSSARY.md, README |
| Phase B — Frontend reframe | ✅ | `app/src/lib/labels.ts`, role-based labels across all screens, 16 files touched |
| Phase C — Landing page reframe | ✅ | `docs/index.html` reframed for non-technical users |
| Phase D — v0.1.0 release | ✅ | Tagged, CI green, 9 binaries on GitHub releases (draft) |
| UX rubric | ✅ | `docs/specs/2026-04-20-ux-principles-rubric.md` — 10 principles, 13 screens scored |
| **Phase E — UI rebuild** | 📋 This handoff | 14 specs in this folder |

**Current state of the code:** Works end-to-end. Setup wizard completes. Vault, forge, pioneer containers build and run. Telegram pairing works. But the UI is still mixed-audience.

---

## What To Build (Implementation Order)

### Phase E.1 — Cross-cutting foundations (~1 day)

Implement the pieces that all screens depend on, in this order:

1. **Design tokens**: extend `app/tailwind.config.js` with the full palette, typography scale, spacing scale from spec `02-design-system.md`. Update `app/src/styles/globals.css` to match.
2. **Routing for dual mode**: refactor `app/src/App.tsx` per spec `03-information-architecture.md`. Add `/dev/*` route prefix for developer mode.
3. **Mode toggle**: add `mode: 'user' | 'developer'` to `AppSettings` in `app/src/lib/settings.ts`. Keyboard shortcut (`Cmd/Ctrl+Shift+D`) and Settings toggle to switch. Persist in Tauri store.
4. **Install new plugins**:
   - `@tauri-apps/plugin-notification` (system notifications)
   - `tauri-plugin-autostart` (Rust, for autostart)
   - `@tauri-apps/plugin-clipboard-manager` (for diagnostics export)
5. **System tray**: add tray config to `app/src-tauri/tauri.conf.json`. Minimum implementation: status icon + "Open Dashboard" + "Quit" menu.

### Phase E.2 — User mode screens (~2-3 days)

Build screens in order 07 → 12. Each screen's spec has exact copy, layout, states, and test plan.

- `07-onboarding.md` — Condense current 6-step wizard to 4 steps
- `08-home-dashboard.md` — New dashboard (replaces current Dashboard.tsx)
- `09-security-monitor.md` — New screen (requires activity persistence — spec defines schema)
- `10-preferences.md` — Replace current Settings.tsx for user mode
- `11-help-and-support.md` — New screen
- `12-use-case-gallery.md` — New screen (optional polish; can ship without)

### Phase E.3 — Developer mode (~1 day)

- `13-dev-dashboard-overview.md` — Routing, layout, sidebar
- `14-component-operations.md` — Mostly reuses existing components (CommandPanel, WorkflowPanel, ConfigPanel) in the new shell. NEW: logs viewer, manifest inspector, allowlist editor, security audit runner.

### Phase E.4 — Polish + verification (~0.5 day)

- Add all new visual assets per `04-visual-assets-plan.md`
- Expand Playwright banned-terms list with new vocabulary from spec 02
- Run full test suite
- Score every new screen against `docs/specs/2026-04-20-ux-principles-rubric.md` — any screen below 7 blocks shipping
- Visual walkthrough: capture screenshots of every screen in both modes
- Update `README.md` if user-facing install/usage changes
- Commit in logical chunks (not one megacommit)

---

## Verification Commands

After each phase:

```bash
# From repo root
bash tests/orchestrator-check.sh              # 42 orchestration checks

# From app/
cd app
npx tsc --noEmit                              # TypeScript
npx vitest run                                # Unit tests (target: all pass)

# With Vite running
npx vite --port 1420 &
npx playwright test                           # E2E (target: all pass)

# With Tauri running
source ~/.cargo/env
npm run tauri dev                             # Full app
```

---

## Success Criteria

A non-technical user (someone who has never heard of OpenClaw, containers, or security sandboxes) should be able to:

1. **Install in under 2 minutes** from downloading the installer.
2. **Connect their first Telegram message** within 5 minutes of install.
3. **Never see** developer terminology: container, proxy, manifest, compose, vault, forge, pioneer, seccomp, component.yml, podman.
4. **Forget about OpenTrApp after week 1** — the app runs invisibly in the tray, checks in only when something needs attention.
5. **Know what to do when something breaks** — contact-support flow with self-heal and diagnostic export.

A developer should be able to:

1. **Toggle to Advanced Mode** from Settings or `Cmd/Ctrl+Shift+D`.
2. **See everything** they need: component status, commands, configs, workflows, logs, manifests, security audit.
3. **Customize everything** that's customizable: allowlist, shell level, refresh interval.
4. **Export a full diagnostic bundle** for reporting issues.

---

## What NOT To Do

- **Do not add an in-app chat.** Telegram is the only AI conversation surface. Our app is the security wrapper, not a chat client.
- **Do not compromise either mode to please the other.** If a design decision helps Karen but bothers developers, that's fine — they have Advanced Mode. If it helps developers but clutters Karen's view, that's also fine — user mode is minimal.
- **Do not rewrite the Rust backend.** The manifest-driven architecture is correct. Extend it (activity persistence) but don't replace it.
- **Do not change `component.yml` schemas** without updating all three alignment layers (JSON Schema, Rust `manifest.rs`, TS `types.ts`).
- **Do not commit secrets.** The vault's `.env` stays gitignored.
- **Do not ship screens that score < 7 on the UX rubric.** Fix or pull from the release.
- **Do not make the specs worse.** If you change direction, update the relevant spec first, then implement.

---

## Key Files (Reference)

| Purpose | File |
|---------|------|
| Top-level app shell | `app/src/App.tsx` |
| Layout | `app/src/components/Layout.tsx` |
| Sidebar | `app/src/components/Sidebar.tsx` |
| Current Dashboard | `app/src/pages/Dashboard.tsx` |
| Current Settings | `app/src/pages/Settings.tsx` |
| Settings types | `app/src/lib/settings.ts` |
| Settings hook | `app/src/hooks/useSettings.ts` |
| Tauri IPC wrappers | `app/src/lib/tauri.ts` |
| Role-to-label mapping | `app/src/lib/labels.ts` |
| Tauri config | `app/src-tauri/tauri.conf.json` |
| Tauri Rust Cargo | `app/src-tauri/Cargo.toml` |
| NPM deps | `app/package.json` |
| Global CSS / tokens | `app/src/styles/globals.css` |
| Tailwind config | `app/tailwind.config.js` |
| Playwright banned terms | `app/e2e/user-facing.spec.ts` |
| Product identity | `docs/specs/2026-04-19-product-identity-spec.md` |
| UX rubric | `docs/specs/2026-04-20-ux-principles-rubric.md` |

---

## Contact / Escalation

If anything is unclear or a spec contradicts reality, **update the spec first**, then implement. Do not just silently deviate — future instances will read the specs and re-learn the mistakes.

Good luck. Make Karen proud.
