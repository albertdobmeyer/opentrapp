# Reproducing every numerical claim in this repository

**Document status:** Active
**Created:** 2026-05-04
**Companion script:** [`docs/reproduce.sh`](reproduce.sh) runs the key verification commands in this document.

This document lists every numerical claim made in the project's user-facing surfaces (the [`README.md`](../README.md), [`docs/whitepaper.md`](whitepaper.md), and [`CLAUDE.md`](../CLAUDE.md)) and gives the exact command sequence to verify it independently. The goal is that a reader running [`docs/reproduce.sh`](reproduce.sh) from a fresh `git clone --recurse-submodules` arrives at the same numbers within roughly ten minutes.

Each claim is presented as a row with: the claim, the command that verifies it, the expected output (or a tight range), an upper-bound runtime, and the source file the claim appears in. Rows whose verification depends on running the perimeter (which requires Podman or Docker) are flagged accordingly; rows whose verification is offline-only run in any environment.

The document is structured in three groups: counted artefacts (1 to 8), test-suite counts (9 to 13), and external claims (14 to 16) where the claim is supported by an external study and we link to that study rather than re-deriving it.

---

## Conventions

- **Working directory:** every command assumes a fresh clone at `opentrapp/` and starts from that directory unless stated otherwise.
- **Recursive clone required:** several rows cite files inside submodules; clone with `--recurse-submodules` (see [`CONTRIBUTING.md`](../CONTRIBUTING.md) "Cloning the repository").
- **Runtime ceilings:** "≤ N s" is the upper bound on the maintainer's dev machine (Lenovo ideapad 320, AMD A12-9720P, 7.2 GB RAM). Faster machines arrive sooner.
- **Test-count drift:** test counts grow as features are added, so the rows below do not pin an exact total. Each row describes the *measurement command* and a *floor*: running the suite at any tagged version should produce a count at or above the floor. The exact tag-time count is in [`docs/handoff.md`](handoff.md).
- **`git lfs` / signed assets:** none of the rows below depend on LFS objects or signed assets being downloaded.

---

## Group 1: counted artefacts (offline)

### 1. Eighty-seven malicious-skill patterns in `vault-skills`

| Field | Value |
|-------|-------|
| Claim | The skill scanner contains 87 patterns mapped to MITRE ATT&CK techniques |
| Source | [`README.md`](../README.md) "Capabilities", [`docs/whitepaper.md`](whitepaper.md) §6, [`docs/trifecta.md`](trifecta.md) §4.2 |
| Command | `grep -cE "^\s*'(CRITICAL\|HIGH\|MEDIUM)\|" workloads/skills/tools/lib/patterns.sh` |
| Expected output | `87` |
| Runtime | < 1 s |

The `SCAN_PATTERNS` array in `patterns.sh` is the single source of truth for the catalogue. Each line carries severity, category, regex, description, and the corresponding MITRE-ATT&CK technique ID. The `grep` above counts only entries that begin with one of the three severity levels; comments and array-syntax lines are excluded.

To inspect the category breakdown:

```bash
grep -oE "^\s*'(CRITICAL|HIGH|MEDIUM)\|[a-z_]+" workloads/skills/tools/lib/patterns.sh \
  | awk -F'|' '{print $2}' | sort | uniq -c | sort -rn
```

### 2. Twenty-four startup-verification checks

| Field | Value |
|-------|-------|
| Claim | A 24-point verification runs at container startup and on demand |
| Source | [`README.md`](../README.md) "Capabilities", [`docs/whitepaper.md`](whitepaper.md) §8 |
| Command | `grep -cE '^check [0-9]+ "' workloads/agent/scripts/verify.sh` plus the inline check-24 row |
| Expected output | `24` (combined; see `docs/reproduce.sh` for the exact recipe) |
| Runtime | < 1 s |

Checks 1 to 23 are invoked through the `check N "<title>"` helper; check 24 (configuration-integrity hash) is implemented inline because it needs the hash-comparison logic in-context. The 24 checks fall into three groups: 14 universal-hardening checks, 4 shell-specific checks, and 6 per-tool security checks. The grouping is documented in [`docs/whitepaper.md`](whitepaper.md) §8.

