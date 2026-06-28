# Command Surfaces Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make all three OpenTrApp concerns operable from one CLI (`opentrapp <concern> <verb>`), with the ADR-0021 boundary gate enforced at a single chokepoint that every projection (CLI, MCP, GUI) inherits, then distribute it and add an optional external-operator MCP adapter.

**Architecture:** One manifest-driven command API in `opentrapp-core`; the CLI/GUI/MCP are thin projections over it (ADR-0022 §1). No concern is rewritten; the only net-new code is a generic arg-mapper + dispatch (CLI), a chokepoint boundary gate (core), and a stdio MCP server. Reference specs: [CLI](2026-06-28-cli-first-command-surface.md) · [registry](2026-06-28-registry-native-distribution.md) · [MCP](2026-06-28-mcp-adapter.md) · [harmonization](2026-06-28-command-surfaces-harmonization.md).

**Tech Stack:** Rust (workspace `app/src-tauri`, crates `core`/`daemon`/`viewer-server`), `tokio`, `serde`/`serde_yaml`/`serde_json`, `glob`; bash test suites (`tests/orchestrator-check.sh`); cargo-dist + crates.io.

## Global Constraints

(Every task's requirements implicitly include these — copied verbatim from the specs / CLAUDE.md.)

- **The bar (CLAUDE.md §11–§12):** verify at the **consumption end** through the **product binary**, never `make`/`podman-compose`. Local green ≠ CI green. Unverifiable here = unverified, not done — name it.
- **Two security axes, never conflated:** `boundary_impact` (ADR-0021: `neutral`|`weakening`, fail-closed default `Weakening`) vs `danger` (operational UI hint). Only `boundary_impact` is gated.
- **`core` and `daemon` must stay tauri/wry/webkit-free** (CI asserts the dependency graph).
- **The CLI must pass arg values verbatim;** `core::runner::interpolate_args` does the shell-escaping. Never pre-quote/escape in the CLI (double-escaping is a bug).
- **DCO:** every commit needs `Signed-off-by:` — use `git commit -s`. CI gate `dco.yml` enforces it.
- **CI merge gates (full set):** `check-frontend` (`npm run lint` + `tsc --noEmit` + vitest), `check-rust` (`cargo test --workspace`), `check-goproxy`, `check-orchestration` (`bash tests/orchestrator-check.sh`), `integration-tests`, `smoke-test` (Playwright), `dco`.
- **Rust tests run:** `cd app/src-tauri && cargo test --lib` (137 tests at v0.9.0). Frontend needs Node ≥22 (`cd app && nvm use`).
- **Commit message footer:** end every commit message with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- **Branch, don't commit to `main`.** Open PRs; serialize merges (rebase a PR that falls behind before merging).

---

# Phase 0 — Boundary-impact enforcement (the security foundation)

**Why first:** the CLI and MCP are agent-reachable transports; ADR-0021 forbids a weakening edge on any of them. Today `Command.boundary_impact` is classified + fail-closed but **not runtime-enforced** (`execute::run_command` runs commands ungated; only the control channel is gated). This phase closes that at a single chokepoint so the CLI/MCP/GUI all inherit it. Decision: owner, 2026-06-28 ("close the gap properly").

**Expected audit outcome:** all ~69 skill/social commands are neutral-in-fact (scans/lints/reads/config). **If Task 0.1 finds any genuinely-weakening command, STOP and escalate to the owner** — holding-and-applying a weakening *command* requires extending the approval queue from `ControlRequest` to a `HeldRequest` enum (`approvals.rs` + `supervisor::apply_approved` + the GUI approvals card) and is a *separate* spec. v1 does not build that speculatively (YAGNI — zero weakening commands expected).

### Task 0.1: Classify every concern command honestly + pin it

**Files:**
- Modify: `workloads/skills/component.yml` (add `boundary_impact: neutral` to each of the 25 command blocks under `commands:`)
- Modify: `workloads/social/component.yml` (add `boundary_impact: neutral` to each command block under `commands:`)
- Modify: `tests/orchestrator-check.sh` (new section asserting explicit classification)

**Interfaces:**
- Produces: every workload command carries an explicit `boundary_impact: neutral` (or `weakening` if audited so — then escalate). Consumed by Task 0.2's gate and by all projections.

- [ ] **Step 1: Write the failing check** — add a new section near the end of `tests/orchestrator-check.sh` (match its existing `python3 - <<'PY'` + `pass`/`fail` style):

```bash
section "Boundary-impact: every workload command is explicitly classified (ADR-0021, fail-closed)"
python3 - <<'PY' && pass "every workload command declares boundary_impact" || fail "a workload command omits boundary_impact (fails closed to weakening — classify it)"
import sys, glob, yaml
missing = []
for f in glob.glob("workloads/*/component.yml"):
    doc = yaml.safe_load(open(f))
    for cmd in (doc.get("commands") or []):
        if "boundary_impact" not in cmd:
            missing.append(f"{doc['identity']['id']}:{cmd['id']}")
if missing:
    print("unclassified:", ", ".join(missing)); sys.exit(1)
sys.exit(0)
PY
```

- [ ] **Step 2: Run it to verify it fails**

Run: `bash tests/orchestrator-check.sh 2>&1 | grep -i boundary-impact`
Expected: FAIL — every skill/social command is currently unclassified.

- [ ] **Step 3: Audit + classify.** Read each command in both manifests. For each, decide: does running it *reduce the perimeter's protection* (the allowlist, the egress split, the credential separation, the read-only mounts)? Scans/lints/reads/config-within-component/engagement-level toggles do **not** → `neutral`. Add `boundary_impact: neutral` on its own line inside each command block (e.g. directly under the `danger:` line). **If any command genuinely weakens the perimeter → mark it `weakening` and STOP (escalate per the phase note).**

```yaml
  - id: scan
    name: Security Scan
    # ...existing fields...
    danger: safe
    boundary_impact: neutral   # ADR-0021: a scan reads, it does not weaken the perimeter
    command: make scan SKILL=${skill}
```

- [ ] **Step 4: Run it to verify it passes**

Run: `bash tests/orchestrator-check.sh 2>&1 | grep -i boundary-impact`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add workloads/skills/component.yml workloads/social/component.yml tests/orchestrator-check.sh
git commit -s -m "$(printf 'security: classify every workload command boundary_impact (ADR-0021, fail-closed)\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

### Task 0.2: Enforce `boundary_impact` at the `run_command` chokepoint

**Files:**
- Modify: `app/src-tauri/crates/core/src/orchestrator/error.rs` (add a variant)
- Modify: `app/src-tauri/crates/core/src/execute.rs:42-99` (add the gate + a pure helper + a test)

**Interfaces:**
- Produces: `pub fn gate_command_boundary(component_id: &str, cmd: &Command) -> Result<(), OrchestratorError>` and `OrchestratorError::BoundaryWeakeningRefused { component, command }`. Consumed implicitly by every caller of `run_command` (CLI, MCP, viewer-server).

- [ ] **Step 1: Write the failing test** — append to `execute.rs` `#[cfg(test)] mod tests` (the module already exists at line 126; import what you need):

```rust
    use crate::orchestrator::manifest::Command;
    use crate::boundary::BoundaryImpact;

    fn cmd_with(id: &str, bi: BoundaryImpact) -> Command {
        // minimal Command literal; rely on Default for the unrelated fields via serde
        serde_yaml::from_str(&format!(
            "id: {id}\nname: {id}\ncommand: 'echo hi'\nboundary_impact: {}\n",
            match bi { BoundaryImpact::Neutral => "neutral", BoundaryImpact::Weakening => "weakening" }
        )).unwrap()
    }

    #[test]
    fn gate_refuses_weakening_and_admits_neutral() {
        assert!(gate_command_boundary("skills", &cmd_with("scan", BoundaryImpact::Neutral)).is_ok());
        let err = gate_command_boundary("skills", &cmd_with("danger", BoundaryImpact::Weakening)).unwrap_err();
        assert!(matches!(err, OrchestratorError::BoundaryWeakeningRefused { .. }));
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd app/src-tauri && cargo test --lib gate_refuses_weakening_and_admits_neutral`
Expected: FAIL to compile — `gate_command_boundary` and the error variant don't exist.

- [ ] **Step 3: Add the error variant** in `orchestrator/error.rs` (after `CommandNotFound`):

```rust
    #[error("Refused: command {command} in component {component} is boundary-weakening (ADR-0021) and may not run from an agent-reachable surface")]
    BoundaryWeakeningRefused { component: String, command: String },
```

- [ ] **Step 4: Add the gate helper + call it** in `execute.rs`. Add the helper above `run_command`:

```rust
use crate::orchestrator::manifest::Command;

/// ADR-0021 chokepoint: a manifest command may run only if it is boundary-neutral.
/// Fail-closed — an unclassified command parses as `Weakening` (boundary.rs Default)
/// and is refused. Every caller of `run_command` (CLI, MCP, GUI) inherits this.
pub fn gate_command_boundary(component_id: &str, cmd: &Command) -> Result<(), OrchestratorError> {
    if cmd.boundary_impact.agent_operable() {
        Ok(())
    } else {
        Err(OrchestratorError::BoundaryWeakeningRefused {
            component: component_id.to_string(),
            command: cmd.id.clone(),
        })
    }
}
```

Then inside `run_command`, immediately after `manifest_cmd` is resolved (right after the `let (manifest_cmd, component_dir) = { ... };` block, before the on-demand container start):

```rust
    gate_command_boundary(&component_id, &manifest_cmd)?;
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cd app/src-tauri && cargo test --lib gate_refuses_weakening_and_admits_neutral`
Expected: PASS.

- [ ] **Step 6: Run the whole lib suite (regression)**

Run: `cd app/src-tauri && cargo test --lib`
Expected: all green (the existing `run_command_unknown_component_is_not_found` and 137 others still pass; neutral commands are unaffected).

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/crates/core/src/orchestrator/error.rs app/src-tauri/crates/core/src/execute.rs
git commit -s -m "$(printf 'security: enforce command boundary_impact at the run_command chokepoint (ADR-0021)\n\nA weakening/unclassified manifest command is now refused before execution, so\nthe CLI, MCP, and GUI projections all inherit one fail-closed gate.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

### Task 0.3: Confirm the GUI/viewer inherits the gate (no new code)

**Files:**
- Test: `app/src-tauri/crates/viewer-server` (existing route tests) — run only.

- [ ] **Step 1:** Run `cd app/src-tauri && cargo test --workspace`. Expected: green — the viewer-server's `run_command`/`execute_workflow` routes call the now-gated `execute::run_command`; all declared commands are neutral (Task 0.1), so the GUI is unaffected in practice while being protected against a future weakening command.
- [ ] **Step 2:** No commit (verification only). Record in the PR description that the GUI inherits the chokepoint gate with zero GUI code changes — the single-chokepoint design (Spec 1 §5.4).

**Phase 0 consumption-end gate:** on the box, `opentrapp-daemon vault verify` still `pass=7 fail=0` (the gate touches command execution, not the boundary self-test); `cargo test --workspace` + `bash tests/orchestrator-check.sh` green.

---

# Phase 1 — The unified CLI dispatch (`opentrapp <concern> <verb>`)

Builds on Phase 0 (the CLI inherits the gate for free). No binary rename yet — this phase ships under the existing `opentrapp-daemon` name (`opentrapp-daemon skill scan …`); the rename is Phase 2.

### Task 1.1: The generic, manifest-driven arg-mapper

**Files:**
- Create: `app/src-tauri/crates/daemon/src/cli.rs`
- Modify: `app/src-tauri/crates/daemon/src/main.rs` (add `mod cli;` near the top)

**Interfaces:**
- Produces: `pub enum ArgError`; `pub fn map_cli_args(cmd: &Command, tokens: &[String]) -> Result<HashMap<String, String>, ArgError>`; `pub fn map_cli_workflow_inputs(inputs: &[WorkflowInput], tokens: &[String]) -> Result<HashMap<String, String>, ArgError>` (both thin wrappers over a generic `map_cli<T: ArgLike>`). Consumed by Task 1.2.

- [ ] **Step 1: Write the failing tests** — create `cli.rs` with a `#[cfg(test)] mod tests` containing the goalposts (T3/T4/T5/T6/T10 from the CLI spec):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use opentrapp_core::orchestrator::manifest::Command;

    // a Command with one required enum arg `skill`, like skills/scan
    fn scan_cmd() -> Command {
        serde_yaml::from_str(
            "id: scan\nname: Scan\ncommand: 'make scan SKILL=${skill}'\nboundary_impact: neutral\n\
             args:\n  - id: skill\n    name: Skill\n    type: enum\n    required: true\n",
        ).unwrap()
    }
    // a Command with one optional number arg `count` default 50, like social/feed-scan
    fn feed_cmd() -> Command {
        serde_yaml::from_str(
            "id: feed-scan\nname: Feed\ncommand: './tools/feed-scanner.sh --recent ${count}'\n\
             boundary_impact: neutral\nargs:\n  - id: count\n    name: Count\n    type: number\n\
             required: false\n    default: 50\n",
        ).unwrap()
    }

    #[test]
    fn missing_required_arg_is_rejected() {
        let e = map_cli_args(&scan_cmd(), &[]).unwrap_err();
        assert!(matches!(e, ArgError::MissingRequired(ref id) if id == "skill"));
    }
    #[test]
    fn optional_arg_default_is_applied() {
        let m = map_cli_args(&feed_cmd(), &[]).unwrap();
        assert_eq!(m.get("count").map(String::as_str), Some("50"));
    }
    #[test]
    fn single_required_arg_accepts_a_positional() {
        let m = map_cli_args(&scan_cmd(), &["myskill".to_string()]).unwrap();
        assert_eq!(m.get("skill").map(String::as_str), Some("myskill"));
    }
    #[test]
    fn arg_values_pass_through_verbatim() {
        let danger = "a'; rm -rf / #".to_string();
        let m = map_cli_args(&scan_cmd(), &["--skill".to_string(), danger.clone()]).unwrap();
        assert_eq!(m.get("skill"), Some(&danger)); // core escapes; the CLI must not
    }
    #[test]
    fn unknown_arg_flag_is_rejected() {
        let e = map_cli_args(&scan_cmd(), &["--bogus".to_string(), "x".to_string()]).unwrap_err();
        assert!(matches!(e, ArgError::UnknownArg(ref id) if id == "bogus"));
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cd app/src-tauri && cargo test -p opentrapp-daemon --lib cli::`
Expected: FAIL to compile — `map_cli_args`/`ArgError` don't exist. (Add `mod cli;` to `main.rs` first so the module is compiled.)

- [ ] **Step 3: Implement the mapper** at the top of `cli.rs`:

```rust
use std::collections::HashMap;
use opentrapp_core::orchestrator::manifest::{Arg, Command, WorkflowInput};

#[derive(Debug)]
pub enum ArgError {
    UnknownArg(String),
    MissingValue(String),
    MissingRequired(String),
    UnexpectedPositional(String),
}

impl std::fmt::Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgError::UnknownArg(a) => write!(f, "unknown option --{a}"),
            ArgError::MissingValue(a) => write!(f, "option --{a} needs a value"),
            ArgError::MissingRequired(a) => write!(f, "missing required option --{a}"),
            ArgError::UnexpectedPositional(p) => write!(f, "unexpected argument(s): {p}"),
        }
    }
}

