# 07 — Container-Name Cleanup (Precondition PR)

**Status:** Draft
**Parent:** [`00-architectural-reframe`](00-architectural-reframe.md)

## What this is

A small, separable PR that removes the hardcoded `container_name:` lines from [`compose.yml`](../../../compose.yml). Lands first; unblocks project-isolated testing for everything else in the v0.4 reframe. No behaviour change for users.

## The problem

`compose.yml` declares explicit `container_name:` values on all four services:

- Line 37: `container_name: vault-agent`
- Line 76: `container_name: vault-proxy`
- Line 126: `container_name: vault-forge`
- Line 164: `container_name: vault-pioneer`

This breaks `--project-name` isolation. When two compose invocations target different project names, they should produce different container names (`<project>_<service>_<n>`); explicit `container_name:` overrides this and forces a fixed string.

Practical consequences:
1. **Integration tests can't run safely on a developer's machine** while a live perimeter is up. The investigation in this overhaul hit exactly this — the planned partial-shell dryrun aborted because `--project-name opentrapp-dryrun up -d vault-proxy ...` would have collided with or terminated the running `vault-proxy` container.
2. **Future multi-tenant work is blocked.** If we ever ship a second tenant variant alongside OpenClaw, two parallel compose graphs need distinct container names — impossible with hardcoded names.
3. **podman-compose's recreate logic is dangerous in this configuration.** With name conflicts, podman-compose 1.0.6 may remove the existing container (with the user's session data) to "make room" for the new one.

## The fix

Remove the four `container_name:` lines. Compose will generate names of the form `<project>_<service>_<n>`. Default project name is the directory name (`opentrapp`), so containers become:
- `opentrapp_vault-agent_1`
- `opentrapp_vault-proxy_1`
- `opentrapp_vault-forge_1`
- `opentrapp_vault-pioneer_1`

Test isolation becomes trivial: `--project-name opentrapp-dryrun` produces `opentrapp-dryrun_vault-proxy_1` etc., zero collision.

## Code that references container names by string

Removing the hardcoded names breaks any code that filters/exec's by exact container name. Audit:

### `app/src-tauri/src/lifecycle.rs:26-27`
```rust
const PERIMETER_CONTAINERS: [&str; 4] =
    ["vault-agent", "vault-proxy", "vault-forge", "vault-pioneer"];
```
Used in `is_container_running` at [`lifecycle.rs:305-324`](../../../app/src-tauri/src/lifecycle.rs):
```rust
fn is_container_running(name: &str) -> bool {
    for runtime in &["podman", "docker"] {
        let out = StdCommand::new(runtime)
            .args([
                "ps",
                "--filter", &format!("name=^{}$", name),
                ...
```

The `name=^vault-proxy$` filter expects the literal name. After cleanup, the actual container name is `opentrapp_vault-proxy_1`. Three options:

1. **Use partial-match filter:** `name=vault-proxy` (matches `opentrapp_vault-proxy_1`). Simple, but matches *any* container with that substring (foo-vault-proxy-bar would also match).
2. **Use compose-aware filter:** `--filter "label=com.docker.compose.service=vault-proxy"` (uses the compose-set label). Requires no name knowledge; works regardless of project name. **Preferred.**
3. **Compute the exact name:** `format!("{}_{}_1", project_name, service)` and compare exactly. Brittle if the project name changes.

Recommended fix: switch to label-based filtering.

```rust
fn is_service_running(service: &str) -> bool {
    for runtime in &["podman", "docker"] {
        let out = StdCommand::new(runtime)
            .args([
                "ps",
                "--filter", &format!("label=com.docker.compose.service={}", service),
                "--filter", "status=running",
                "--format", "{{.Names}}",
            ])
            .output();
        if let Ok(o) = out {
            if o.status.success() && !o.stdout.trim_ascii().is_empty() {
                return true;
            }
        }
    }
    false
}
```