To list the 23 helper-invoked check titles:

```bash
grep -E '^check [0-9]+ "' workloads/agent/scripts/verify.sh
```

### 3. One hundred twenty orchestration checks

| Field | Value |
|-------|-------|
| Claim | A 114-check manifest-orchestration suite runs on every commit and reports zero warnings in the released configuration |
| Source | [`README.md`](../README.md) "Test suite", [`docs/whitepaper.md`](whitepaper.md) §8, [`CLAUDE.md`](../CLAUDE.md) §7 |
| Command | `bash tests/orchestrator-check.sh` |
| Expected output | Last line: `Results: 114 passed, 0 failed, 0 warnings (total: 114 checks)` |
| Runtime | ≤ 30 s |

The script needs `python3` (with `pyyaml` installed) and `node` available. The `Results:` line is what the README cites; rows above it are the per-check pass/fail listing.

### 4. Twenty-eight reserved terms in the user-facing surface

| Field | Value |
|-------|-------|
| Claim | The user-facing surface enforces a 28-term reserved-word list |
| Source | [`README.md`](../README.md) (implicit), [`CLAUDE.md`](../CLAUDE.md) §3, [`CONTRIBUTING.md`](../CONTRIBUTING.md) "The 28 reserved-term list" |
| Command | `awk '/const BANNED_TERMS = \[/,/^\];/' app/e2e/user-facing.spec.ts \| grep -cE '^\s*"[^"]+",?$'` |
| Expected output | `28` |
| Runtime | < 1 s |

The reserved-term array is defined at the top of `app/e2e/user-facing.spec.ts`. The Playwright test reads every user-mode page's visible text and asserts that none of the listed terms appear. The full list is small enough to print:

```bash
awk '/const BANNED_TERMS = \[/,/^\];/' app/e2e/user-facing.spec.ts | grep -oE '"[^"]+"'
```

### 5. Five containers in the perimeter

| Field | Value |
|-------|-------|
| Claim | The runtime perimeter is composed of five containers with an L7/L3 policy split |
| Source | Multiple: [`README.md`](../README.md), [`docs/whitepaper.md`](whitepaper.md), [`docs/trifecta.md`](trifecta.md), [`adr/0009-five-container-perimeter.md`](adr/0009-five-container-perimeter.md) |
| Command | `python3 -c "import yaml; print(len(yaml.safe_load(open('compose.yml'))['services']))"` |
| Expected output | `5` |
| Runtime | < 1 s |

The five services are `vault-agent`, `vault-skills`, `vault-social`, `vault-proxy` (L7 policy), `vault-egress` (L3 policy + pinned DoT resolver). The social container is opt-in / off by default (a live AT Protocol adapter shipped under ADR-0017; full build-out deferred) but its definition remains in the compose file (see [`docs/whitepaper.md`](whitepaper.md) §3.2). The L7/L3 split between vault-proxy and vault-egress is enforced by `tests/orchestrator-check.sh` §10 (no container holds both API keys and `NET_ADMIN`).

### 6. Three trust tiers

| Field | Value |
|-------|-------|
| Claim | The architecture organises components into three trust tiers |
| Source | [`docs/trifecta.md`](trifecta.md) §2, [`docs/whitepaper.md`](whitepaper.md) §3.1 |
| Command | `grep -cE "^TIER " docs/trifecta.md` |
| Expected output | `3` |
| Runtime | < 1 s |

The tier names appear in the architecture diagram in `docs/trifecta.md` §2 ("TIER 1: TRUSTED", "TIER 2: INFRASTRUCTURE", "TIER 3: CONTAINED").

### 7. Three shell levels

