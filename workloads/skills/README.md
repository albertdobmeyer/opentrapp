# Skill Firewall — supply-chain defense for agent skills

A malicious skill runs as *part of the agent's own reasoning*, so it has to be vetted
**before** it ever reaches the agent. The Skill Firewall does that: offline static analysis
(87 MITRE ATT&CK-mapped patterns + a 16-pattern prompt-injection detector), line-level
zero-trust verification, and — the part that is original work — **Content Disarm &
Reconstruction**: the skill is rebuilt from its *extracted intent* in isolation, so the
original downloaded bytes never reach the agent. It is a clean rebuild or a discard — nothing
in between.

**Runs three ways, lowest commitment first:**
1. **GitHub Action** — one line in CI, fully offline, no model; findings land in your repo's
   Security tab and a finding fails the job. (`actions/skill-scan/` at the repo root.)
2. **Local one-line pre-install gate** — `workloads/skills/skill scan ./that-plugin --strict`
   from a clone, no global install.
3. **The `vault-skills` container** inside OpenTrApp's [five-container
   perimeter](../../docs/perimeter-explained.md) — all untrusted skill content is processed
   off your host.

It doubles as the author's environment for publishing to [ClawHub](https://clawdhub.com);
twenty-five published skills are the corpus the scanner is regression-tested against. This is
`workloads/skills/` in the OpenTrApp monorepo ([ADR-0013](../../docs/adr/0013-monorepo-consolidation.md)).

**Elevator pitch + why CDR-for-skills is original work:**
[`docs/skills-spotlight.md`](../../docs/skills-spotlight.md).

**Author:** [@albertdobmeyer](https://github.com/albertdobmeyer)

---

## Why a static scanner

Generating a skill takes seconds. The gap between *generated* and *production-ready* — which is where supply-chain risk lives — is the problem this toolchain addresses. The pipeline gates each skill through:

- **87 malicious-content patterns across 13 categories**, MITRE ATT&CK-mapped, derived from observed trojanised skills (the ClawHavoc campaign and the `moltbook-ay` trojan).
- **Sixteen prompt-injection detection patterns**, covering instruction override, persona hijacking, stealth commands, exfiltration directives, and LLM control-token injection.
- **Multi-file scanning** across `.md`, `.sh`, `.py`, `.js`, `.ts`, `.yaml`, `.yml`, `.json`. Many trojanised skills hid the payload outside `SKILL.md`.
- **Strict mode** (`make scan-strict`) blocks `HIGH`-severity findings in addition to `CRITICAL`. Defends against credential theft, persistence, and container-escape patterns that fall short of `CRITICAL` thresholds.
- **Post-install quarantine** — when `ALLOW_INSTALL=1` is used, the scanner re-runs on newly installed skills; failures are quarantined.
- **Suppression audit** — `.scanignore` ranges greater than 50 lines are rejected, preventing blanket suppression of large file regions.
- **Behavioural assertions** — 168+ assertions enforce structural and content consistency across all included skills.
- **Mandatory test gate** — every skill must have a test file before it can be published.
- **Gated publishing** — `make publish` will not run unless lint, scan, and test all pass.
- **SARIF output** — `make scan-sarif` produces SARIF 2.1.0 for GitHub code-scanning integration.
- **Zero-trust line verifier** — `make verify-all` classifies every line in every file. A single unrecognised line quarantines the entire skill. Detects novel attacks the static blocklist does not cover.
- **Trend tracking** — `make stats-trend` and `make stats-rank` report adoption metrics over time.

`make report` produces a concrete summary of the gates exercised by the pipeline.

---

## Operator commands

```bash
make verify                              # 12-point workbench health check
make report                              # Pipeline summary
make scan-strict                         # Scan with HIGH-severity blocking
make verify-all                          # Zero-trust verify all skills
make verify-skill SKILL=docker-sandbox   # Verify a single skill
make verify-report SKILL=docker-sandbox  # Per-line verdict report
make test-tools                          # Tool behavioural tests
make check-all                           # Full pipeline + self-test + tool tests
make explore                             # Top 20 skills on ClawHub by downloads
make explore QUERY="docker"              # Semantic search
make explore SORT=trending               # Currently trending
make stats                               # Adoption metrics for the included skills
make stats-trend                         # Growth deltas vs previous snapshots
make stats-rank                          # Included skills ranked against ClawHub top 50
```

---

## Limitations

The toolchain is transparent about what it cannot provide:

- **Installer identity** — the ClawHub API does not expose installer identities or per-user usage data. Download counts are the best signal.
- **Bot-versus-human attribution** — there is no way to distinguish human installs from automated agent installs.
- **No web dashboard** — this is a CLI-first toolchain; terminal output and machine-readable summaries are the supported interfaces.
- **No dependency resolution** — ClawHub treats skills as standalone; no automatic dependency installation.
- **No automatic version bumping** — version is always an explicit `VERSION=x.y.z` parameter, requiring deliberate operator action.

---

## Quick start

### Dev container (recommended)

Open this repository in VS Code with the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers). The dev container installs Node.js, Python, `molthub`, and required dependencies automatically.

