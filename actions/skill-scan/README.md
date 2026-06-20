# OpenTrApp Skill Firewall (GitHub Action)

Scan agent **skills and plugins for malware and prompt injection before an agent loads them**, right
in your CI. This is the standalone, host-runnable half of [OpenTrApp](https://github.com/albertdobmeyer/opentrapp)'s
skill defense (the Vault is the full five-container perimeter; this is the one-line pre-install check).

The scan is **fully offline**: no model and no network. It runs the same engine OpenTrApp runs inside
its perimeter, so there is no separate fork to trust.

The same action is also published as the standalone repository
[`opentrapp/skill-firewall`](https://github.com/opentrapp/skill-firewall), a one-way generated
projection of this engine, so you can reference it with the shorter `uses: opentrapp/skill-firewall@v1`.
Both run identical code; edit and file issues here in the source repo.

## What it checks

- An **87-pattern blocklist** mapped to MITRE ATT&CK, including **16 prompt-injection** patterns.
- A **zero-trust line classifier** that quarantines a skill if a single line is unrecognised, which is
  the defense against the novel attack the pattern set has not seen yet.
- Output as **SARIF**, so findings appear in your repository's **Security tab** (code scanning).

## The honesty boundary (please read)

This Action **reads and pattern-matches text. It does not execute the skill.** Its guarantee is *"vet a
skill before an agent loads it,"* not *"no untrusted content ever touches your runner."* It reads
untrusted text on the runner, which is materially lower risk than executing a skill, but it is not zero.
The stronger "untrusted content is only ever processed inside an isolated container" property belongs to
the full OpenTrApp perimeter (the Vault). See [ADR-0025](https://github.com/albertdobmeyer/opentrapp/blob/main/docs/adr/0025-standalone-skill-firewall-scope.md).

## Usage

```yaml
name: skill-firewall
on: [pull_request]

permissions:
  contents: read
  security-events: write   # only needed to upload findings to the Security tab

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: albertdobmeyer/opentrapp/actions/skill-scan@skill-scan-v1
        with:
          path: ./skills        # a skill dir, a parent of several, or a single SKILL.md
          strict: false
```

If a finding is detected the job fails (the point of a pre-install gate). To report without failing,
set `fail-on-finding: false`. If you do not grant `security-events: write`, set `upload-sarif: false`
and the Action still scans and gates, it just skips the Security-tab upload.

## Inputs

| Input | Default | Description |
|-------|---------|-------------|
| `path` | `.` | What to scan: a skill or plugin directory, a parent of several, or a single `SKILL.md`. |
| `strict` | `false` | Treat suspicious lines as findings, not only the clearly malicious ones. |
| `format` | `summary` | Human-readable format printed to the job log: `default`, `summary`, or `json`. |
| `upload-sarif` | `true` | Upload findings to the Security tab. Needs `security-events: write`. |
| `fail-on-finding` | `true` | Fail the job on a finding. Set `false` to report only. |

## Exit behaviour

The underlying scanner uses a simple contract: `0` clean, `1` a finding, `2` a usage error. A path with
no `SKILL.md` is treated as "nothing to scan" and passes, so the gate does not false-fail on an empty or
restructured path.

## Requirements

A Linux or macOS runner with `bash` and `python3` (both are present on the standard GitHub-hosted
runners). Windows runners are not currently supported because the scanner is a bash toolchain.

## Pre-install hook outside CI

The same scanner runs locally as a one-line gate, for example before installing an agent plugin:

```bash
skill scan "$PLUGIN_DIR" --strict || { echo "blocked by the skill firewall" >&2; exit 1; }
```

MIT, built in public. Issues and reviews welcome at [albertdobmeyer/opentrapp](https://github.com/albertdobmeyer/opentrapp).