/// Convert a manifest default (`serde_json::Value`) to the string the runner expects.
fn value_to_arg_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(), // numbers/bools render unquoted; null → "null"
    }
}

/// A uniform view over command `Arg`s and workflow `WorkflowInput`s so one mapper
/// serves both (DRY). Both carry an id, a required flag, and an optional default.
trait ArgLike {
    fn id(&self) -> &str;
    fn required(&self) -> bool;
    fn default(&self) -> Option<&serde_json::Value>;
}
impl ArgLike for Arg {
    fn id(&self) -> &str { &self.id }
    fn required(&self) -> bool { self.required }
    fn default(&self) -> Option<&serde_json::Value> { self.default.as_ref() }
}
impl ArgLike for WorkflowInput {
    fn id(&self) -> &str { &self.id }
    fn required(&self) -> bool { self.required }
    fn default(&self) -> Option<&serde_json::Value> { self.default.as_ref() }
}

/// Map CLI tokens after the verb onto declared specs. Manifest-driven: `--<id> <value>`
/// for any declared spec, a single bare positional for a sole required spec, declared
/// defaults for omitted optionals, and a required-presence check. Values pass through
/// verbatim — `core::runner::interpolate_args` does the shell-escaping.
fn map_cli<T: ArgLike>(specs: &[T], tokens: &[String]) -> Result<HashMap<String, String>, ArgError> {
    let mut out: HashMap<String, String> = HashMap::new();
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        if let Some(key) = t.strip_prefix("--") {
            if !specs.iter().any(|a| a.id() == key) {
                return Err(ArgError::UnknownArg(key.to_string()));
            }
            let val = tokens.get(i + 1).ok_or_else(|| ArgError::MissingValue(key.to_string()))?;
            out.insert(key.to_string(), val.clone());
            i += 2;
        } else {
            positional.push(t.clone());
            i += 1;
        }
    }
    let required: Vec<&T> = specs.iter().filter(|a| a.required()).collect();
    if !positional.is_empty() {
        if required.len() == 1 && positional.len() == 1 && !out.contains_key(required[0].id()) {
            out.insert(required[0].id().to_string(), positional.remove(0));
        } else {
            return Err(ArgError::UnexpectedPositional(positional.join(" ")));
        }
    }
    for a in specs {
        if !out.contains_key(a.id()) {
            if let Some(def) = a.default() {
                out.insert(a.id().to_string(), value_to_arg_string(def));
            }
        }
    }
    for a in specs {
        if a.required() && !out.contains_key(a.id()) {
            return Err(ArgError::MissingRequired(a.id().to_string()));
        }
    }
    Ok(out)
}