### Local

Requirements: bash, git, python3 (for YAML and JSON validation). Optional: `molthub` (`npm install -g molthub`).

```bash
make help                                # All available commands
make new SKILL=my-tool                   # Scaffold a skill from template
make lint                                # Lint all skills
make scan                                # Security-scan all skills
make test                                # Run behavioural tests
make check                               # Full pipeline: lint + scan + test
```

---

## Pipeline stages

### Linter (`make lint`)

Verifies skill quality. Checks frontmatter completeness (`name`, `description`, `metadata`; valid slug; description length; valid JSON metadata), structural elements (H1 title, `## When to Use`, `## Tips`), content quality (line count between 150 and 700, ≥8 code blocks, language tags on fences, absence of `TODO`/`FIXME`/`XXX` placeholders), and metadata consistency (every binary in `requires.anyBins` referenced in the body).

### Scanner (`make scan`)

Offline. Network access is not required at scan time. Operates on the entire skill directory, not just `SKILL.md`. Pattern catalogue:

| Category | Severity | Coverage |
|----------|----------|----------|
| C2 / download | CRITICAL | curl, wget, fetch to external URLs |
| Archive execution | CRITICAL | Password-protected ZIP / 7z extraction |
| Exec download | CRITICAL | chmod-then-execute, `bash -c` with curl, eval with subshell |
| Credential access | HIGH | `.env`, `.ssh` keys, AWS/Kubernetes credentials, `/proc/environ`, PEM files |
| Data exfiltration | CRITICAL | `curl POST`, netcat, DNS exfiltration, `scp`, `git push`, FTP to literal IPs |
| Obfuscation | HIGH | Base64/hex-decode-to-shell, Python/Perl/Ruby `eval`, OpenSSL decrypt |
| Persistence | HIGH | crontab, `~/.bashrc`/`.profile`/`.zshrc`/fish, `at now`, `launchctl` |
| Privilege escalation | MEDIUM–HIGH | `sudo chmod 777`, setuid, `sudo su`, `nsenter` |
| Container escape | HIGH | `--privileged`, `SYS_ADMIN`, host-mount, docker.sock, `sysrq` |
| Supply chain | MEDIUM | Unsafe `npm install`, `pip --pre`, registry hijack |
| Environment injection | MEDIUM | `LD_PRELOAD`, PATH manipulation, `env -i` |
| Resource abuse | HIGH | Fork bomb, infinite loop with network |
| Prompt injection | HIGH–CRITICAL | Override attempts, persona hijacking, stealth instructions, exfiltration directives, LLM control-token injection |

Output modes:

| Command | Output |
|---------|--------|
| `make scan` | Coloured terminal output |
| `make scan-summary` | One line per skill |
| `make scan-json` | Structured JSON |
| `make scan-sarif` | SARIF 2.1.0 for GitHub code scanning |
| `make scan-strict` | HIGH severity blocks in addition to CRITICAL |
| `make self-test` | Validates the scanner against known-bad and known-clean fixtures |

Skills that legitimately reference malicious patterns (e.g. `security-audit`) may use inline `<!-- scan:ignore -->` markers or a `.scanignore` file. Suppressions are audited; ranges greater than 50 lines are rejected.

