# Monitoring — Runtime Observability

This directory contains tools for monitoring the vault at runtime — what the agent is actually doing inside the perimeter, what it has tried to reach on the network, and whether anything looks anomalous.

## Tools

| Tool | Status | Run with | Purpose |
|------|--------|----------|---------|
| `network-log-parser.py` | Implemented | `make network-report` | Analyze proxy logs for anomalies (blocked requests, exfiltration attempts, key reflections, large payloads, frequency spikes) |
| `session-report.py` | Implemented | `make session-report` | Generate post-session summary (duration, messages, tool calls, commands executed, files accessed, approval outcomes) |

Both tools treat all input as **untrusted data** from inside the container. All string fields are sanitized to prevent terminal injection. See the docstrings in each file for security details.

## Skill Scanning Does NOT Belong Here

Skill scanning is a **supply chain concern**, not a runtime concern. It lives in **openskill-forge** (`tools/skill-scan.sh`), which has a complete 87-pattern scanner with MITRE ATT&CK mapping, zero-trust verification, and SARIF output.

See `docs/trifecta.md` in the opentrapp root for the full ownership matrix.