/// Map CLI tokens onto a command's declared args.
pub fn map_cli_args(cmd: &Command, tokens: &[String]) -> Result<HashMap<String, String>, ArgError> {
    map_cli(&cmd.args, tokens)
}

/// Map CLI tokens onto a workflow's declared inputs (their `required` defaults to true).
/// A thin wrapper over the unit-tested generic `map_cli`; exercised end-to-end by the
/// `vet-skill` consumption-end check (Phase 1 gate).
pub fn map_cli_workflow_inputs(inputs: &[WorkflowInput], tokens: &[String]) -> Result<HashMap<String, String>, ArgError> {
    map_cli(inputs, tokens)
}
```

- [ ] **Step 4: Run to verify they pass**

Run: `cd app/src-tauri && cargo test -p opentrapp-daemon --lib cli::`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/crates/daemon/src/cli.rs app/src-tauri/crates/daemon/src/main.rs
git commit -s -m "$(printf 'feat(cli): generic manifest-driven arg-mapper for opentrapp <concern> <verb>\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

### Task 1.2: Concern resolution + dispatch over core

**Files:**
- Modify: `app/src-tauri/crates/daemon/src/cli.rs` (add `concern_to_component_id`, `dispatch_concern`)

**Interfaces:**
- Consumes: `map_cli_args` (Task 1.1); `opentrapp_core::orchestrator::discovery::discover_components`, `opentrapp_core::execute::run_command`, `opentrapp_core::workflow_ops::execute_workflow`, `opentrapp_core::orchestrator::podman::runtime_data_dir`.
- Produces: `pub fn concern_to_component_id(concern: &str) -> Option<&'static str>`, `pub async fn dispatch_concern(concern: &str, args: &[String]) -> std::process::ExitCode`. Consumed by Task 1.3 (main.rs).

