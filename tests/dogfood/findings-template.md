# Dogfood Full Arc — Findings (template)

**Session date:** _2026-05-DD_
**Run by:** _operator name_ + Claude (session ID)
**Build:** `git rev-parse HEAD` →
**Spec:** [`docs/specs/2026-05-05-dogfood-full-arc-spec.md`](../../docs/specs/2026-05-05-dogfood-full-arc-spec.md)

When populated, save this as `docs/specs/2026-05-DD-dogfood-full-arc-findings.md`.

---

## §0 — Pre-flight snapshot

`verify.sh` at session start:

```
[paste output of: podman exec vault-agent /vault/scripts/verify.sh]
```

Test bot handle: `@…`
Anthropic key prefix (last 4 of `sk-ant-…`): `…`
Spending cap: $1.00

---

## §A — Tier A (happy path)

| # | Latency to first byte (s) | Pass? | Banned-term hits | One-line verdict |
|---|---:|:--:|---|---|
| A1 | | | | |
| A2 | | | | |
| A3 | | | | |
| A4 | | | | |
| A5 | | | | |

### A1 — meeting action items
**Reply text (or summary):**
**Out-of-band check:**
**Bot voice — score (1–10):**
**Karen-level: would she be helped?** _yes / no / partial_
**Friction notes:**

### A2 — landlord email
**Reply text (or summary):**
**Out-of-band check:** `landlord-email.txt` in workspace? _yes / no_; usable as a draft? _yes / no_
**Bot voice — score (1–10):**
**Friction notes:**

### A3 — paella scaling
**Which path?** _(a) recipe + scale / (b) honest "I'd need Soft Shell"_
**Reply text (or summary):**
**Bot voice — score (1–10):**
**Friction notes:**

### A4 — KEYSTONE: install CSV-formatting skill from ClawHub
**Reply text (or summary):**
**Out-of-band suite:**
- forge artefact present: _yes / no_
- agent rebuilt artefact present: _yes / no_
- `.trust` file matches clearance-report SHA-256: _yes / no_
- clearance-report `scan.status`: _PASS / FAIL / OTHER_
- proxy log shows ALLOWED to `clawhub.ai` only: _yes / no_
- agent confirmed loaded in chat: _yes / no_

**Total time end-to-end:** _… min_
**Verdict:** _ship-quality / has-bugs / broken_
**Friction notes:**

### A5 — use installed skill
**Reply text (or summary):**
**Cleaned CSV in workspace:** _yes / no_
**Cleaned CSV correct:** _yes / no / partial_
**Friction notes:**

### Tier A close-out
**Karen sign-off question** — _Would Karen show this to a friend?_

---

## §B — Tier B (adversarial)

| # | Defence layer | Defence held? | Bot reply summary | Banned-term hits |
|---|---|:--:|---|---|
| B1 | proxy-side credential injection | | | |
| B2 | workspace restriction | | | |
| B3 | exec safelist + proxy allowlist | | | |
| B4 | indirect prompt injection | | | |
| B5 | forge scanner + line classifier + CDR | | | |
| B6 | DM pairing policy | | | |
| B7 | adaptive-shell self-promote refusal | | | |
| B8 | 28-term banned-word audit | | | |

### Per-scenario notes
*(Add a paragraph per scenario — what attack pattern was used, what defence fired, was the user-facing copy clear, anything the harness didn't catch.)*

---

## §C — Tier C (AssistantStatus state coverage)

| # | State | Hero-card copy verbatim | Banned-term hits | Screenshot file |
|---|---|---|---|---|
| C1 | `not_setup` | | | |
| C2 | `starting` | | | |
| C3 | `recovering` | | | |
| C4 | `ok` | | | |
| C5 | `error_perimeter` | | | |
| C6 | `error_key` | | | |
| C7 | `paused_by_user` | | | |

### Notes
- Did any state cause the user to feel anxious / blamed?
- Did any state require developer-jargon to understand?
- Did the marker file `~/.lobster-trapp/paused` survive C7's app-restart?

---

## §D — Tier D (termination-path coverage)

| # | Path | Containers down within 30s? | Orphans? | RunGuard reaped? (D5 only) |
|---|---|:--:|:--:|:--:|
| D1 | window close | | | n/a |
| D2 | tray Quit | | | n/a |
| D3 | SIGTERM | | | n/a |
| D4 | SIGINT | | | n/a |
| D5 | SIGKILL | n/a | yes (expected) | |
| D6 | reboot simulation | | | n/a |
| D7 | pause + close + relaunch | n/a | n/a | (paused state survived?) |

### Notes
- Did the GUI show a clear "shutting down…" indicator on D1/D2?
- Did SIGTERM/SIGINT respect the documented 30-second ceiling?
- Was the RunGuard reap on D5 visible to the user, or invisible?

---

## §E — Cross-cutting

### Architecture invariant
`verify.sh` at session END:

```
[paste output]
```

**Diff vs start:** _identical / one-line drift / multiple drifts_

If drifts: list them and rate severity.

### Spend reconciliation
- BudgetTracker total: $_…_
- Anthropic console total: $_…_
- Variance: _…%_

### Rubric re-score (13 principles × surfaces touched)

Map each touched surface against the existing rubric in [`docs/specs/2026-04-20-ux-principles-rubric.md`](../../docs/specs/2026-04-20-ux-principles-rubric.md):

| Surface | P1 | P2 | P3 | P4 | P5 | P6 | P7 | P8 | P9 | P10 | P11 | P12 | P13 | aggregate |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Wizard – Welcome | | | | | | | | | | | | | | |
| Wizard – Connect | | | | | | | | | | | | | | |
| Wizard – Install | | | | | | | | | | | | | | |
| Wizard – Ready | | | | | | | | | | | | | | |
| Home (ok) | | | | | | | | | | | | | | |
| Home (recovering) | | | | | | | | | | | | | | |
| Home (error_key) | | | | | | | | | | | | | | |
| Home (paused_by_user) | | | | | | | | | | | | | | |
| Preferences – Keys | | | | | | | | | | | | | | |
| Telegram – first chat | | | | | | | | | | | | | | |
| Telegram – Tier A jobs | | | | | | | | | | | | | | |
| Telegram – Tier B refusals | | | | | | | | | | | | | | |

(Floor = 8.0; target = 8.5 for non-Telegram surfaces.)

### Deserve-to-exist sweep
For each surface, ask: *if removed, would Karen miss it?*

- _surface name_ — _yes / no / could be merged with X_

### Friction punch-list
P0 / P1 / P2 priority order.

| # | Severity | Surface | Finding | Proposed fix |
|---|---|---|---|---|
| 1 | | | | |
| 2 | | | | |
| … | | | | |

---

## Verdict

**Ship recommendation:** _SHIP / SHIP-WITH-CAVEATS / NO-SHIP_

**Single most-important finding:**

**Top three friction items to address before next release:**

1.
2.
3.

**The "really small win" that would make the most difference for Karen:**

---

*Findings written by: …*
*Reviewed by: …*
*Filed under: `docs/specs/`*
