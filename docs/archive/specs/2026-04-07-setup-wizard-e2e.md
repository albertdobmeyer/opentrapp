# Spec: Setup Wizard End-to-End

**Date:** 2026-04-07
**Phase:** I (Finalization Roadmap v4)
**Depends on:** Nothing
**Blocks:** Phase J (release — non-technical users need GUI-only setup)

---

## Problem

The setup wizard exists as a 5-step guided flow (`app/src/pages/Setup.tsx`) that checks prerequisites, initializes submodules, and creates config files from templates. However, it does **not** trigger the actual component setup commands (e.g., `make setup` for vault, which builds the container image and starts the stack).

After completing the wizard, a non-technical user still needs to open a terminal and run `make setup` / `make start` inside each component directory. This defeats the purpose of the GUI.

v3 Phase D requirement: *"Non-technical user can set up the full stack through the GUI — no terminal."*

---

## Current State

### Setup Wizard (`app/src/pages/Setup.tsx`)

5 steps:

| Step | Component | What It Does | What's Missing |
|------|-----------|-------------|----------------|
| `welcome` | `WelcomeStep` | Intro screen, "Get Started" button | Nothing |
| `prerequisites` | `PrerequisitesStep` | Calls `check_prerequisites` Tauri command → shows container runtime status, required files | Nothing |
| `submodules` | `SubmodulesStep` | Calls `init_submodules` Tauri command → clones missing submodules | Nothing |
| `config` | `ConfigStep` | Calls `create_config_from_template` → creates `.env` from `.env.example` | Does not guide API key entry |
| `complete` | `CompleteStep` | Sets `wizardCompleted: true`, navigates to dashboard | Does not trigger `setup_command` |

### Backend Commands Available

The Tauri backend already has all required handlers:

- `check_prerequisites` — checks for container runtime, required files (`app/src-tauri/src/commands/prerequisites.rs`)
- `init_submodules` — runs `git submodule update --init` (`prerequisites.rs:130-160`)
- `create_config_from_template` — copies `.env.example` to `.env` (`prerequisites.rs:180-226`)
- `run_command` — executes any manifest-declared command with injection safety (`app/src-tauri/src/commands/execute.rs`)
- `start_stream` — streams command output in real-time (`app/src-tauri/src/commands/stream.rs`)

### Manifest Contract

Each component.yml has a `prerequisites` section:

```yaml
prerequisites:
  container_runtime: true          # vault requires Podman/Docker
  setup_command: setup             # ID of the command to run for initial setup
  config_files:
    - template: .env.example
      target: .env
  check_command: "podman ps | grep openclaw-vault"
```

The `setup_command` field (`"setup"`) references a command declared in the `commands` section of the same manifest. The backend's `run_command` handler can execute it.

---

## Design

### Principle: Manifest-Driven, Zero Component Knowledge

The wizard must remain generic. It reads `prerequisites.setup_command` from the manifest and calls `run_command` with that command ID. It never contains vault/forge/pioneer-specific logic.

### New Step: "Setup Components"

Insert between `config` and `complete`:

```
welcome → prerequisites → submodules → config → setup-components → complete
```

The `setup-components` step:

1. Lists all discovered components that have `prerequisites.setup_command` defined
2. For each component, shows a "Set Up" button that triggers the setup command
3. Uses `start_stream` (not `run_command`) to show real-time output — setup commands can take minutes (container builds)
4. Shows per-component status: pending → running (with streaming output) → done / failed
5. "Next" button enabled only when all components with `container_runtime: true` succeed (or user explicitly skips)

### Config Step Enhancement: API Key Entry

The current config step creates `.env` from template but doesn't help the user fill in values. Enhance:

1. After creating `.env`, read the file via `read_config` Tauri command
2. Parse as `env` format (key-value pairs)
3. Show editable fields for keys that have empty or placeholder values (e.g., `ANTHROPIC_API_KEY=`, `TELEGRAM_BOT_TOKEN=`)
4. Save via `write_config` Tauri command
5. Link to relevant docs (e.g., "Get your API key at console.anthropic.com")

