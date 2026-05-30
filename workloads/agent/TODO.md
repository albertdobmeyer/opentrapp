# OpenCli-Container — TODO

Tracked gaps. See `docs/roadmap.md` for the phased development plan, and `docs/trifecta.md` in opentrapp root for cross-module strategy.

---

## Monitoring Stubs (Roadmap Phase 2)

The following scripts exist but are placeholder implementations (~15 lines each, print a message and exit):

- [x] `monitoring/network-log-parser.py` — Implemented: anomaly detection on proxy logs (2026-03-27)
- [x] `monitoring/session-report.py` — Implemented: post-session summary generator (2026-03-27)

Skill scanning was removed from this module — it belongs in openskill-forge (`tools/skill-scan.sh`). See `monitoring/README.md`.

---

## VM Isolation Stubs (Phase 9+ — Aspirational)

These scripts exist for future Hyper-V and WSL isolation (beyond containers). Config files are real; scripts are placeholder. Container-based isolation (Phases 1-8) is complete and certified. VM-level isolation is a future enhancement for Windows users who want defense-in-depth beyond containers.

- [ ] `hyperv/create-vm.ps1` — Placeholder
- [ ] `hyperv/provision.ps1` — Placeholder
- [ ] `wsl/wsl-setup.ps1` — Placeholder
- [ ] `wsl/wsl-teardown.ps1` — Placeholder

---

## Resolved

- [x] Tool control system — Per-tool whitelisting/blacklisting with manifest, generator, 23-point verify (2026-03-30)
- [x] rm removed from safeBins — Drift bug fixed, agent is constructive only (2026-03-30)
- [x] askFallback removed — Documented in official docs but rejected by OpenClaw 2026.2.26 Zod schema (2026-03-30)
- [x] Test scripts fixed — 12/12 pass (was 5/12), seccomp/escape/config tests rewritten (2026-03-30)
- [x] `monitoring/skill-scanner.sh` — Removed (2026-03-27). Skill scanning lives in openskill-forge.
- [x] Gear → Shell terminology — Migrated across all docs and configs (2026-03-27).
- [x] `component.yml` config paths — Fixed: `allowlist.txt` → `proxy/allowlist.txt`, `openclaw-hardening.yml` → `config/openclaw-hardening.json5` (2026-03-27).
- [x] `CLAUDE.md` profile claim — Updated from `minimal` to `coding` for Split Shell state (2026-03-27).
- [x] `tests/test-network-isolation.sh` — replaced `wget` with Node.js `http` module (2026-03-23)
- [x] `component.yml` proxy-logs command — fixed container name `openclaw-proxy` → `vault-proxy` (2026-03-23)
- [x] `CLAUDE.md` command table — same proxy container name fix (2026-03-23)
- [x] `proxy/vault-proxy.py` anthropic-version header — made configurable via `ANTHROPIC_API_VERSION` env var (2026-03-23)
- [x] `compose.yml` — passes `ANTHROPIC_API_VERSION` to proxy container with default (2026-03-23)

---

## Minor

- [ ] README screenshot placeholder: `<!-- TODO: capture terminal screenshot -->` — capture and embed actual terminal output of `verify.sh` running
