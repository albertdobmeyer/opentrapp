# Lobster-TrApp — Vision & Status

> **DEPRECATED (2026-03-27):** This document is outdated. It has been superseded by:
> - **`docs/trifecta.md`** — Cross-module strategy and ownership matrix
> - **`docs/superpowers/plans/2026-03-25-master-roadmap-v2.md`** — Unified product roadmap
> - **Per-module roadmaps** — `components/*/docs/roadmap.md`
>
> This file is preserved as historical context. Do not update it — update the successor documents instead.

**Last updated**: 2026-03-03 (frozen)

This is the centralized planning document for the entire OpenClaw stack. It lives in lobster-trapp because the orchestrator is the integration point for all components.

---

## The Product Thesis

The OpenClaw ecosystem will mature beyond terminal-only developers. When non-technical users arrive — people who want to run their own agentic workstation, explore an agent social network, or contribute skills — Lobster-TrApp will be the **only security-first GUI** for this ecosystem. The app earns its existence by making dangerous-by-default container orchestration safe-by-default for people who can barely find a download button.

---

## Detailed Gap Tracking

Each repo now has a `TODO.md` documenting specific gaps found in the 2026-03-03 audit. This vision doc stays as the high-level overview; the TODO files are the actionable checklists:

- `components/openclaw-vault/TODO.md` — monitoring stubs, VM isolation stubs, test bug
- `components/clawhub-forge/TODO.md` — devcontainer setup, CI auto-publish, registry API
- `components/moltbook-pioneer/TODO.md` — no tests, safe_patterns bug, chmod, eval in curl
- `docs/TODO.md` — app-level gaps (test framework, ANSI, streaming, setup wizard)

---

## The Four Repos

| Repo | Role | What It Does |
|------|------|-------------|
| **lobster-trapp** | Orchestrator + GUI | Discovers components via manifests, renders dashboards, controls lifecycle |
| **openclaw-vault** | Runtime | Hardened 2-container sandbox. API keys never touch the agent container. |
| **clawhub-forge** | Toolchain | Skill development workbench. 87-pattern security scanner, gated publish pipeline. |
| **moltbook-pioneer** | Network | Safe reconnaissance of the Moltbook agent social network. Feed scanner, census, identity tools. |

---

## Component Maturity

### openclaw-vault — 75%

**Core security model: COMPLETE.** The two-container stack (agent + mitmproxy sidecar) is fully implemented with production-grade security: read-only root filesystem, all Linux capabilities dropped, custom seccomp profiles (~150 allowed syscalls, deny-by-default), noexec tmpfs mounts, 256 PID limit, 4GB RAM cap, proxy-injected API keys.

| Area | Status | Notes |
|------|--------|-------|
| compose.yml | Done | All security controls wired, networks isolated |
| Containerfile | Done | Multi-stage, image digest-pinned, non-root user, tini PID 1 |
| vault-proxy.py | Done | 241-line mitmproxy addon: allowlist, key injection, exfil detection, response scanning |
| Seccomp profiles | Done | Deny-by-default, separate profiles for vault and proxy |
| setup.sh / setup.ps1 | Done | Container runtime detection, interactive key input, auto-verify |
| kill.sh / kill.ps1 | Done | 3-level escalation: soft/hard/nuclear (includes WSL/Hyper-V teardown) |
| verify.sh | Done | 15-point live security check against running container |
| entrypoint.sh | Done | Solves tmpfs/config copy race correctly |
| Proxy allowlist | Done | Annotated with security trade-offs per domain |
| Hardening config | Done | Approval mode, no persistence, no telemetry, no pairing |
| 12 isolation tests | Done | Real tests that exec into live containers (not stubs) |
| README.md | Done | 21KB, honest residual-risk section |
| Makefile | Done | 7 targets wrapping shell scripts, runtime auto-detection (podman/docker) |
| **Monitoring (3 scripts)** | **STUBS** | network-log-parser.py, session-report.sh, skill-scanner.sh — all placeholder |
| **Phase 2 VM isolation** | **STUBS** | Hyper-V and WSL scripts are placeholders (config files are real) |
| Test bug | Minor | test-network-isolation.sh uses `wget` which was stripped from image |

**No blockers for GUI integration.** Makefile now wraps all shell scripts with runtime auto-detection.

---

### clawhub-forge — 83%