This uses the existing `EnvEditor` component (`app/src/components/EnvEditor.tsx`) which already handles secret masking.

### Complete Step: Verify & Launch

After setup-components succeeds:

1. Run each component's `check_command` (from `prerequisites.check_command`) to verify setup worked
2. Show green checkmarks for verified components
3. Offer "Start Vault" button (calls the component's `start` command from the manifest)
4. "Finish" marks wizard complete and navigates to dashboard

---

## Files to Modify

| Action | Path | Change |
|--------|------|--------|
| **Create** | `app/src/components/wizard/SetupComponentsStep.tsx` | New wizard step — triggers `setup_command` per component with streaming output |
| **Modify** | `app/src/pages/Setup.tsx` | Add `"setup-components"` to `STEP_ORDER` array (line 13), import and render new step |
| **Modify** | `app/src/components/wizard/ConfigStep.tsx` | After config creation, show editable env fields for empty/placeholder values |
| **Modify** | `app/src/components/wizard/CompleteStep.tsx` | Add verification check and optional "Start" button |
| **Modify** | `app/src/hooks/usePrerequisites.ts` | Add `runSetup(componentId: string)` and `runCheck(componentId: string)` functions that call `run_command`/`start_stream` |

### Existing Code to Reuse

- `StreamOutput` component (`app/src/components/StreamOutput.tsx`) — real-time output display with stop button and elapsed timer
- `EnvEditor` component (`app/src/components/EnvEditor.tsx`) — secret-masking env file editor
- `useCommand` hook (`app/src/hooks/useCommand.ts`) — command execution with error handling and toast notifications
- `useCommandStream` hook (`app/src/hooks/useCommandStream.ts`) — streaming command execution
- `useManifests` hook (`app/src/hooks/useManifests.ts`) — component manifest data

---

## Component Setup Commands

For reference, these are the setup commands declared in each component's manifest:

| Component | `setup_command` | Actual Command | Duration | `container_runtime` |
|-----------|----------------|----------------|----------|---------------------|
| openclaw-vault | `setup` | `make setup` | 2-5 min (container build) | `true` |
| clawhub-forge | `setup` | `make setup` | < 10 sec (no container) | `false` |
| moltbook-pioneer | `setup` | `make setup` | < 5 sec (config copy) | `false` |

The wizard should run all three but only gate progress on components where `container_runtime: true` (vault). Forge and pioneer setup is fast and unlikely to fail.

---

## UX Flow

```
┌──────────────────────────────────────────┐
│  Step 5 of 6: Set Up Components          │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │ ● OpenClaw Vault                   │  │
│  │   Building container image...      │  │
│  │   ┌──────────────────────────┐     │  │
│  │   │ Streaming output here... │     │  │
│  │   │ Step 1: Building image   │     │  │
│  │   │ Step 2: Starting stack   │     │  │
│  │   └──────────────────────────┘     │  │
│  │   [Stop]                           │  │
│  └────────────────────────────────────┘  │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │ ✓ ClawHub Forge — Ready            │  │
│  └────────────────────────────────────┘  │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │ ✓ Moltbook Pioneer — Ready         │  │
│  └────────────────────────────────────┘  │
│                                          │
│            [Back]  [Next →]              │
└──────────────────────────────────────────┘
```

---

## Testing

### Unit Tests

- `SetupComponentsStep` renders component list from manifests
- Clicking "Set Up" triggers `start_stream` with correct component ID and command ID
- "Next" button disabled while setup is running
- "Next" button enabled when all required setups succeed
- Failed setup shows error state with retry button

### E2E Tests (Playwright)

- Full wizard flow with mocked Tauri commands
- Setup step shows streaming output
- Wizard completes and navigates to dashboard
- `wizardCompleted` setting persisted

---

## Verification

1. Fresh install (or `wizardCompleted: false` in settings) → app redirects to setup wizard
2. Walk through all 6 steps without opening a terminal
3. After completion, dashboard shows all components with correct status
4. Re-running wizard from Settings page works correctly