### Zero-trust verifier (`make verify-skill SKILL=name`)

The scanner uses a blocklist (search for known-bad, allow everything else). The verifier inverts this: every line in every file must classify as `SAFE`, otherwise the entire skill is quarantined. No partial passes; no thresholds.

| Verdict | Meaning |
|---|---|
| `SAFE` | Matches a known-safe pattern (structural Markdown, prose under 500 characters, code inside fenced blocks, frontmatter fields) |
| `SUSPICIOUS` | Does not match any safe pattern (possible obfuscation, unknown encoding, excessively long content) |
| `MALICIOUS` | Triggers the 87-pattern blocklist |

A skill is released from quarantine only if it has zero malicious lines and zero suspicious lines.

**Two-stage post-install defence:** newly installed skills pass through `skill-scan.sh --strict` (blocklist, fast) and then `skill-verify.sh --strict` (allowlist, thorough). Both must pass.

**Trust manifests:** included skills can carry `.trust` files containing SHA-256 content hashes, allowing them to skip verification when unchanged. External skills do not carry trust manifests.

### Test framework (`make test`)

Behavioural assertions for skills, equivalent in role to a unit-test framework for `SKILL.md` files:

```bash
assert_section_exists "$SKILL" "When to Use"
assert_contains "$SKILL" "docker\s+(run|build|exec)"
assert_not_contains "$SKILL" "(TODO|FIXME|XXX)"
assert_min_code_blocks "$SKILL" 8
assert_frontmatter_field "$SKILL" "name" "^docker-sandbox$"
```

Tests live at `tests/<skill-name>.test.sh` with `test_*` functions. The runner discovers and executes them automatically.

### Publisher (`make publish`)

Gated pipeline. Lint, scan, and test must all pass before `molthub publish` is invoked.

```bash
make publish SKILL=my-tool VERSION=1.0.0
```

---

## Containerised deployment

In production, the toolchain runs inside the `vault-skills` container of the OpenTrApp five-container perimeter. All untrusted content (downloaded skills) is processed inside the container and never reaches the host filesystem.

- The `Containerfile` in this repository's root defines the image (~72 MB, `python:3.10-alpine` plus the bash toolchain; WS-B shrank it from the former 233 MB `python:3.10-slim`).
- `vault-skills` is one of five services in `compose.yml` at the opentrapp root (see [ADR-0009](../../docs/adr/0009-five-container-perimeter.md) for the topology).
- It runs on `skills-net`, an internal network. It can reach `vault-proxy` for outbound HTTPS but cannot reach `vault-agent` or `vault-social` directly.
- Certified skills are delivered to the agent through the `skills-deliveries` shared volume, which is writable in `vault-skills` and read-only in `vault-agent`.
- Non-root user, capabilities dropped, 1 GB memory limit, custom seccomp profile.

The CLI/Makefile usage documented above remains the supported interface for development. The `Containerfile` copies this repository into the image and runs the same bash toolchain.

---

## Included skills

Twenty-five reference skills, all passing the full pipeline:

| Skill | Install | Description |
|---|---|---|
| [api-dev](skills/api-dev/SKILL.md) | `molthub install api-dev` | curl testing, bash and Python test runners, OpenAPI-spec generation, mock servers, Express scaffolding |
| [cicd-pipeline](skills/cicd-pipeline/SKILL.md) | `molthub install cicd-pipeline` | GitHub Actions for Node, Python, Go, Rust; matrix builds, caching, Docker build-and-push, secrets management |
| [coding-agent](skills/coding-agent/SKILL.md) | `molthub install coding-agent` | Reference patterns for an autonomous coding-agent system prompt and toolset |
| [container-debug](skills/container-debug/SKILL.md) | `molthub install container-debug` | Container logs, exec, networking diagnostics, resource inspection, multi-stage build debugging, health checks, Compose |
| [cron-scheduling](skills/cron-scheduling/SKILL.md) | `molthub install cron-scheduling` | Cron syntax, systemd timers, one-off jobs, timezone and DST handling, monitoring, locking, idempotent patterns |
| [csv-pipeline](skills/csv-pipeline/SKILL.md) | `molthub install csv-pipeline` | CSV/JSON/TSV processing — filter, join, aggregate, deduplicate, validate, convert |
| [data-validation](skills/data-validation/SKILL.md) | `molthub install data-validation` | JSON Schema, Zod, Pydantic, CSV and JSON integrity checks, migration validation |
| [dns-networking](skills/dns-networking/SKILL.md) | `molthub install dns-networking` | DNS debugging (`dig`, `nslookup`), port testing, firewall rules, curl diagnostics, proxy configuration, certificates |
| [docker-sandbox](skills/docker-sandbox/SKILL.md) | `molthub install docker-sandbox` | Container sandbox VM management, network proxy, workspace mounting, troubleshooting |
| [emergency-rescue](skills/emergency-rescue/SKILL.md) | `molthub install emergency-rescue` | Recovery procedures: git disasters, credential leaks, disk full, OOM kills, database failures, deploy rollback, SSH lockout, network outage |
| [encoding-formats](skills/encoding-formats/SKILL.md) | `molthub install encoding-formats` | Base64, URL encoding, hex, Unicode, JWT decoding, hashing, serialization-format conversion |
| [git-workflows](skills/git-workflows/SKILL.md) | `molthub install git-workflows` | Interactive rebase, bisect, worktree, reflog recovery, cherry-pick, subtree/submodule, sparse checkout, conflict resolution |
| [infra-as-code](skills/infra-as-code/SKILL.md) | `molthub install infra-as-code` | Terraform, CloudFormation, Pulumi — VPC, compute, storage, state management, multi-environment patterns |
| [log-analyzer](skills/log-analyzer/SKILL.md) | `molthub install log-analyzer` | Log parsing, error patterns, stack-trace extraction, structured logging, real-time monitoring, correlation |
| [makefile-build](skills/makefile-build/SKILL.md) | `molthub install makefile-build` | Make targets, pattern rules, language-specific Makefiles, modern alternatives (Just, Task) |
| [perf-profiler](skills/perf-profiler/SKILL.md) | `molthub install perf-profiler` | CPU and memory profiling, flame graphs, benchmarking, load testing, leak detection, query optimisation |
| [regex-patterns](skills/regex-patterns/SKILL.md) | `molthub install regex-patterns` | Validation patterns, parsing, extraction across JS/Python/Go/grep, search-and-replace, lookahead/lookbehind |
| [security-audit](skills/security-audit/SKILL.md) | `molthub install security-audit-toolkit` | Dependency scanning, secret detection, OWASP patterns, SSL/TLS verification, file permissions, audit scripts |
| [shell-scripting](skills/shell-scripting/SKILL.md) | `molthub install shell-scripting` | Argument parsing, error handling, trap and cleanup, temp files, parallel execution, portability, config parsing |
| [skill-reviewer](skills/skill-reviewer/SKILL.md) | `molthub install skill-reviewer` | Quality audit framework — rubric, defect checklists, structural/content/actionability review |
| [skill-search-optimizer](skills/skill-search-optimizer/SKILL.md) | `molthub install skill-search-optimizer` | Registry discoverability — semantic-search mechanics, description optimisation, visibility testing |
| [skill-writer](skills/skill-writer/SKILL.md) | `molthub install skill-writer` | `SKILL.md` authoring guide — format spec, frontmatter schema, content patterns, templates |
| [sql-toolkit](skills/sql-toolkit/SKILL.md) | `molthub install sql-toolkit` | SQLite, PostgreSQL, MySQL — schema design, queries, CTEs, window functions, migrations, EXPLAIN, indexing |
| [ssh-tunnel](skills/ssh-tunnel/SKILL.md) | `molthub install ssh-tunnel` | Local/remote/dynamic port forwarding, jump hosts, SSH config, key management, scp/rsync, debugging |
| [test-patterns](skills/test-patterns/SKILL.md) | `molthub install test-patterns` | Jest/Vitest, pytest, Go, Rust, bash — unit tests, mocking, fixtures, coverage, TDD, integration testing |

---

<details>
<summary><strong>Research and threat-landscape findings</strong></summary>

