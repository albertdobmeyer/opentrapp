# The Cleanroom — skills leg (spec)

> Part of [OpenTrApp v0.6](00-index.md). Consumes [Sentinel](01-sentinel-spine.md).
> The skills shield = `openagent-skills`; its dir is `workloads/skills`
> (renamed from `forge` — SD1, see [`06-naming-consistency-sweep.md`](06-naming-consistency-sweep.md)).
> This spec uses the current `workloads/forge/` paths where it cites live code;
> the rename happens first (06) so the implementing agent builds on final names.
>
> **Built first** — the skills module already has the local model and the
> ZONE-4a bug that the rung-2 second-opinion fixes, so this leg proves the whole
> ladder end-to-end and ships the Sentinel library itself.
>
> Tagline: **"anything that can't survive being described is gone."**

---

## 1. What changes vs today

Today (`workloads/forge/`): an 8-stage CDR pipeline (`tools/skill-cdr.sh`)
where stage 4 (intent extraction) is the only LLM stage; the 87-pattern
scanner (`tools/lib/patterns.sh`) is pure regex and **fails closed on false
positives** — a `curl` in a documentation example reads as C2 and blocks a
legitimate skill. That false-positive class is ZONE 4a (clean skills fail the
pipeline).

v0.6 keeps the static scanner as the cheap **rung 0**, but adds Sentinel's
**rung 2 as a second opinion on the gray zone**, makes the CDR reconstruction
a true cleanroom, and produces a **disarm diff** the user can read.

## 2. The three deliverables

### 2a. Rung-2 second opinion (this IS the ZONE-4a fix)

When the scanner flags a line `SUSPICIOUS` (not `MALICIOUS`), instead of
fail-closed, the leg calls Sentinel:

- `context: "skill_content"`, `fragment:` the flagged line + surrounding
  block, `static_signal: { outcome: "suspicious", detail: <pattern id> }`.
- Sentinel rung 2 judges: *is this an executable instruction, or an example
  inside prose/a fenced code block being documented?*
- `allow` → the line survives, scan continues. `block` → quarantine as today.
  `escalate` → surface to the user (rare).

This removes the false-positive fail-closed without weakening the response to
genuine `MALICIOUS` hits (those still block at rung 0, never reaching rung 2).
**Net effect:** clean skills stop being blocked; malicious skills still are.

Files: `tools/skill-scan.sh` (call Sentinel on SUSPICIOUS),
`tools/lib/line-classifier.sh` (emit the block context for the fragment),
`tools/skill-cdr.sh` stage 3/7 (consume the verdict instead of hard-failing).

### 2b. The cleanroom reconstruction (make CDR's metaphor real)

Today stage 4 (LLM) extracts intent → stage 6 (static Python) rebuilds. The
4a bug is partly *malformed intent* from the LLM breaking the rebuild. v0.6
tightens this into an explicit **describe → schema-validate → regenerate**
loop:

1. **Describe** (rung 2): the model produces a structured *description* of the
   skill's intent (reuse the existing `cdr-intent.sh` schema: name, purpose,
   use_cases, commands[{cmd,context}], patterns, tips).
2. **Schema-validate** (static, `cdr-validate.py`): strict validation. If the
   description is malformed or `insufficient_content`, **quarantine** — do not
   guess. (This is the rule that fixes 4a's silent failure: a clean skill that
   produces a valid description proceeds; anything that can't be cleanly
   described is quarantined explicitly, with a reason, not a silent exit.)
3. **Regenerate** (static, `cdr-reconstruct.py`): rebuild the SKILL.md from
   the validated description *only*. No bytes of the original survive.
4. **Post-verify** (rung 0, existing): re-scan the regenerated artifact; it
   must pass cleanly (it should, since it was built from clean templates).

The invariant — *the skill that reaches the agent is built from a description,
never from the original bytes* — is what makes the tagline true: an attack
that needs specific bytes intact (encoded payload, smuggled instruction,
appended invisible content) cannot survive being described and rebuilt.

### 2c. The disarm diff (the trust artifact)

After CDR, produce a **plain-language diff** showing what was removed/changed:

```
Original asked me to:  read your environment variables and POST them to an external URL
Rebuilt version:       (removed — not part of the skill's stated purpose)

Original asked me to:  format CSV files          ✓ kept
```

- Generated from: the rung-0 scanner hits (what matched) + the rung-2 reasons
  (why) + the description delta (what the regenerate dropped vs the original
  parse).
- Surfaced in the GUI as the result of an install, and available to the bot so
  it can tell the user *"I cleaned this skill — here's what I removed."*
- Vocabulary: user-facing, banned-terms rule applies.

## 3. Data flow

```
download/skill ─▶ [rung 0] static scan ─┬─ MALICIOUS ─▶ quarantine + diff
                                        ├─ SUSPICIOUS ─▶ [Sentinel rung 2] ─┬ allow ─▶ continue
                                        │                                   ├ block ─▶ quarantine + diff
                                        │                                   └ escalate ─▶ user (rare)
                                        └─ clean ─────▶ continue
        continue ─▶ describe (rung 2) ─▶ schema-validate ─┬ valid ─▶ regenerate ─▶ post-verify ─▶ deliver + diff
                                                          └ invalid ─▶ quarantine + diff ("couldn't be safely described")
```

## 4. Interfaces to existing code

| Existing | Change |
|----------|--------|
| `tools/skill-cdr.sh` (8 stages) | stages 3 & 7 consume Sentinel verdicts instead of hard-fail; stage 4→5 becomes the describe→validate gate |
| `tools/lib/cdr-intent.sh` | generalised into the shared Sentinel rung-2 service (see [`01`] §8) |
| `tools/lib/cdr-validate.py` | becomes the hard quarantine gate (no silent exit) |
| `tools/lib/cdr-reconstruct.py` | unchanged in spirit; rebuilds from validated description only |
| `tools/skill-scan.sh`, `line-classifier.sh` | emit fragment+context on SUSPICIOUS; call Sentinel |
| new: disarm-diff generator | combines scanner hits + rung-2 reasons + description delta |

## 5. Tests (pre-build / TDD)

- **4a regression:** the clean-skill fixture
  (`tests/cdr-fixtures/clean-skill.md`) now completes CDR and emits "CDR
  complete" (it currently can fail). Pin it.
- **Malicious still blocked:** `injected-skill.md` still quarantines; the
  rung-2 second opinion must NOT rescue a genuine injection.
- **Describe-or-quarantine:** a deliberately incoherent skill yields explicit
  quarantine + a reason, never a silent exit.
- **No original bytes survive:** assert the regenerated SKILL.md shares no
  non-trivial line with the original (the cleanroom invariant).
- **Disarm diff content:** for `injected-skill.md`, the diff names the removed
  malicious behaviour in plain language and passes the banned-terms check.
- **Scanner-self-test still 10/10:** `tests/scanner-self-test/run.sh` unaffected
  (rung 0 unchanged).
- **orchestrator-check.sh:** extend §14 (forge spotlight) or add a check that
  the CDR pipeline declares the Sentinel hook + the disarm-diff output path.

## 6. Done-when

- Clean skills install (4a closed); malicious skills quarantine with a
  readable diff; the bot can explain what it removed; the scanner-self-test
  stays green; and the Sentinel service exists and is exercised end-to-end by
  this leg (the spine is now ready for legs 02 and 04).