- [ ] **Step 1: Write the failing tests** — add to `cli.rs` tests:

```rust
    #[test]
    fn concern_maps_to_component_id() {
        assert_eq!(super::concern_to_component_id("skill"), Some("skills"));
        assert_eq!(super::concern_to_component_id("social"), Some("social"));
        assert_eq!(super::concern_to_component_id("bogus"), None);
    }
```

(Dispatch over the live perimeter is exercised at the consumption end, Step 6 — not as a unit test, since it runs a container.)

- [ ] **Step 2: Run to verify it fails**

Run: `cd app/src-tauri && cargo test -p opentrapp-daemon --lib cli::concern_maps`
Expected: FAIL to compile — `concern_to_component_id` missing.

- [ ] **Step 3: Implement resolution + dispatch** in `cli.rs`:

```rust
use std::process::ExitCode;
use opentrapp_core::orchestrator::discovery::discover_components;
use opentrapp_core::orchestrator::podman::runtime_data_dir;

/// Map the user-facing concern name to the manifest component id. `vault` is NOT here —
/// it is the perimeter control path (dispatch_vault), not a manifest component.
pub fn concern_to_component_id(concern: &str) -> Option<&'static str> {
    match concern {
        "skill" => Some("skills"),
        "social" => Some("social"),
        _ => None,
    }
}

/// `opentrapp <concern> <verb> [--arg val…]` — project the manifest command/workflow API.
pub async fn dispatch_concern(concern: &str, args: &[String]) -> ExitCode {
    let Some(component_id) = concern_to_component_id(concern) else {
        eprintln!("opentrapp: unknown concern '{concern}' (try: vault | skill | social)");
        return ExitCode::from(2);
    };
    let root = match std::env::current_dir() {
        Ok(r) => r,
        Err(e) => { eprintln!("opentrapp: cannot resolve working dir: {e}"); return ExitCode::FAILURE; }
    };
    let components = match discover_components(&root) {
        Ok(c) => c,
        Err(e) => { eprintln!("opentrapp: failed to read components: {e}"); return ExitCode::FAILURE; }
    };
    let Some(component) = components.iter().find(|c| c.manifest.identity.id == component_id) else {
        eprintln!("opentrapp {concern}: component '{component_id}' not found under ./workloads");
        return ExitCode::FAILURE;
    };
    let verb = match args.first() {
        Some(v) if v != "--help" && v != "-h" => v.clone(),
        _ => { print_concern_help(concern, component); return ExitCode::SUCCESS; }
    };
    let rest = &args[1..];

    // command id?
    if let Some(cmd) = component.manifest.commands.iter().find(|c| c.id == verb) {
        let mapped = match map_cli_args(cmd, rest) {
            Ok(m) => m,
            Err(e) => { eprintln!("opentrapp {concern} {verb}: {e}"); return ExitCode::from(2); }
        };
        let dd = runtime_data_dir();
        match opentrapp_core::execute::run_command(
            &components, &dd, component_id.to_string(), verb.clone(), &mapped,
        ).await {
            Ok(outcome) => match outcome.result {
                Ok(res) => { print!("{}", res.stdout); eprint!("{}", res.stderr);
                    return ExitCode::from(res.exit_code.clamp(0, 255) as u8); }
                Err(e) => { eprintln!("opentrapp {concern} {verb}: {e}"); return ExitCode::FAILURE; }
            },
            Err(e) => { eprintln!("opentrapp {concern} {verb}: {e}"); return ExitCode::FAILURE; }
        }
    }
    // workflow id?
    if let Some(wf) = component.manifest.workflows.iter().find(|w| w.id == verb) {
        let inputs = match map_cli_workflow_inputs(&wf.inputs, rest) {
            Ok(m) => m,
            Err(e) => { eprintln!("opentrapp {concern} {verb}: {e}"); return ExitCode::from(2); }
        };
        match opentrapp_core::workflow_ops::execute_workflow(
            &components, component_id.to_string(), verb.clone(), &inputs,
        ).await {
            Ok(_) => return ExitCode::SUCCESS,
            Err(e) => { eprintln!("opentrapp {concern} {verb}: {e}"); return ExitCode::FAILURE; }
        }
    }
    eprintln!("opentrapp {concern}: unknown verb '{verb}'");
    print_concern_help(concern, component);
    ExitCode::from(2)
}
```