**Security scanning pipeline: COMPLETE.** The most feature-rich component. 87 malicious patterns across 13 MITRE ATT&CK categories, zero-trust line-by-line classification, SARIF 2.1.0 output for GitHub code scanning, .scanignore with line-range syntax, gated publish pipeline, and a self-validation test suite.

| Area | Status | Notes |
|------|--------|-------|
| skill-scan.sh | Done | 360 lines, 4 output formats, .scanignore, self-suppression fix |
| skill-verify.sh | Done | 287 lines, zero-trust model, .trust hash-pinning fast path |
| skill-lint.sh | Done | 149 lines, frontmatter + structure + content density checks |
| skill-test.sh | Done | Runner wrapper (delegates to framework) |
| skill-new.sh | Done | Scaffolder with 3 templates, sed-based placeholder substitution |
| skill-publish.sh | Done | 5-gate pipeline: lint -> scan -> verify -> test -> publish |
| skill-stats.sh | Done | 303 lines, 3 modes (table/trend/rank), API integration |
| registry-explore.sh | Done | API browser with snapshot caching |
| workbench-verify.sh | Done | 12-point health check calling real sub-tools |
| pipeline-report.sh | Done | Value summary with integrated stats |
| patterns.sh | Done | 87 patterns + 16 prompt injection patterns |
| line-classifier.sh | Done | 277 lines, batch blocklist + per-line allowlist |
| trust-manifest.sh | Done | SHA-256 hash pinning for verified skills |
| sarif_formatter.py | Done | SARIF 2.1.0 with MITRE ATT&CK rule properties |
| 25 skills | Done | 12,260 lines of curated reference content |
| 168 test functions | Done | Domain-specific assertions, not boilerplate |
| 9 tool tests | Done | Integration tests with fixture creation |
| Scanner self-test | Done | 10-point accuracy validation (known-bad, known-clean, allowlisted) |
| CI pipeline | Done | lint -> scan -> test -> tool-tests, SARIF upload to GitHub |
| Makefile | Done | 27 targets with auto-generated help |
| DevContainer | Done | Node 20, Python 3.12, VS Code extensions |
| component.yml | Done | 14 commands, 3 states, 1 health probe |
| **Registry integration** | **Untested** | stats/explore/publish hit `clawdhub.com` API — may not be live |
| **.devcontainer/setup.sh** | **Missing** | Referenced in devcontainer.json but file doesn't exist |
| **Auto-publish CI** | **TODO** | Commented out in CI pipeline |
| **No .trust files exist** | Minor | No skill has been through full publish gate yet |

**No blockers for GUI integration.** Makefile exists, all commands work.

---

### moltbook-pioneer — 73%

**Feed scanner and documentation: COMPLETE.** All three tools are fully implemented with real bash logic, not stubs. The 30-pattern injection database and the three-tier documentation (platform anatomy, threat landscape, safe participation guide) are publication-quality.

| Area | Status | Notes |
|------|--------|-------|
| feed-scanner.sh | Done | 381 lines, 3 modes, loads patterns from YAML, allowlist filtering |
| agent-census.sh | Done | 233 lines, census + trend modes, multi-endpoint API calls |
| identity-checklist.sh | Done | 204 lines, 7-section pre-flight check |
| injection-patterns.yml | Done | 30 patterns across 6 categories |
| feed-allowlist.yml | Done | Structure seeded, intentionally sparse |
| .env.example | Done | 8 variables with conservative rate-limit defaults |
| platform-anatomy.md | Done | Full API reference, data models, ecosystem diagram |
| threat-landscape.md | Done | 4 threat categories, real incidents, risk matrix |
| safe-participation-guide.md | Done | 3 engagement tiers, retraction plan template |
| examples/ | Done | Annotated first-post and feed-analysis examples |
| component.yml | Done | 6 commands, 3 configs, 3 states, 1 health probe |
| CLAUDE.md | Done | Session context guide |
| **No automated tests** | **GAP** | Zero test files — unlike vault (12 tests) and forge (168 functions) |
| **safe_patterns not wired** | **BUG** | feed-allowlist.yml `safe_patterns` key is silently ignored by scanner |
| **No executable bits** | **Minor** | Fresh clone needs `chmod +x tools/*.sh` (not documented) |
| **eval in curl** | **Minor** | feed-scanner.sh uses `eval` for auth header construction |
| **API may not exist** | **Risk** | Tools call `moltbook.com/api/v1` — unclear if this is a live endpoint |

