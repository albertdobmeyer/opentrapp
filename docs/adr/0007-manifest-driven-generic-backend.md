# ADR-0007 — Manifest-driven generic backend

**Status:** Accepted
**Decision date:** 2026-04-15 (architecture v2 redesign)
**Implemented by:** [`schemas/component.schema.json`](../../schemas/component.schema.json) (the contract); [`app/src-tauri/src/orchestrator/manifest.rs`](../../app/src-tauri/src/orchestrator/manifest.rs) (Rust serde); [`app/src/lib/types.ts`](../../app/src/lib/types.ts) (TypeScript types); per-component `component.yml` files
**Verified by:** [`tests/orchestrator-check.sh`](../../tests/orchestrator-check.sh) §7 (manifest enum values match Rust serde expectations) and §6 (frontend-backend command parity); [`app/src-tauri/src/orchestrator/tests.rs`](../../app/src-tauri/src/orchestrator/tests.rs) (parser unit tests)

---

## Context

The application orchestrates three components (`opencli-container`, `openskill-forge`, `openagent-social`) plus a fourth slot (the `vault-proxy` egress gateway, which is part of `opencli-container`'s deployment but logically a separate role). Each component has its own status states, its own commands, its own configuration files, its own health probes, and its own multi-step workflows.

A naïve implementation hard-codes one component's behaviour into the orchestrator: `if component == "vault" then run verify.sh; else if component == "forge" then run skill-scan.sh ...`. This produces an orchestrator that grows linearly with the number of components, requires Rust changes for every component update, and tightly couples the GUI to the component set. Adding pioneer would require a Rust release; un-parking pioneer would require a Rust release; switching forge's verify command would require a Rust release.

The architectural goal is the opposite: the application should be able to add, remove, replace, or modify components by editing manifest files alone, with no Rust or React changes for component-specific behaviour.

This is a generic-backend constraint and it has a structural cost — the schema must be expressive enough to declare every behaviour the orchestrator needs to execute, and the schema must remain in lock-step across three implementations (the JSON Schema, the Rust serde structs, the TypeScript types).

## Decision

The Tauri backend reads `component.yml` manifests and executes what they declare. **It must not contain component-specific logic.**

The manifest contract has six sections, defined by [`schemas/component.schema.json`](../../schemas/component.schema.json):

1. **identity** — `id`, `name`, `version`, `role`, `icon`, `color`. The frontend uses these to render generic per-component UI.
2. **status** — declared states (`running`, `stopped`, `error`, etc.) and the probe commands that distinguish them. The orchestrator runs the probes; the frontend renders the resulting state.
3. **commands** — individual operations. Each command declares its argument schema, danger level, and output format. The orchestrator's runner ([`runner.rs`](../../app/src-tauri/src/orchestrator/runner.rs)) interpolates user-supplied arguments into the command line with shell-safe escaping; the frontend renders an argument form from the schema.
4. **configs** — editable configuration files with format metadata (JSON, YAML, JSON5, plain). The frontend renders a generic editor; the backend writes the file with path-traversal validation.
5. **health** — lightweight probes for dashboard badges. Distinct from the `status` probes in cardinality (frequent, cheap) and intent (per-tile UI signal vs. per-component overall state).
6. **workflows** — multi-step automated sequences (chains of commands presented as a single user action). Two flavours: *component workflows* declared inside a single `component.yml` (referencing only that component's commands), and *orchestrator workflows* declared in [`config/orchestrator-workflows.yml`](../../config/orchestrator-workflows.yml) (referencing component IDs plus command or workflow IDs across components).

The schema is implemented in three places that must stay in sync, and the alignment is verified mechanically:

- [`schemas/component.schema.json`](../../schemas/component.schema.json) — the source of truth
- [`app/src-tauri/src/orchestrator/manifest.rs`](../../app/src-tauri/src/orchestrator/manifest.rs) — the Rust serde structs
- [`app/src/lib/types.ts`](../../app/src/lib/types.ts) — the TypeScript types

The orchestrator-check suite (§7 of [`tests/orchestrator-check.sh`](../../tests/orchestrator-check.sh)) verifies enum values agree across the three layers. The §6 check verifies that every Rust command handler has a matching frontend invoke wrapper. Cross-references — commands referenced from workflows, states referenced from `available_when`, orchestrator workflow steps referencing component commands — are validated as part of §9.

## Consequences

### Positive

- **Components are pluggable.** Adding pioneer (when it un-parks), replacing forge (if a successor scanner is built), or extending vault (if a new sandbox-mode is added) requires editing manifest files plus shipping the underlying scripts. No Rust or React change is needed for new component-specific functionality unless the new functionality requires a new *kind* of behaviour the schema does not yet support.
- **The frontend is component-agnostic.** Dashboard tiles, command pickers, workflow runners, configuration editors are generic React components that accept manifest data as input. Three components ship as three rendered tiles with no per-tile React code; un-parking pioneer adds a fourth tile automatically.
- **Schema changes are forced to be coordinated.** Modifying the schema without also updating the Rust struct or the TypeScript type causes the orchestrator-check to fail at commit time. A contributor cannot add a manifest field that one layer does not understand without the failure being visible.
- **The component contract is auditable.** A reader inspecting a `component.yml` sees the component's complete external surface — what states it has, what commands it accepts, what configuration files it owns, what workflows it composes. There is no second source that could disagree.
- **Cross-component workflows are declarative.** [`config/orchestrator-workflows.yml`](../../config/orchestrator-workflows.yml) declares sequences like *forge.scan → vault.install* as YAML; the orchestrator runner walks them generically.
- **The constraint applies to AI contributions as well as human ones.** [`CLAUDE.md`](../../CLAUDE.md) §5 makes the generic-backend constraint explicit so that future agent-driven contributions do not re-introduce hard-coded component logic.

### Negative

- **The schema must anticipate every behaviour.** A new *kind* of behaviour (e.g. a streaming command output, a long-running background task, a per-command resource limit) requires a schema change, with the corresponding Rust + TypeScript + orchestrator-check updates. The cost is up-front; once the schema supports a behaviour, every component can use it.
- **Three-way alignment is non-trivial.** Schema, Rust serde, and TypeScript types are three artefacts that drift independently if not actively maintained. The orchestrator-check suite catches the most common drift (enum values, command parity, cross-references) but does not catch every possible mismatch. Subtle mis-alignments (e.g. an optional field in the schema that is required in Rust) are caught at parse time rather than at commit time.
- **Component-specific logic that genuinely belongs in the orchestrator is awkward.** Some behaviour — the assistant-status aggregator's seven-state machine, for example — is genuinely about the *application* rather than any single component, and lives in the Rust backend. Distinguishing "application logic" from "component logic" is a judgement call; the rule of thumb is that if the behaviour mentions a specific component by name, it belongs in that component's manifest, not in the orchestrator.
- **Manifest rendering carries some overhead.** Every dashboard render reads the manifests from disk, parses them, and projects them to JSON for the frontend. This is fast (sub-millisecond per component) but is non-zero work compared with hand-written component-specific UI.

### Neutral

- **The constraint does not prevent component-specific *scripts*.** Each component has full freedom to ship scripts, libraries, and binaries that implement its declared commands however it likes. The constraint is about the *orchestrator's* logic, not the components' logic. A forge script can be arbitrarily complex; the orchestrator just runs it.

## Alternatives considered

**(A) Hard-code component-specific logic in Rust.** The naïve approach. Rejected because it produces an orchestrator that requires Rust releases for every component update, and tightly couples the GUI to the component set.

**(B) A simpler manifest with only command-list and status-list.** Drop the `workflows`, `configs`, `health`, and detail-rich `commands` sections in favour of a minimal `[name, [commands]]` structure. Rejected because the dropped sections are exactly what allows the GUI to be component-agnostic; without an argument schema, the GUI cannot render an argument form, so the GUI must contain per-command form code, so the GUI is no longer component-agnostic.

**(C) A WASM-plugin model.** Each component ships a WASM module that the orchestrator loads. Rejected because the operational complexity (WASM toolchain, ABI versioning, plugin loading) is high for a project that has three components and is unlikely to grow to ten. The manifest approach gives the same composability with less moving parts.

**(D) An IPC-only model.** Each component runs a long-lived process that the orchestrator talks to over a Unix socket. Rejected because it adds per-component daemon processes (operational and resource cost) and produces an additional failure mode (component-daemon crash distinct from component-process-crash).

**(E) Embed the schema inside Rust source.** Define the manifest types in Rust and generate the JSON Schema and TypeScript from them. Rejected because the JSON Schema is the contract that components from outside this repository (a hypothetical third-party scanner module) read; making it a generated artefact of one implementation removes its standalone authority.

## References

- The schema itself: [`schemas/component.schema.json`](../../schemas/component.schema.json)
- The Rust serde structs: [`app/src-tauri/src/orchestrator/manifest.rs`](../../app/src-tauri/src/orchestrator/manifest.rs)
- The TypeScript types: [`app/src/lib/types.ts`](../../app/src/lib/types.ts)
- The validation suite: [`tests/orchestrator-check.sh`](../../tests/orchestrator-check.sh)
- The contract documented for contributors: [`CLAUDE.md`](../../CLAUDE.md) §4 (manifest contract) and §5 (generic-backend constraint)
- Architecture: [`docs/trifecta.md`](../trifecta.md) §9 (manifest-driven workflows)
- Whitepaper: [`docs/whitepaper.md`](../whitepaper.md) §7 (the implementation paragraph that frames the generic-backend constraint)
- Live cross-component workflows: [`config/orchestrator-workflows.yml`](../../config/orchestrator-workflows.yml)
- Component manifests: `components/opencli-container/component.yml`, `components/openskill-forge/component.yml`, `components/openagent-social/component.yml`