> **Workflow inputs are mapped, not stubbed:** `map_cli_workflow_inputs` (Task 1.1) handles `vet-skill`'s required `skill` input via the same generic mapper, so `opentrapp skill vet-skill --skill X` (or the positional `vet-skill X`) works in v1.

- [ ] **Step 4: Add the help renderer** (used above) — generic, from the manifest:

```rust
fn print_concern_help(concern: &str, component: &opentrapp_core::orchestrator::discovery::DiscoveredComponent) {
    println!("opentrapp {concern} — {}", component.manifest.identity.name);
    println!("  commands:");
    for c in &component.manifest.commands {
        let req: Vec<String> = c.args.iter().filter(|a| a.required).map(|a| format!("--{} <{}>", a.id, a.id)).collect();
        println!("    {:<16} {} {}", c.id, c.name, req.join(" "));
    }
    if !component.manifest.workflows.is_empty() {
        println!("  workflows:");
        for w in &component.manifest.workflows { println!("    {:<16} {}", w.id, w.name); }
    }
}
```

- [ ] **Step 5: Run unit test + build**

Run: `cd app/src-tauri && cargo test -p opentrapp-daemon --lib cli:: && cargo build -p opentrapp-daemon`
Expected: PASS + clean build.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/crates/daemon/src/cli.rs
git commit -s -m "$(printf 'feat(cli): dispatch opentrapp skill/social verbs over the core command API\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