**No blockers for GUI integration.** Commands are direct `./tools/*.sh` calls, no Makefile needed.

---

### lobster-trapp (Tauri App) — 78%

**Manifest-driven GUI framework: COMPLETE.** The Rust backend has 10 fully-implemented Tauri commands with zero stubs, injection-safe arg interpolation, path traversal protection, and proper async. The React frontend has a complete dark design system, live status polling, health badges with threshold evaluation, grouped command panels with danger confirmation, format-specific config editors, and 6 output renderers.

#### Rust Backend (18 .rs files, 10 commands)

| Area | Status | Notes |
|------|--------|-------|
| Manifest parsing | Done | Full serde structs covering entire schema |
| Component discovery | Done | Glob-based filesystem scan, sorted, cached |
| Command execution | Done | Arg interpolation with injection-safe quoting, per-command timeout |
| Dynamic options | Done | `options_from` shell commands for dropdown population |
| Status probes | Done | exit_code, stdout_contains, stdout_regex evaluation |
| Health probes | Done | Raw command execution, frontend handles parsing |
| Config read/write | Done | Path traversal prevention via canonicalize + starts_with |
| Streaming | Done | Async stdout/stderr line readers, Tauri event emission |
| Stream dedup | Done | Kills old process before starting new stream |
| Shell detection | Done | Windows Git Bash path candidates + which/where fallback |
| Error handling | Done | 8-variant thiserror enum, serializable for IPC |
| 14 unit tests | Done | Manifest parsing, injection prevention, discovery |

#### React Frontend (26 files, 6 hooks)

| Area | Status | Notes |
|------|--------|-------|
| Dashboard | Done | Responsive grid, loading/empty states, placeholder sorting |
| Component detail | Done | Header, status badge, health badges, commands, configs |
| Command panel | Done | Grouped, availability-filtered, arg forms, danger confirmation |
| Argument forms | Done | All 4 arg types: string, number, boolean, enum (with dynamic options) |
| Config editors | Done | EnvEditor (secret masking), YamlEditor (textarea), LineListEditor (pills) |
| Status polling | Done | Configurable interval, placeholder optimization |
| Health badges | Done | Per-probe intervals, threshold evaluation (numeric + string) |
| Output renderers | Done | 6 renderers: log, terminal, checklist, table, badge, report |
| Dark design system | Done | Tailwind, gray-950 bg, danger-coded buttons |
| Dynamic icons | Done | Lucide-react, kebab-to-PascalCase lookup |
| Routing | Done | React Router v6, 3 routes |
| Sidebar | Done | Dynamic component list, active highlighting |
| **ANSI color rendering** | **MISSING** | ansi.ts strips codes only — all terminal output is plain text |
| **Streaming not wired to UI** | **GAP** | useCommandStream exists but CommandPanel only uses useCommand |
| **Settings page** | **STUB** | Path override input is UI-only, never calls backend |
| **Test framework** | **BROKEN** | Vitest/Playwright installed but not in package.json, no config, no test script |
| **YAML validation** | **MISSING** | YamlEditor saves without syntax checking |
| **Setup wizard** | **NOT STARTED** | No prerequisite detection or first-run flow |
| **card-grid renderer** | **ALIASED** | Maps to ReportRenderer instead of distinct implementation |

---

## The 6-Phase Roadmap

### Phase 1: Cleanup & Consolidation — DONE

- [x] Rename docker-compose.yml -> docker-compose.example.yml
- [x] Convert moltbook-pioneer from placeholder to real submodule
- [x] Create component.yml for moltbook-pioneer
- [x] Purge all clawhub-lab -> clawhub-forge references (23+ across 3 repos)
- [x] Update CLAUDE.md, README.md, memory files
- [x] Cross-repo harmonization (CLAUDE.md, .gitignore, LICENSE, sort_order, state labels)
- [x] All 39 orchestrator checks pass, 14 Rust tests pass

### Phase 2: Distribution & CI — NOT STARTED

- [ ] Add `vitest` + `@testing-library/react` to package.json devDependencies
- [ ] Create vitest.config.ts, add `test` script to package.json
- [ ] Verify all 22 frontend unit tests pass
- [ ] Verify 4 Playwright e2e tests pass
- [ ] GitHub Actions CI: `cargo test` + `npm test` + `orchestrator-check.sh` on PR
- [ ] NSIS installer configuration review
- [ ] macOS dmg + Linux AppImage bundle targets
- [ ] Release workflow (tag -> build -> publish)