| Field | Value |
|-------|-------|
| Claim | The agent's privilege model defines three shell levels (Hard, Split, Soft) |
| Source | [`docs/trifecta.md`](trifecta.md) §5, [`docs/whitepaper.md`](whitepaper.md) §5, [`docs/adr/0002-adaptive-shell-levels.md`](adr/0002-adaptive-shell-levels.md) |
| Command | `grep -E "^\| (Hard\|Split\|Soft) Shell" docs/trifecta.md \| wc -l` |
| Expected output | `3` |
| Runtime | < 1 s |

### 8. Six attacker categories in the threat model

| Field | Value |
|-------|-------|
| Claim | The threat model enumerates six attacker categories (T1 to T6) |
| Source | [`docs/threat-model.md`](threat-model.md) |
| Command | `grep -cE "^## T[1-6]:" docs/threat-model.md` |
| Expected output | `6` |
| Runtime | < 1 s |

---

## Group 2: test-suite counts (offline)

### 9. Rust unit-test count

| Field | Value |
|-------|-------|
| Claim | The Rust unit-test suite passes with a floor of ≥ 56 tests (counts grow with later versions) |
| Source | [`README.md`](../README.md) "Test suite", [`docs/whitepaper.md`](whitepaper.md) §8, [`CLAUDE.md`](../CLAUDE.md) §7 |
| Command | `cd app/src-tauri && cargo test --lib 2>&1 \| tail -3` |
| Expected output | A line containing `test result: ok. N passed; 0 failed; 0 ignored` with `N ≥ 56` |
| Runtime | ≤ 4 min on a cold Cargo cache, ≤ 30 s on a warm one |

### 10. Vitest frontend unit-test count

| Field | Value |
|-------|-------|
| Claim | The Vitest frontend suite passes with a floor of ≥ 74 tests (the current count is well above this and grows with later versions) |
| Source | [`README.md`](../README.md) "Test suite", [`CLAUDE.md`](../CLAUDE.md) §7, [`whitepaper.md`](whitepaper.md) §8 |
| Command | `cd app && npm test -- --run 2>&1 \| tail -3` |
| Expected output | A line containing `Tests  N passed (N)` with `N ≥ 74` |
| Runtime | ≤ 30 s |

### 11. Playwright end-to-end count

| Field | Value |
|-------|-------|
| Claim | The Playwright end-to-end suite passes with a floor of ≥ 25 browser tests (counts grow with later versions) |
| Source | [`README.md`](../README.md) "Test suite", [`docs/whitepaper.md`](whitepaper.md) §8 |
| Command | `cd app && npx playwright test 2>&1 \| tail -3` |
| Expected output | A line containing `N passed` with `N ≥ 25` |
| Runtime | ≤ 90 s; first run downloads ~150 MB of Chromium |

### 12. TypeScript strict-mode pass

| Field | Value |
|-------|-------|
| Claim | TypeScript strict mode passes with zero errors |
| Source | [`README.md`](../README.md) "Test suite" |
| Command | `cd app && npx tsc --noEmit; echo "exit=$?"` |
| Expected output | `exit=0` and no error lines |
| Runtime | ≤ 20 s |

### 13. Orchestrator-check warnings

| Field | Value |
|-------|-------|
| Claim | The 114-check orchestrator suite reports zero warnings |
| Source | [`README.md`](../README.md), [`docs/whitepaper.md`](whitepaper.md) §8, [`docs/handoff.md`](handoff.md) |
| Command | `bash tests/orchestrator-check.sh 2>&1 \| grep -E "Results:"` |
| Expected output | `Results: 114 passed, 0 failed, 0 warnings (total: 114 checks)` |
| Runtime | ≤ 30 s |

---

## Group 3: external claims (cited, not re-derived)

These are claims supported by external studies or vendor-disclosed events. We do not re-derive them; we cite the original source.

### 14. The 11.9 % malicious-skill rate (ClawHavoc study)