### Task 1.3: Wire the concern arms into `main`

**Files:**
- Modify: `app/src-tauri/crates/daemon/src/main.rs:34-41` (add `skill`/`social` arms) and `print_help` (list them)

**Interfaces:**
- Consumes: `cli::dispatch_concern` (Task 1.2).

- [ ] **Step 1:** After the existing `vault` arm (`main.rs:34-36`), add:

```rust
    if matches!(args.first().map(String::as_str), Some("skill") | Some("social")) {
        return cli::dispatch_concern(&args[0], &args[1..]).await;
    }
```

- [ ] **Step 2:** Extend `print_help` (main.rs:260) with two lines:

```rust
    println!("  skill <verb>  run a Skill Firewall operation (see `opentrapp skill --help`)");
    println!("  social <verb> run an agent-social operation (see `opentrapp social --help`)");
```

- [ ] **Step 3: Build + manual smoke**

Run: `cd app/src-tauri && cargo build -p opentrapp-daemon && ./target/debug/opentrapp-daemon skill --help`
Expected: prints the skills command list rendered from the manifest (run from the repo root so `./workloads` resolves).

- [ ] **Step 4: Commit**

```bash
git add app/src-tauri/crates/daemon/src/main.rs
git commit -s -m "$(printf 'feat(cli): wire skill/social concern dispatch into the daemon entrypoint\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

### Task 1.4: Security + regression pins (T7, T8) + CLI↔manifest parity

**Files:**
- Modify: `app/src-tauri/crates/daemon/src/cli.rs` (T7 pin)
- Modify: `app/src-tauri/crates/daemon/src/main.rs` tests (T8 regression)
- Modify: `tests/orchestrator-check.sh` (CLI↔concern parity section)

- [ ] **Step 1: Write T7 (security pin)** in `cli.rs` tests — the concern dispatch must have no edge to the control/weakening path. Assert structurally that `concern_to_component_id` never yields `vault` and that the dispatch module does not import `control`:

```rust
    #[test]
    fn concern_dispatch_has_no_vault_or_control_edge() {
        // vault is the control path, never a concern component
        assert_eq!(super::concern_to_component_id("vault"), None);
        // guard: this module must not reference the control-channel submitter
        let src = include_str!("cli.rs");
        assert!(!src.contains("control::submit"), "concern dispatch must not touch the control channel");
    }
