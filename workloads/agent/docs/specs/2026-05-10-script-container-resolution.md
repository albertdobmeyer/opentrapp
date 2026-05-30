# Script Container Resolution

**Status:** Draft (2026-05-10)
**Pairs with:** parent PR-1 ([`docs/specs/v0.4-shell-tenant-reframe/07-container-name-cleanup.md`](https://github.com/albertdobmeyer/opentrapp/blob/main/docs/specs/v0.4-shell-tenant-reframe/07-container-name-cleanup.md) in opentrapp)

## Why

The parent perimeter (opentrapp) just dropped the four `container_name:` overrides from its `compose.yml` so containers get standard project-prefixed names (`<project>_<service>_<n>`). This was a precondition for project-isolated testing in v0.4.

The submodule's helper scripts (`verify.sh`, `vault-audit.sh`, `log-rotate.sh`, `setup.sh`) currently `inspect` and `exec` containers by literal name (`vault-proxy`, `opencli-container`). The names worked because:

- In standalone use, the submodule's own `compose.yml` declares `container_name: opencli-container` and `container_name: vault-proxy` — literal match.
- In parent use, the parent's `compose.yml` *also* used to declare `container_name: vault-proxy` — literal match for the proxy by coincidence. After parent PR-1 that override is gone.

The agent-side name match (`opencli-container`) was *only* ever true in standalone use — the parent's agent service has always been called `vault-agent`, so `verify.sh` against the parent has always exited 1 at its "container not running" gate.

## Scope of this PR

Switch all four scripts to a single `resolve_service_container` helper that looks containers up by the `com.docker.compose.service` label rather than by literal name. The label is set by `compose` regardless of project name or `container_name:` override, so:

- **Standalone use** keeps working unchanged. The submodule's `compose.yml` retains its `container_name:` overrides; standalone users with automation against literal names see no behaviour change. The label lookup happens to find them too.
- **Parent use** keeps working for the proxy (the only service whose name matches across both compose files). The proxy-side checks in `verify.sh` and the `exec_in_proxy` helper in `vault-audit.sh` resolve correctly post-PR-1.

## Out of scope (deeper architectural mismatch)

The agent service name differs between contexts:

| Context | Service name |
|---------|--------------|
| Standalone (this submodule's `compose.yml`) | `vault` |
| Parent (opentrapp's `compose.yml`) | `vault-agent` |

So `verify.sh`'s 24 agent-side checks continue to be standalone-only. The parent's `verify` command in `component.yml` (line 135) was always exiting 1 immediately. Fixing this requires either:

- Renaming the parent's `vault-agent` service to `vault` in the parent's `compose.yml`, or
- Renaming the submodule's `vault` service to `vault-agent` in the submodule's `compose.yml`, or
- The parent reimplementing the verify check natively rather than calling into the submodule's standalone harness.

Pick a path during the v0.4 reframe; tracked as a parent-side architecture follow-up. Not in this PR.

## Helper design

```bash
# resolve_service_container <service-name> [<alt-service-name>...]
#
# Resolves a compose service name to the actual running container name by
# label lookup. Tries each service name in order; returns the first match
# on stdout and exits 0. Returns 1 with empty stdout if nothing matches.
#
# Works in any compose project — uses the com.docker.compose.service label
# rather than depending on `container_name:` overrides or project names.
resolve_service_container() {
    local service container
    for service in "$@"; do
        container=$($RUNTIME ps -a \
            --filter "label=com.docker.compose.service=$service" \
            --format '{{.Names}}' 2>/dev/null | head -n 1)
        if [ -n "$container" ]; then
            echo "$container"
            return 0
        fi
    done
    return 1
}
```

Used as:

```bash
PROXY_CONTAINER=$(resolve_service_container vault-proxy) || PROXY_CONTAINER=""
```

Then existing `$RUNTIME exec "$PROXY_CONTAINER" sh -c ...` calls continue to work.

## Files touched

- `scripts/verify.sh` — replace literal `vault-proxy` references with `$PROXY_CONTAINER` resolved at script start; gate the proxy section on `[ -n "$PROXY_CONTAINER" ]`
- `scripts/vault-audit.sh` — replace `is_container_running` to use label lookup; resolve `CONTAINER` and `PROXY_CONTAINER` at script start
- `scripts/log-rotate.sh` — resolve `PROXY_CONTAINER` at script start
- `scripts/setup.sh` — replace the post-build `exec vault-proxy` echo-check with a label-based lookup

## Verification

- Standalone: `make verify` continues to pass (no behaviour change in standalone mode)
- Standalone: `make log-rotate` continues to function
- Parent context: `verify.sh`'s proxy section (checks 25+ if reached) resolves the parent's project-prefixed proxy container correctly. Agent-side checks still gate-fail per "out of scope" above.

No new tests needed — the existing 24-check verify suite is the test surface.