### Phase 3: Setup Wizard — NOT STARTED

The highest-impact UX feature for non-technical users.

- [ ] Prerequisite detection: Podman/Docker installed? Running? Container runtime version?
- [ ] Submodule health check: cloned? correct remote? detached HEAD?
- [ ] First-run flow: guided config setup, API key entry, container image build
- [ ] Progress indicators for long operations (image builds)
- [ ] Error recovery: what to do when Docker isn't installed, when builds fail

### Phase 4: Error UX — NOT STARTED

- [ ] ANSI color rendering in terminal output (replace strip-only ansi.ts)
- [ ] Wire useCommandStream to CommandPanel for live streaming output
- [ ] YAML/JSON syntax validation before config save
- [ ] Friendly error messages for common failures (container not running, network timeout)
- [ ] Settings page: wire monorepo path override to backend

### Phase 5: Component Build-Out — NOT STARTED

- [x] **openclaw-vault Makefile** — DONE (7 targets, runtime auto-detection)
- [ ] openclaw-vault monitoring scripts (log parser, session report, skill scanner)
- [ ] moltbook-pioneer automated tests
- [ ] moltbook-pioneer: wire safe_patterns from feed-allowlist.yml into scanner
- [ ] Fix moltbook-pioneer chmod +x issue (document or add to setup)
- [ ] clawhub-forge: create .devcontainer/setup.sh (referenced but missing)

### Phase 6: Polish & Hardening — NOT STARTED

- [ ] card-grid renderer (distinct from report)
- [ ] CSP headers for production builds
- [ ] Keyboard navigation and accessibility
- [ ] App auto-update mechanism
- [ ] Performance: RwLock instead of Mutex for read-heavy state
- [ ] Deep-link race condition fix (get_component before list_components)

---

## Critical Path

The shortest path to a usable product for non-technical users:

```
Phase 2 (CI/tests)     ← confidence to ship
    ↓
Phase 3 (setup wizard) ← non-technical users can install
    ↓
Phase 5 (vault Makefile) ← GUI can actually control vault
    ↓
Phase 4 (ANSI + streaming) ← output is readable
    ↓
Phase 6 (polish)       ← production quality
```

**The single highest-priority item**: Test framework configuration (Phase 2). Without vitest configured, no automated test validation can run in CI.

---

## Platform Reach

| Platform | Status | Notes |
|----------|--------|-------|
| Windows | Primary target | NSIS installer configured in tauri.conf.json |
| macOS | Config change away | Add `"dmg"` to bundle targets |
| Linux | Config change away | Add `"appimage"` or `"deb"` to bundle targets |
| Mobile | Out of scope | Core value = local container orchestration. Phones can't run Podman. |
| Remote browser | Out of scope | Security risk (exposes container control over network) |

---

## Hard Constraints

1. **Zero component-specific knowledge in the Rust backend.** If you deleted openclaw-vault and dropped in a completely different component with a valid component.yml, the app must render it correctly. The moment vault-specific or forge-specific code appears in the Rust backend, the manifest-driven architecture is compromised.

2. **The manifest is the API.** `component.yml` (validated against `schemas/component.schema.json`) is the only interface between the Tauri app and any component. The three component repos own their domain logic. The app owns the UX.

3. **The app does exactly three things:**
   - Detect and bootstrap prerequisites (setup wizard)
   - Start/stop/monitor via manifest-driven commands (dashboard)
   - Surface security state (verify results, proxy logs, scan findings)

---

## Numbers At A Glance

| Metric | Value |
|--------|-------|
| Rust backend files | 18 .rs files |
| Tauri commands | 10 (all implemented) |
| Rust unit tests | 14 |
| React components | 26 files |
| React hooks | 6 |
| Output renderers | 6 |
| Frontend unit tests | 22 (not runnable — config missing) |
| E2E tests | 4 (not runnable — config missing) |
| Orchestrator checks | 39 (all passing) |
| Component manifests | 3 |
| Schema enums tracked | 9 (across 3 alignment layers) |
| Vault security checks | 15 (verify.sh) + 12 test scripts |
| Forge malicious patterns | 87 (MITRE ATT&CK mapped) |
| Forge skills | 25 (12,260 lines) |
| Forge test assertions | 168 |
| Pioneer injection patterns | 30 (6 categories) |
| Total lines of docs | ~2,500 across all repos |