The toolchain's design was informed by ecosystem analysis. The following research artifacts are preserved in `docs/`:

- **Trojanised skill discovery** — the `moltbook-ay` skill contained instructions to download and execute malware via password-protected archives. Classic social engineering adapted for autonomous agents. No code was executed; the `molthub install` process was [verified from source](docs/journey.md#phase-11-security-audit) to be download-extract-write only.
- **ClawHub platform analysis** — API reverse-engineering, registry discovery protocol, skill format schema, publishing flow, semantic-search mechanics, and registry statistics. Full report: [`docs/research/clawdhub-platform-report.md`](docs/research/clawdhub-platform-report.md).
- **Security compilation** — Willison's "lethal trifecta" framework, CVE-2026-25253 (one-click RCE), the ClawHavoc supply-chain campaign (341 malicious skills), the Moltbook database breach, and 21,639 publicly-exposed instances. Full analysis: [`docs/research/security-report.md`](docs/research/security-report.md).
- **End-to-end narrative** — From package vetting to twenty-five published skills, ecosystem retraction, and lessons learned: [`docs/journey.md`](docs/journey.md).

</details>

---

## Repository structure

```
openagent-skills/
├── Makefile                       single entry point for all commands (~35 targets)
├── component.yml                  OpenTrApp manifest contract
├── Containerfile                  vault-skills container image
├── skills/                        25 reference skills
├── tools/
│   ├── lib/
│   │   ├── common.sh              colours, logging, skill discovery
│   │   ├── frontmatter.sh         YAML frontmatter parser and validator
│   │   ├── patterns.sh            87 malicious-content patterns
│   │   ├── line-classifier.sh     SAFE / SUSPICIOUS / MALICIOUS classifier
│   │   ├── trust-manifest.sh      `.trust` file generation and SHA-256 validation
│   │   └── sarif_formatter.py     SARIF 2.1.0 output formatter
│   ├── skill-lint.sh              linter
│   ├── skill-scan.sh              static scanner
│   ├── skill-verify.sh            zero-trust line verifier
│   ├── skill-test.sh              test runner
│   ├── skill-new.sh               scaffolder
│   ├── skill-publish.sh           gated publisher
│   ├── skill-stats.sh             adoption metrics
│   ├── registry-explore.sh        registry browsing
│   ├── workbench-verify.sh        12-point health check
│   └── pipeline-report.sh         pipeline summary
├── templates/                     skill templates
├── tests/
│   ├── _framework/                test runner and assertions
│   ├── scanner-self-test/         scanner accuracy fixtures
│   └── *.test.sh                  25 test files
├── .github/workflows/
│   └── skill-ci.yml               CI: lint, scan, test on every PR
└── docs/                          research, setup, journey
```

## Skill format

Each skill is a `SKILL.md` file with YAML frontmatter that informs an AI agent of when and how to use it:

```yaml
---
name: my-skill
description: When to activate this skill
metadata: {"clawdbot":{"emoji":"...","requires":{"anyBins":["tool1","tool2"]}}}
---

# Skill Title

Reference material, patterns, commands, and examples that the agent
follows to perform the task.
```

Skills install via `molthub install <slug>` and are placed at `./skills/<slug>/`; the agent loads them on demand.

---

## Sibling workloads in the monorepo

Other directories at the same level since the v0.5.0 consolidation
([ADR-0013](../../docs/adr/0013-monorepo-consolidation.md)):

- [`workloads/agent/`](../agent/) — runtime containment for the agent
  (the `vault-agent` container). Hardened container, proxy-side API-key
  injection, domain allowlist, three-level kill switch, 24-point
  verification.
- [`workloads/social/`](../social/) — agent-social-feed analysis (the
  `vault-social` container). **Parked since 2026-05-03** following Meta's
  acquisition of Moltbook; re-aim to a generalised agent-to-agent shield
  is tracked in `MISSION.md` Thread C.
- [`infra/proxy/`](../../infra/proxy/) and [`infra/egress/`](../../infra/egress/) —
  the L7 and L3 egress chain that the agent + forge + social all share.

## License

Skills are published to ClawHub under the registry's terms. Source files in this repository are licensed under [MIT](LICENSE).
