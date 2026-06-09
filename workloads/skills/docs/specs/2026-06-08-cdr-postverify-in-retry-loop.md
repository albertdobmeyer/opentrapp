# Spec — CDR: bring post-verification into the retry-repair loop

**Status:** Proposed → implementing
**Date:** 2026-06-08
**Touches security boundary?** Yes (CDR pipeline control flow) — designed to PRESERVE
the security invariant; see §Security. No change to the scanner, prefilter, line
classifier, or the "original is never delivered" property.

## Problem (verified, not assumed)

The CDR retry-repair loop in `tools/skill-cdr.sh` covers stages 4–6
(describe → validate → reconstruct). **Stage 7 (post-verify: lint, scan, verify) runs
AFTER the loop and is terminal** — any failure `exit 1`s with no repair attempt
(skill-cdr.sh lines ~197–217).

Consequence: a reconstruction that passes `cdr-validate` (the schema) but marginally
fails `skill-lint` / `skill-scan` / `skill-verify` is **quarantined outright**, even
when the source skill is legitimate and clean. This is the ZONE-4a false-quarantine
class, and it is exactly why `qwen2.5-coder:3b` failed 2/2 where 1.5b passed:

- **Verified deterministically:** a schema-valid intent whose `tips` contains a
  `TODO` token passes `cdr-validate` (`VALID`) but the reconstructed `SKILL.md`
  FAILs `skill-lint` ("Found 1 TODO/FIXME/XXX placeholders"). A clean intent passes
  both. The linter's only three FAIL conditions are: missing H1, missing
  `## When to Use`, and TODO/FIXME/XXX placeholders — the reconstructor always emits
  the first two, so **placeholders in model-extracted prose are the only lint-FAIL
  path**, and the schema doesn't check for them. A more elaborate model is simply
  more likely to emit a placeholder-ish phrase.

The narrow patch (reject placeholders in `cdr-validate`) would fix only this one
symptom. The root issue is structural: **post-verify failures are terminal.** Any
marginal-but-clean reconstruction (placeholder, a `cmd` that resembles a scanner
pattern, an unrecognised line) deserves the same repair attempt stages 4–6 already
get.

## Change — post-verify inside the retry loop

Move the stage-7 checks (lint → scan → verify) INTO the describe→regenerate loop,
after a successful reconstruct. On any failure: capture the reason, set it as the
`repair_hint`, and `continue` (retry). Only after the retry budget is exhausted does
the skill quarantine (unchanged ZONE-4a "explicit quarantine, never silent" path).
After the loop, stage 7 just reports the already-passed result.

- The repair hint is specific ("reconstruction failed lint: Found 1 TODO/…
  placeholders" / "tripped the security scanner" / "failed zero-trust verification")
  so the describe step regenerates intent that avoids it.
- The loop header becomes `[4-7/8] Describe → Validate → Regenerate → Verify`.
- `CDR_MAX_RETRIES` (default 3) now bounds the full describe→…→verify cycle.

## Security (the invariant is preserved)

A malicious skill **cannot be "retried into passing"**:

1. Untrusted content is stripped at **stage 3 (pre-filter)** before the model ever
   sees it; every retry re-derives intent from the SAME already-filtered JSON.
2. `skill-scan` + `skill-verify` still gate **delivery** — a skill is delivered
   (stage 8) only if they pass on the reconstruction. Moving them earlier in the
   control flow does not remove that gate; it adds repair attempts before it.
3. The scanner/verifier are deterministic on patterns. If a retry's output still
   trips them, it trips them again → after the budget, quarantine. The only way a
   retry "passes" is if the offending content is DROPPED — i.e. not delivered. So
   the outcome is always either "clean delivery" or "quarantine", never "malicious
   delivery". This generalises the existing in-loop retry, whose own comment already
   states a malicious skill cannot survive the parse→rebuild.

Net: latency on a marginal skill may rise (up to N model calls instead of 1); the
security outcome is unchanged.

## Verification

- **Deterministic:** craft a schema-valid intent containing a `TODO` placeholder;
  confirm the OLD flow terminal-fails at stage 7 and the NEW flow retries (repair
  hint visible) — and that a forced-always-placeholder source eventually quarantines
  (fail-closed) rather than delivering.
- **Regression:** full pipeline on a real opencode skill with **1.5b** → still PASS,
  delivered.
- **Higher-fidelity model:** full pipeline with **3b** → now PASS (the placeholder
  reconstruction is repaired instead of terminally quarantined).
- **Security:** the ClawHavoc-style malicious opencode skill → still **BLOCKED**
  (quarantined, never delivered), confirming retries don't launder malice.
- `make self-test` (scanner/linter unchanged) and the CDR test suite green.