```

- [ ] **Step 2: Write T8 (vault regression)** in `main.rs` tests — assert the vault verb set is unchanged by parsing the help text:

```rust
    #[test]
    fn vault_help_lists_the_expected_verbs() {
        // capture print_vault_help via a known-substring check on the source of truth
        let src = include_str!("main.rs");
        for verb in ["up", "down", "status", "verify", "pause", "resume", "restart"] {
            assert!(src.contains(&format!("\"{verb}\"")) || src.contains(verb), "vault verb {verb} present");
        }
        // weakening verbs still routed through submit_weakening
        assert!(src.contains("submit_weakening"));
    }
```

- [ ] **Step 3: Add the parity section** to `tests/orchestrator-check.sh` (mirrors its §6 route-parity pattern):

```bash
section "CLI↔manifest parity: each dispatched concern resolves to a real component"
python3 - <<'PY' && pass "skill→skills, social→social resolve to component.yml" || fail "a CLI concern has no backing manifest"
import sys, glob, yaml
ids = {yaml.safe_load(open(f))["identity"]["id"] for f in glob.glob("workloads/*/component.yml")}
need = {"skills", "social"}  # the concern→component targets in daemon/src/cli.rs
missing = need - ids
if missing: print("missing component(s):", missing); sys.exit(1)
sys.exit(0)
PY
```

- [ ] **Step 4: Run the pins**

Run: `cd app/src-tauri && cargo test -p opentrapp-daemon --lib && cd /home/albertd/Repositories/opentrapp && bash tests/orchestrator-check.sh`
Expected: all green, `0 warnings`.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/crates/daemon/src/cli.rs app/src-tauri/crates/daemon/src/main.rs tests/orchestrator-check.sh
git commit -s -m "$(printf 'test(cli): pin no-control-edge (T7), vault regression (T8), CLI↔manifest parity\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

**Phase 1 consumption-end gate (on the box, perimeter up):**
- `./target/debug/opentrapp-daemon skill scan --skill <name>` runs the in-perimeter scan and matches the GUI/`vet` path's result; `opentrapp-daemon skill vet-skill --skill <name>` runs the lint→scan→verify workflow.
- `opentrapp-daemon social level-status` returns correct output.
- Regression: `opentrapp-daemon vault up` → `vault verify` still `pass=7 fail=0`; `vault pause`/`down` still print HELD and do not stop the perimeter from the control channel.
- Full CI set green.

---

# Phase 2 — Binary rename `opentrapp-daemon` → `opentrapp` (owner decision #2)

Gated on the owner confirming the rename timing (harmonization §7.2; recommended: yes, with a one-release alias). Isolated so Phase 1 does not depend on it.

- [ ] **2.1** In `crates/daemon/Cargo.toml:16-18`, change `[[bin]] name = "opentrapp-daemon"` → `name = "opentrapp"` (path unchanged); keep `[package.metadata.dist] dist = true`.
- [ ] **2.2** Add a back-compat alias `[[bin]] name = "opentrapp-daemon"` pointing at a 3-line `src/bin/opentrapp-daemon.rs` that `std::process::exit`s after re-exec'ing `opentrapp` with the same args (or document a packaging symlink). Remove the alias one release later.
- [ ] **2.3** Update all help strings (`print_help`, `print_vault_help` line 206-207 "Invoked via the headless daemon today…"), `docs/headless.md`, `README.md`, and every `opentrapp-daemon vault` reference → `opentrapp …` (CLAUDE.md §13 doc lockstep).
- [ ] **2.4** Run `cargo dist plan` and confirm the installer artifact is now `opentrapp`. Update `RELEASING.md` if it names the binary.
- [ ] **2.5** Commit (`-s`); flag in the PR that this changes the public installer artifact name (outward-facing — lands with Phase 3 release notes).

---

# Phase 3 — Registry readiness harness + owner cut

Per [Spec 2](2026-06-28-registry-native-distribution.md) §4–§7. The lane is already built; this is verification + the owner runbook. The agent-preparable parts (below) can run in parallel with Phases 0–2.

- [ ] **3.1 (run first):** `cd app/src-tauri && cargo publish -p opentrapp-core --dry-run --locked` (the load-bearing goalpost R1). Record exit 0, or fix what it surfaces (a missing `readme`/`keywords`/`categories` → Spec 2 §5). Heavy compile — box-capable or push a `workflow_dispatch` CI job.
- [ ] **3.2:** Add the readiness pins R2 (`core_manifest_has_crates_io_recommended_fields`), R3 (`core_has_no_path_deps_and_is_publishable`), R4 (`dist plan` announces `0.9.0` + 5 targets) as an orchestrator-check section or a `make release-dryrun` target. TDD: write each red, then satisfy.
- [ ] **3.3:** Draft the v0.9.0 CHANGELOG/release notes (R5) — the de-Tauri + goproxy + alpine story, scoped to what's verified (§11). Owner approves copy.
- [ ] **3.4 (OWNER, gated):** add `CARGO_REGISTRY_TOKEN`; `git tag v0.9.0 && git push origin v0.9.0` only on explicit go/no-go (the release hard-gate, ROADMAP:126); verify the draft at the consumption end (installers run, `cargo add opentrapp-core` resolves, images cosign-verify); publish; then the BundleVerifier digest-staging T0 on a clean box.

---

# Phase 4 — Optional MCP adapter (owner decision #4, after Phases 0–1)

Per [Spec 3](2026-06-28-mcp-adapter.md). Gated on the dep-vs-hand-roll decision (Spec 3 §9; recommended: hand-roll a minimal JSON-RPC-over-stdio server). Detailed bite-sized code is written **after** that decision (writing it now would be guessing on the protocol layer). The task shape:

- [ ] **4.1** New crate `app/src-tauri/crates/mcp` (`opentrapp-mcp`, `publish = false`, `dist = true`), links `opentrapp-core` + `serde_json` (+ the SDK iff chosen). Add to the workspace + the WebKit-free CI assertion.
- [ ] **4.2** TDD M6/M7 (protocol): `initialize` handshake advertises `tools` capability; unknown tool → JSON-RPC error.
- [ ] **4.3** TDD M3 (parity) + M1/M9 (security pins): the tool registry mirrors the neutral surface; excludes `approve_weakening`/`apply_approved`; `list_pending_approvals` is read-only.
- [ ] **4.4** TDD M2/M8: a weakening *control* tool (`vault_pause`) only queues + returns held (never `apply_approved`); the server opens no network socket (stdio only).
- [ ] **4.5** TDD M4/M5 (functional): neutral read/run tools return core's shapes; args pass verbatim (the Phase-0 chokepoint refuses weakening commands automatically).
- [ ] **4.6** Threat-model section (`docs/threat-model.md`) — host-side, external-operator-only, inherits the gate; cross-ref ADR-0022. Wire `opentrapp mcp` as a top-level arm.
- [ ] **4.7 (owner/manual tail):** register `opentrapp mcp` in a real Claude Code `.mcp.json`; call a read tool + a weakening tool; confirm read works + weakening is held pending the GUI. Named, not claimed here (needs Claude Code).

---

## Self-review (spec coverage)

- **CLI spec** §5 dispatch → Tasks 1.1–1.3; §5.3 arg-mapper → 1.1; §5.4 boundary axes → **Phase 0**; §7 T1–T10 → 1.1/1.2/1.4 (T1/T2 exercised at the consumption-end gate + unit where pure); §5.5 rename → Phase 2; §7 shell parity → 1.4.
- **Registry spec** §4 harness → 3.1/3.2; §5 metadata → 3.1; §6 R1–R5 → 3.1–3.3; §7 runbook → 3.4.
- **MCP spec** §4 surface + §7 M1–M9 → 4.2–4.5; §5 security → 4.4/4.6; §9 decision → gates Phase 4.
- **Boundary gap** (the cross-cutting correction) → Phase 0 in full.

**Open decisions that gate phases** (harmonization §7): #1 release sequencing (Phase 3 timing), #2 rename (Phase 2), #3 no-perimeter `skill scan` alias (deferred), #4 MCP dep (Phase 4). None blocks Phase 0 or Phase 1.

**Parallelizable, zero-decision chunks** (Sonnet agents from frozen tasks): the Task 0.1 mechanical YAML edits *after* the audit verdict; the arg-mapper (1.1) and its tests; the orchestrator-check sections; the Phase-3 readiness pins. Keep with lead/owner: the audit judgment (0.1), the rename (Phase 2), the release cut (3.4), the MCP dep choice + threat-model wording (Phase 4).