| Field | Value |
|-------|-------|
| Claim | The ClawHavoc study (2026-Q1) classified 341 of 2,857 published ClawHub skills (11.9 %) as malicious |
| Source | [`README.md`](../README.md) "Purpose", [`docs/whitepaper.md`](whitepaper.md) §1, [`docs/trifecta.md`](trifecta.md) §1 |
| Verification | The study's methodology and per-skill classification dataset are documented in `workloads/agent/docs/research/` (companion repository, agent workload) |
| Reproduction | The study's methodology can be re-run on the current ClawHub corpus by following `workloads/agent/docs/research/clawhavoc-methodology.md`. We do not re-run it as part of `reproduce.sh`. |

The figure is intentionally a snapshot of the registry as of 2026-Q1. A current re-run would produce a different (likely lower, as ClawHub has since added moderation) percentage; the architectural assumption (that *every* incoming skill is potentially hostile) does not depend on the precise rate.

### 15. CVE-2026-25253 (OpenClaw management API RCE)

| Field | Value |
|-------|-------|
| Claim | A one-click remote-code-execution path through OpenClaw's management API |
| Source | [`docs/whitepaper.md`](whitepaper.md) §1, [`docs/adr/0001-proxy-side-api-key-injection.md`](adr/0001-proxy-side-api-key-injection.md) |
| Verification | The CVE record at the National Vulnerability Database (NVD) and the upstream OpenClaw advisory are the authoritative sources. |
| Reproduction | Not applicable; the vulnerability has been patched upstream, and the architectural lesson (do not co-locate credential and runtime) is encoded in the proxy-side credential-injection ADR. |

### 16. The Moltbook database breach (2026-01)

| Field | Value |
|-------|-------|
| Claim | 1.5 M API tokens, 35 K e-mail addresses, and direct messages exposed via Supabase row-level-security misconfiguration |
| Source | [`docs/whitepaper.md`](whitepaper.md) §1, [`docs/adr/0001-proxy-side-api-key-injection.md`](adr/0001-proxy-side-api-key-injection.md) |
| Verification | The breach was disclosed on `haveibeenpwned.com` and covered in security press at the time. |
| Reproduction | Not applicable. |

---

## Optional: running the perimeter (online, requires Podman or Docker)

The perimeter smoke test is part of `reproduce.sh` only when the `RUN_PERIMETER=1` environment variable is set, because it requires container tooling that is not always available on a documentation-review machine.

```bash
# Bring the perimeter up
podman compose up -d

# All five services should be in "running" state
podman compose ps

# Full per-container 24-point verification
podman exec vault-agent /vault/scripts/verify.sh

# Tear down
podman compose down
```

The smoke test does not exchange real credentials; the proxy reads its environment from `.env.test` if `.env` is absent, which contains placeholder values. No real API calls are issued.

---

## Limitations of this document

The list above does *not* cover:

- **Build reproducibility** in the strict-cryptographic sense (same source → same artefact bytes). That is the SLSA work tracked in [`roadmap-post-launch.md`](roadmap-post-launch.md) §4 second half (cosign signing + provenance attestation in CI). When that lands, this document will gain a section pointing readers at `cosign verify` and the SLSA `intoto.jsonl` evidence.
- **Performance claims** about the runtime (memory, latency, CPU). The whitepaper makes no specific performance claim; if it later does, those become reproducibility rows here.
- **Subjective claims** like "polished", "academic-tone", "delightful": these are evaluated by a human reviewer using the rubric in [`docs/specs/2026-04-20-ux-principles-rubric.md`](specs/2026-04-20-ux-principles-rubric.md), not by a deterministic command. The Pass-8 audit ([`docs/specs/2026-05-02-pass-8-preship-walk.md`](specs/2026-05-02-pass-8-preship-walk.md)) is the corresponding evaluation artefact.

---

## Cross-references

- [`README.md`](../README.md) "Test suite": points readers at this document for verification.
- [`docs/whitepaper.md`](whitepaper.md) §8: the empirical-evaluation section that this document operationalises.
- [`docs/handoff.md`](handoff.md): the working-state snapshot that names the current tag-time test counts.
- [`docs/roadmap-post-launch.md`](roadmap-post-launch.md) §4: the planned SLSA / SBOM / cosign work that will extend this document.
