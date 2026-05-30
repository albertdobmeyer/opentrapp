# Spec: Trial Run Findings — Hardening Before Soft Shell

**Date:** 2026-03-31
**Context:** Complete trial run of Hard Shell and Split Shell with attack surface probing from inside the container. Four findings require action before the module can be considered shippable.

---

## Finding 1: Config Self-Modification (CRITICAL)

**What:** The agent's config file (`~/.openclaw/openclaw.json`) is writable from inside the container because the persistent volume is mounted read-write. If an attacker gains arbitrary code execution (bypassing Layer 3 tool policy), they can modify the config to: enable elevated access, clear the deny list, set exec security to "full".

**Current mitigations:**
- Hard Shell: no exec tool — agent can't run code
- Split Shell: exec requires `ask: "always"` — user approves every command
- workspaceOnly: write/edit tools blocked outside workspace
- No interpreters (python, bash) in image — only node and sh
- safeBins don't include interpreters

**Remaining risk:** If OpenClaw's gateway has a vulnerability that allows code execution outside the tool policy, the config could be silently modified and hot-reloaded.

**Proposed fix:** Split the persistent volume into two mounts:
1. `~/.openclaw/workspace/` — read-write (agent workspace, memory, files)
2. `~/.openclaw/openclaw.json` — read-only bind mount from host

This requires changes to `compose.yml` and `entrypoint.sh`. The config would only be writable during container startup (entrypoint) and via `tool-control.sh --apply` (which stops the container first).

**Security implications:** Hot-reload stops working for config changes. Shell switching requires container restart (which `tool-control.sh --apply` already does). This is a feature, not a bug — the agent should NEVER be able to change its own security policy.

**Verification:** After fix, `echo 'test' >> ~/.openclaw/openclaw.json` inside the container should fail with "Read-only file system" or "Permission denied".

---

## Finding 2: rm Binary in Container Image (MEDIUM)

**What:** `/bin/rm` exists in the Alpine base image. It's not in safeBins and the exec allowlist blocks it, but if an attacker bypasses the allowlist, `rm` is available for destructive operations.

**Proposed fix:** Strip `rm`, `rmdir`, and other destructive binaries from the image in the Containerfile, same as we already strip `wget` and `curl`.

**Verification:** `which rm` inside the container returns "not found".

---

## Finding 3: Formalize Attack Surface Tests (MEDIUM)

**What:** The trial run probes (direct internet, shadow read, root write, API key visibility, config modification, Docker socket, interpreter availability) are not automated. They should be a test script.

**Proposed fix:** Create `tests/test-attack-surfaces.sh` that runs all the probes and reports pass/fail. Add to `make test`.

**Verification:** `make test` includes the new test script, all pass.

---

## Finding 4: Soft Shell Not Implemented (TRACKED)

**What:** The GLOSSARY defines Soft Shell, the roadmap references it, but no config, no design, no implementation exists. This is known and intentional — we complete Split Shell hardening before designing Soft Shell.

**Action:** Design and implement after Findings 1-3 are resolved. Requires a proper spec per our development principles.

---

## Implementation Order

```
Finding 1 (Config read-only mount)     — CRITICAL, do first
    ↓
Finding 2 (Strip rm from image)         — MEDIUM, quick Containerfile change
    ↓
Finding 3 (Attack surface test script)  — MEDIUM, formalizes the trial run
    ↓
Finding 4 (Soft Shell design + impl)    — TRACKED, full spec-driven process
    ↓
Final holistic review + certification
```

Each finding gets its own commit, tested and verified before moving to the next.
