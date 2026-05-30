# OpenClaw Tool Policy — Official Documentation Research

**Date:** 2026-03-29
**Sources:** docs.openclaw.ai (gateway/configuration-reference, tools/exec, tools/exec-approvals, gateway/sandbox-vs-tool-policy-vs-elevated, concepts/agent-workspace)
**OpenClaw version:** 2026.2.26 (pinned in Containerfile)
**Purpose:** Understand OpenClaw's tool system from official docs to inform Split Shell completion and Soft Shell design.

---

## 1. Tool Profiles (Official)

| Profile | Tools Included |
|---|---|
| `minimal` | `session_status` only |
| `coding` | `group:fs`, `group:runtime`, `group:sessions`, `group:memory`, `image` |
| `messaging` | `group:messaging`, `sessions_list`, `sessions_history`, `sessions_send`, `session_status` |
| `full` | No restriction (all tools) |

**DISCREPANCY FOUND:** Our `openclaw-reference.md` said `minimal` includes "message, read (read-only)". The official docs say `minimal` only includes `session_status`. This means in Hard Shell (profile: minimal), Hum had even fewer tools than we thought — no messaging tools, no read. The messaging must have been available through the Telegram channel integration, not through the tool profile.

## 2. Tool Groups (Official)

| Group | Tools |
|---|---|
| `group:runtime` | exec, bash, process |
| `group:fs` | read, write, edit, apply_patch |
| `group:sessions` | sessions_list, sessions_history, sessions_send, sessions_spawn, session_status |
| `group:memory` | memory_search, memory_get |
| `group:ui` | browser, canvas |
| `group:automation` | cron, gateway |
| `group:messaging` | message |
| `group:nodes` | nodes |
| `group:openclaw` | all built-in tools (excludes provider plugins) |

**NOTE:** `group:runtime` includes `bash` as a separate tool from `exec`. We only denied `exec` and `process` individually in some configs — we should use `group:runtime` for completeness.

## 3. Exec Security Modes (Official)

| Mode | Behavior |
|---|---|
| `deny` | Block all execution |
| `allowlist` | Only commands matching allowlist or safeBins can execute |
| `full` | All commands permitted |

Default: `deny` (sandbox), `allowlist` (gateway/node)

## 4. Exec Ask Modes (Official)

| Mode | Behavior |
|---|---|
| `off` | Never prompt |
| `on-miss` | Prompt only when allowlist doesn't match |
| `always` | Prompt on every command |

Default: `on-miss`

**NOTE:** If approval is required but no UI is reachable, the `askFallback` setting determines outcome: deny, allowlist-only, or full allow. We don't set `askFallback` — should verify what the default is.

## 5. Exec Host Modes (Official)

| Mode | Behavior |
|---|---|
| `auto` | Sandbox when active, otherwise gateway |
| `sandbox` | Run inside Docker container — fails closed if sandboxing disabled |
| `gateway` | Run in the Gateway process directly |
| `node` | Run on a remote node device |

Our config uses `host: "gateway"` — correct, since sandbox.mode is "off".

## 6. safeBins (Official)

"Stdin-only safe binaries that can run without explicit allowlist entries."

**Behavior:**
- Match resolved binary paths only (not basenames)
- Accept stdin-only input
- Cannot include interpreters/runtimes (python3, node, ruby, bash)
- Require explicit `safeBinProfiles` for custom restrictions
- Execution only from trusted directories (`/bin`, `/usr/bin`, plus `safeBinTrustedDirs`)

**Default safeBins (built into OpenClaw):** `cut`, `uniq`, `head`, `tail`, `tr`, `wc`

**Our safeBins (16 total):** cat, echo, mkdir, cp, mv, touch, date, head, tail, wc, sort, uniq, tr, cut, jq, tee

*Note: `rm` was removed (2026-03-30) — the agent is constructive only. All destructive operations are user-side.*

**CONCERN:** The official docs say safeBins are for "stdin-only" binaries. But we have `mkdir`, `cp`, `mv`, `touch` which take file path arguments. The official default safeBins (cut, uniq, head, tail, tr, wc) are all stdin filters. Our additions may not behave as expected with the safeBins path validation. Needs live testing.

## 7. safeBinProfiles (Official — CONFIRMED)

Custom argv policies per safe binary. Available fields:

| Field | Type | Purpose |
|---|---|---|
| `minPositional` | number | Minimum required positional arguments |
| `maxPositional` | number | Maximum allowed positional arguments |
| `allowedValueFlags` | array | Flags that are explicitly allowed |
| `deniedFlags` | array | Flags that are explicitly blocked |

**Example:**
```json5
safeBinProfiles: {
  "jq": {
    maxPositional: 1,
    deniedFlags: ["--rawfile", "--slurpfile"]
  },
  "pandoc": {
    deniedFlags: ["--lua-filter", "--filter", "-F"]
  }
}
```

**This confirms:** We CAN restrict arguments per binary. The `deniedFlags` field answers the Pandoc question — we could block `--lua-filter`. But we parked Pandoc; this knowledge is for future reference.

**Our current config:** All safeBinProfiles are empty `{}` — no argument restrictions. This means every safeBin can receive any arguments.

## 8. Approval Flow (Official)

When `ask: "always"`:
1. Command sent to approval system with command text, cwd, resolved path
2. User sees approval request in Telegram
3. User can: Allow once, Always allow (adds to allowlist), or Deny
4. "Always allow" persists the pattern in `~/.openclaw/exec-approvals.json`
5. Timeout: 30 minutes, then treated as denial
6. Telegram approval via `/approve <id> allow-once|allow-always|deny`

## 9. tools.fs.workspaceOnly — NOT IN OFFICIAL DOCS

**CRITICAL FINDING:** `tools.fs.workspaceOnly` does not appear in the official documentation. The docs say:

> "Tools resolve relative paths against the workspace, but absolute paths can still reach elsewhere on the host unless sandboxing is enabled"
> "Without sandboxing, the workspace is the default cwd, not a hard sandbox"

This means our `tools.fs.workspaceOnly: true` setting may be:
- An undocumented feature that works
- A config key that OpenClaw silently ignores (Zod doesn't reject unknown keys under `tools.fs`?)
- Something that was removed from recent versions

**MUST VERIFY:** Test whether the agent can actually read files outside the workspace with our current config. We already saw `/etc/passwd` in the session report's "Files Accessed" list — this suggests workspaceOnly might NOT be enforced.

## 10. Policy Application Order (Official)

Five layers, applied in order:
1. Tool Profile (base allowlist)
2. Provider Tool Profile (per-provider override)
3. Global/Per-Agent Policy (tools.allow/deny)
4. Provider Policy (per-provider allow/deny)
5. Sandbox Policy (when sandboxed)

"Deny always wins" — if a tool is in any deny list, it's removed regardless of other settings.

## 11. Debugging Tool Visibility

Official command: `openclaw sandbox explain [--session/--agent/--json]`

This shows effective permissions, current sandbox state, tool allow/deny status. We should try running this inside the container to see exactly what Hum has access to.

---

## Action Items

### CRITICAL — Verify workspaceOnly
- [ ] Test if Hum can read `/etc/passwd` or other files outside workspace
- [ ] Check if `tools.fs.workspaceOnly` is actually enforced by OpenClaw
- [ ] If NOT enforced: our security model has a gap — the container's hardening (read-only root) is the only thing preventing filesystem access outside the workspace
- [ ] Run `openclaw sandbox explain` inside the container

### HIGH — Update openclaw-reference.md
- [ ] Fix minimal profile description (session_status only, not messaging + read)
- [ ] Add group:memory and group:ui to tool groups
- [ ] Document safeBinProfiles argument restriction syntax
- [ ] Document approval flow details (timeout, /approve command)
- [ ] Note that tools.fs.workspaceOnly is unverified

### MEDIUM — Improve Split Shell config
- [ ] Consider adding `askFallback: "deny"` for when no UI is reachable
- [ ] Verify our safeBins behavior with path arguments (mkdir, cp, etc.)
- [ ] Test whether empty safeBinProfiles actually work or if they silently break

### LOW — Future reference
- [ ] safeBinProfiles.deniedFlags confirmed — can be used for Pandoc (parked)
- [ ] `openclaw sandbox explain` command exists for debugging tool visibility
- [ ] Loop detection available (`tools.loopDetection`) — could be useful for monitoring

---

*This research document captures official documentation findings. When official docs conflict with our assumptions, the official docs are authoritative. When official docs are silent (like tools.fs.workspaceOnly), we must verify from live testing or source code.*
