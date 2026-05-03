# Lobster-TrApp (App) — TODO

Tracked gaps from the 2026-03-03 audit. This covers the Tauri app's own issues, not submodule internals. See `docs/vision-and-status.md` for the high-level roadmap.

---

## Test Framework Not Configured (Phase 2 Blocker) — RESOLVED

- [x] `vitest`, `@testing-library/react`, `@testing-library/jest-dom`, `jsdom`, and `@playwright/test` added to `package.json` devDependencies
- [x] `vitest.config.ts` created with jsdom environment, globals, path aliases, and setup file
- [x] 52 frontend unit tests across 8 test files + 17 Rust unit tests + 40 orchestrator checks all passing
- [ ] 4 Playwright E2E tests exist in `tests/` — runnable but need a dev server (Phase 4)

---

## ANSI Color Rendering (Phase 4) — RESOLVED

- [x] Full ANSI escape code parser in `app/src/components/renderers/ansi.ts` (parseAnsi)
- [x] `AnsiLine` component renders styled spans with color, bold, dim, italic, underline, strikethrough
- [x] Wired into `LogRenderer` and `TerminalRenderer`

---

## Streaming Not Wired to UI (Phase 4) — RESOLVED

- [x] `StreamOutput` component with real-time output display
- [x] `CommandPanel` detects `type: stream` and routes to `StreamOutput` instead of blocking execution
- [x] Elapsed timer shows duration while streaming
- [x] Auto-scroll, stop button, exit code display

---

## Settings Persistence (Phase 4) — RESOLVED

- [x] `tauri-plugin-store` v2 added (Rust + npm)
- [x] `AppSettings` interface: monorepoPathOverride, autoRefreshInterval, wizardCompleted, lastViewedComponentId
- [x] `useSettings` hook wraps store with typed read/write and forward-compatible merge
- [x] `AppContext` provides settings to all components
- [x] Settings page has working controls: monorepo path override, refresh interval slider, save/cancel
- [x] Backend `set_monorepo_root` command validates path and re-discovers components
- [x] `monorepo_root` is now `RwLock<PathBuf>` for thread-safe mutation
- [x] Last-viewed component persisted and restored

---

## Error UX + Loading States (Phase 4) — RESOLVED

- [x] Toast notification system: color-coded borders, auto-dismiss, expandable details, retry button
- [x] Error classification: maps OrchestratorError patterns to categories (timeout, not_found, permission, execution, config, parse)
- [x] Toasts wired into all hooks: useCommand, useConfig, useManifests, useCommandStream, useComponentStatus
- [x] Skeleton screens for Dashboard (loading cards) and ComponentDetail (loading header + blocks)
- [x] Debounced status probe toast (only after 3 consecutive failures)

---

## YAML Validation (Phase 4) — RESOLVED

- [x] `YamlEditor` validates YAML syntax with `js-yaml` before saving
- [x] Parse errors shown inline with line number
- [x] Error clears on edit, save is blocked until syntax is valid

---

## Setup Wizard (Phase 3) — RESOLVED

- [x] Schema extended with optional `prerequisites` section (container_runtime, setup_command, config_files, check_command)
- [x] All 3 component manifests updated with prerequisites sections
- [x] Backend: `check_prerequisites`, `init_submodules`, `create_config_from_template` commands
- [x] Frontend: 5-step wizard (Welcome → Prerequisites → Submodules → Config → Complete)
- [x] First-run detection: redirects to `/setup` when `wizardCompleted` is false
- [x] "Re-run Setup Wizard" button in Settings page
- [x] `usePrerequisites` hook with toast notifications
- [x] Orchestrator check section 8 validates prerequisites cross-references (40 checks total)

---

## card-grid Renderer (Phase G) — Spec Written

- [ ] `card-grid` output display is aliased to `ReportRenderer` instead of having its own implementation
- [ ] Should render structured data as a grid of cards (e.g., skill scan results, census data)
- [ ] **Spec:** `docs/specs/2026-04-07-card-grid-renderer.md`

---

## Finalization — Remaining Work (v4 Roadmap)

Full roadmap: `docs/roadmap-v4-finalization.md` (supersedes `docs/superpowers/plans/2026-04-04-master-roadmap-v3.md`)

| Phase | Area | Spec |
|-------|------|------|
| F | Test infrastructure + CI hardening | `docs/specs/2026-04-07-ci-integration-tests.md` |
| G | Card-grid renderer | `docs/specs/2026-04-07-card-grid-renderer.md` |
| H | Cross-platform bundle + updater | `docs/specs/2026-04-07-bundle-and-updater.md` |
| I | Setup wizard E2E + forge polish | `docs/specs/2026-04-07-setup-wizard-e2e.md` |
| J | Landing page + release prep | (no spec — straightforward) |

---

## CSP Headers (Phase 6) — RESOLVED

- [x] Restrictive CSP set in `tauri.conf.json`: `default-src 'self'`, `script-src 'self'`, `style-src 'self' 'unsafe-inline'` (Tailwind), `connect-src ipc: http://ipc.localhost` (Tauri IPC)
- [x] No `unsafe-eval`, no external script sources

---

## Deep-Link Race Condition (Phase 6) — RESOLVED

- [x] `get_component` now falls back to `discover_components()` on cache miss
- [x] Cache is populated as a side effect, so subsequent calls are fast
- [x] Direct navigation to `/component/:id` works without visiting the dashboard first