The `PERIMETER_CONTAINERS` array stays as a list of *service* names (which match `compose.yml`'s service keys), not container names.

### `app/src-tauri/src/commands/diagnostics.rs`
Likely has similar filter logic. Audit and update to label-based.

### `components/openclaw-vault/scripts/kill.sh` and `kill.ps1`
These currently use `compose down`/`compose kill` which are project-aware (no name knowledge needed). Likely no changes required; verify by reading.

### Documentation references
README.md, docs/trifecta.md, docs/diagrams.md may reference container names by their hardcoded string. Update any user-facing references to use service names (still `vault-agent`, `vault-proxy`, etc.) rather than container names. Diagrams that show "vault-proxy" as a box label stay correct because the service name doesn't change.

### `app/e2e/` tests
If any E2E test exec's into a container by exact name, update to label-based or use `compose exec` (which is project + service aware).

### Submodule scripts (follow-up PR in `openclaw-vault`)

The audit during PR-1 implementation surfaced four scripts inside the `components/openclaw-vault/scripts/` submodule that `inspect`/`exec` containers by hardcoded name:

- `verify.sh:19` — `CONTAINER="openclaw-vault"`
- `verify.sh:308-309` — `inspect "vault-proxy"` / `exec vault-proxy ...`
- `vault-audit.sh:26-27` + downstream — `CONTAINER="openclaw-vault"`, `PROXY_CONTAINER="vault-proxy"`
- `log-rotate.sh:29-30` + downstream — same pattern
- `setup.sh:107` — `exec vault-proxy ...`

**What PR-1.5 actually fixes (and what it doesn't):**

| Service | Standalone name | Parent name | Pre-PR-1 parent behaviour | Post-PR-1, pre-PR-1.5 | Post-PR-1.5 |
|---------|-----------------|-------------|---------------------------|------------------------|-------------|
| Proxy | `vault-proxy` | `vault-proxy` (literal pre-PR-1, project-prefixed post-PR-1) | Worked by coincidence (matching `container_name:` overrides) | Broken — `vault-proxy` literal no longer matches | Works in both contexts via label lookup |
| Vault/Agent | `vault` (service) → `openclaw-vault` (container_name) | `vault-agent` (service) → `vault-agent` literal pre-PR-1 | **Already broken** — `openclaw-vault` literal never existed in parent | Still broken (same reason) | Still broken (service-name mismatch is deeper) |

So PR-1.5 restores the proxy-side checks for the parent context (the actual regression introduced by PR-1) but does **not** restore the agent-side checks — those have been broken in the parent context since the parent existed and reflect a deeper service-naming mismatch between the submodule's standalone `compose.yml` (service `vault`) and the parent's (service `vault-agent`). Reconciling that is out of scope here; tracked as a future architectural cleanup (likely a coordinated rename in one of the two compose files).

The PR-1.5 fix uses a `resolve_service_container` helper that looks containers up by the `com.docker.compose.service` label. See [openclaw-vault `docs/specs/2026-05-10-script-container-resolution.md`](https://github.com/albertdobmeyer/openclaw-vault/blob/main/docs/specs/2026-05-10-script-container-resolution.md).

`kill.sh` and `kill.ps1` only reference the *volume* names (`openclaw-vault_vault-proxy-logs`), which are project-scoped and unaffected by `container_name:` removal — no change needed there.

## Migration concern: existing users

Existing users have stopped containers from the v0.3 build with hardcoded names (`vault-agent`, etc.). On v0.4 first launch with this PR landed:

1. Bootstrap runs `compose up -d` against the new project (now generating compose-named containers)
2. The old hardcoded-name containers are *orphans* — they exist in stopped state, not referenced by any compose project
3. RunGuard at [`lifecycle.rs:266-293`](../../../app/src-tauri/src/lifecycle.rs) already handles orphan reaping — but it currently uses `compose down`, which targets the current project. Old orphans wouldn't match.

Add a one-time orphan cleanup at first v0.4 launch:

```rust
fn reap_legacy_hardcoded_containers() {
    const LEGACY: [&str; 4] = ["vault-agent", "vault-proxy", "vault-forge", "vault-pioneer"];
    for runtime in &["podman", "docker"] {
        for name in LEGACY {
            let _ = StdCommand::new(runtime)
                .args(["rm", "-f", name])
                .status();
        }
    }
}
```

Run this once during migration ([`06-migration.md`](06-migration.md)), gated on a `~/.opentrapp/legacy-reaped` marker so it doesn't run on every launch. The volumes attached to the old containers stay intact (volumes are project-scoped, not container-scoped) — `vault-data`, `forge-deliveries`, `vault-proxy-logs`, `proxy-ca` survive the `rm -f` of the containers because volumes are independent of container lifecycle.

After reap, `compose up -d` against the new project mounts the same volumes and the user's session history is preserved.

## Test coverage

Unit tests in `app/src-tauri/src/lifecycle.rs`:
- `is_service_running` returns true when label-matching containers exist; false otherwise
- New label filter syntax round-trips through the runtime correctly

Integration tests:
- `compose up -d` with no project name produces correctly-labeled containers; `is_service_running` matches them
- `compose --project-name foo up -d` and `compose --project-name bar up -d` coexist without collision
- `reap_legacy_hardcoded_containers` removes pre-v0.4 containers; volumes survive

Manual smoke test:
- On a machine with a v0.3 perimeter running, install v0.4
- Verify the migration reaps the old containers
- Verify the new perimeter mounts the same volumes (session history visible in agent)

## Why this is a separable PR

The change has zero behavioural impact for users — same containers, same volumes, same compose semantics. It just removes a name override that was never load-bearing. The PR is small (4 line removals + ~50 lines of label-based filter refactor + ~20 lines of legacy reap). Reviewable in 30 minutes. Lands ahead of the rest of v0.4 to unblock isolated testing for the larger PRs.

## Risk analysis

**What could go wrong:**
- Some code path filters by exact container name and we miss it during the audit → label-based filtering returns wrong result → state machine misreports → user sees "recovering" when everything's fine. Mitigation: comprehensive grep + integration test.
- Legacy reap removes a container the user actually wanted to keep (e.g., they manually created `vault-proxy` for an unrelated purpose). Mitigation: highly unlikely; the names are project-specific. The marker file gates the reap so it only runs once.
- Documentation references go stale if we miss them. Mitigation: doc audit; the user-facing names (service names) don't change so most diagrams stay correct.

**What's safe:**
- Volumes are independent of container names; data preservation is guaranteed.
- Compose semantics with auto-generated names is the documented default; we're moving from the override back to standard.

## Out of scope

- Renaming the *services* in compose.yml (e.g., `vault-proxy` → `proxy`) — service names stay; only the explicit container_name override goes
- Restructuring compose.yml further (separate compose-files-per-tenant, profiles, etc.) — that's v1.0+
- Updating the `PERIMETER_CONTAINERS` array name in Rust — keep the variable for clarity; just rebrand it as service names
