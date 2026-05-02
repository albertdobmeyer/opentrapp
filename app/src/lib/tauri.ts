import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type {
  DiscoveredComponent,
  CommandResult,
  ComponentStatus,
  Workflow,
  WorkflowResult,
} from "./types";

// Detect if running inside Tauri webview vs plain browser
const isTauri = !!(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    return Promise.reject(
      new Error(
        `Tauri IPC not available — running in browser mode. Command "${cmd}" requires the Tauri desktop app. Run "npm run tauri dev" instead of "npm run dev".`,
      ),
    );
  }
  return args ? tauriInvoke<T>(cmd, args) : tauriInvoke<T>(cmd);
}

export async function listComponents(): Promise<DiscoveredComponent[]> {
  return invoke<DiscoveredComponent[]>("list_components");
}

export async function setMonorepoRoot(
  path: string,
): Promise<DiscoveredComponent[]> {
  return invoke<DiscoveredComponent[]>("set_monorepo_root", { path });
}

export async function getComponent(
  componentId: string,
): Promise<DiscoveredComponent> {
  return invoke<DiscoveredComponent>("get_component", {
    componentId,
  });
}

export async function runCommand(
  componentId: string,
  commandId: string,
  args: Record<string, string> = {},
): Promise<CommandResult> {
  return invoke<CommandResult>("run_command", {
    componentId,
    commandId,
    args,
  });
}

export async function loadOptions(
  componentId: string,
  commandString: string,
  timeoutSeconds: number = 5,
): Promise<string[]> {
  return invoke<string[]>("load_options", {
    componentId,
    commandString,
    timeoutSeconds,
  });
}

export async function startStream(
  componentId: string,
  commandId: string,
  args: Record<string, string> = {},
): Promise<void> {
  return invoke("start_stream", {
    componentId,
    commandId,
    args,
  });
}

export async function stopStream(
  componentId: string,
  commandId: string,
): Promise<void> {
  return invoke("stop_stream", {
    componentId,
    commandId,
  });
}

export async function readConfig(
  componentId: string,
  configPath: string,
): Promise<string> {
  return invoke<string>("read_config", {
    componentId,
    configPath,
  });
}

export async function writeConfig(
  componentId: string,
  configPath: string,
  content: string,
): Promise<void> {
  return invoke("write_config", {
    componentId,
    configPath,
    content,
  });
}

export async function runHealthProbe(
  componentId: string,
  probeCommand: string,
  timeoutSeconds: number = 10,
): Promise<{ probe_id: string; stdout: string; stderr: string; exit_code: number }> {
  return invoke("run_health_probe", {
    componentId,
    probeCommand,
    timeoutSeconds,
  });
}

export async function getStatus(
  componentId: string,
): Promise<ComponentStatus> {
  return invoke<ComponentStatus>("get_status", {
    componentId,
  });
}

export interface PrerequisiteReport {
  container_runtime: {
    found: boolean;
    name: string | null;
    version: string | null;
  };
  submodules: Array<{
    id: string;
    name: string;
    cloned: boolean;
    has_manifest: boolean;
  }>;
  components: Array<{
    component_id: string;
    component_name: string;
    needs_container_runtime: boolean;
    missing_config_files: Array<{
      path: string;
      template: string | null;
      description: string | null;
    }>;
    check_passed: boolean | null;
  }>;
}

export async function checkPrerequisites(): Promise<PrerequisiteReport> {
  return invoke<PrerequisiteReport>("check_prerequisites");
}

export async function initSubmodules(): Promise<string> {
  return invoke<string>("init_submodules");
}

export async function createConfigFromTemplate(
  componentId: string,
  configPath: string,
  templatePath: string,
): Promise<void> {
  return invoke("create_config_from_template", {
    componentId,
    configPath,
    templatePath,
  });
}

// ─── Workflow commands ───────────────────────────────────────────

export async function listWorkflows(
  componentId: string,
): Promise<Workflow[]> {
  return invoke<Workflow[]>("list_workflows", {
    componentId,
  });
}

export async function executeWorkflow(
  componentId: string,
  workflowId: string,
  inputs: Record<string, string> = {},
): Promise<WorkflowResult> {
  return invoke<WorkflowResult>("execute_workflow", {
    componentId,
    workflowId,
    inputs,
  });
}

/**
 * Returns a redacted diagnostic bundle as plain text. Safe to copy to clipboard
 * or paste into a support email — secrets, IP addresses, and usernames are
 * stripped server-side. See `app/src-tauri/src/commands/diagnostics.rs`.
 */
export async function generateDiagnosticBundle(): Promise<string> {
  return invoke<string>("generate_diagnostic_bundle");
}

/**
 * Live state of the 4-container perimeter. Updated by the Rust watchdog
 * every 30s and emitted as a `perimeter-state-changed` event on each
 * transition. The frontend can either read the latest cached value via
 * `getPerimeterState()` or subscribe to the event for push updates.
 *
 * Snake-case matches Rust's serde rename. See `app/src-tauri/src/lifecycle.rs`.
 */
export type PerimeterState =
  | "not_setup"
  | "starting"
  | "running_safely"
  | "recovering"
  | "stopped";

export interface ContainerStatus {
  name: string;
  running: boolean;
}

export interface PerimeterStatus {
  state: PerimeterState;
  containers: ContainerStatus[];
  /** Unix-millis timestamp of the last watchdog poll. 0 if watchdog hasn't ticked yet. */
  last_checked_unix_ms: number;
}

export async function getPerimeterState(): Promise<PerimeterStatus> {
  return invoke<PerimeterStatus>("get_perimeter_state");
}

/**
 * Aggregated user-facing status. Backend evaluator (Pass 7 Day 2)
 * combines perimeter state + .env presence + Anthropic auth probe into
 * a single value driving the Home hero state machine. Snake-case
 * matches Rust serde rename.
 *
 * `paused_by_user` is reserved for Day 4. `starting` is currently
 * unused by the backend (which maps any partial state to `recovering`)
 * — the frontend's `useHero` hasBeenRunning ref flips the first
 * occurrence to "starting" for nicer first-run copy.
 */
export type AssistantStatus =
  | "not_setup"
  | "starting"
  | "recovering"
  | "ok"
  | "error_perimeter"
  | "error_key"
  | "paused_by_user";

export type AlertSeverity = "danger" | "warning" | "info";

export interface BackendAlert {
  id: string;
  severity: AlertSeverity;
  title: string;
  body: string | null;
  cta_label: string | null;
  cta_to: string | null;
  dismissable: boolean;
  /** True when the alert should NOT show during the wizard. */
  suppress_during_wizard: boolean;
}

export interface AssistantStatusSnapshot {
  status: AssistantStatus;
  alerts: BackendAlert[];
  last_checked_unix_ms: number;
}

export async function getAssistantStatus(): Promise<AssistantStatusSnapshot> {
  return invoke<AssistantStatusSnapshot>("get_assistant_status");
}

/**
 * Cycle the perimeter (down + up). Awaited — resolves only when the
 * perimeter is actually back online (typically ~10–20s). Used by
 * Preferences after a key rotation so vault-agent picks up the new
 * value without making the user manually relaunch.
 *
 * Rejects with a friendly Error message when bring-up fails (most
 * likely a malformed key the user just saved). Caller should surface
 * via `classifyError`.
 */
export async function restartPerimeter(): Promise<void> {
  return invoke<void>("restart_perimeter");
}

/**
 * Pause the perimeter on user request. Stops containers but keeps them
 * around (no destroy) so resume is fast (~3-5s). Persists across app
 * restarts via `~/.lobster-trapp/paused` so the user's intent survives.
 * Status aggregator reports `paused_by_user` while paused, suppressing
 * all "didn't recover" / "key not working" alerts.
 */
export async function pausePerimeter(): Promise<void> {
  return invoke<void>("pause_perimeter");
}

/**
 * Bring the perimeter back online after a pause. Same `compose up -d`
 * path as restart since `compose stop` left the containers around.
 */
export async function resumePerimeter(): Promise<void> {
  return invoke<void>("resume_perimeter");
}

/**
 * Resolved Telegram bot identity. Both fields come from a single `getMe`
 * call and are always populated together.
 */
export interface TelegramBot {
  url: string;
  username: string;
}

/**
 * Resolves a Telegram bot token into a `{url, username}` pair. Calls
 * Telegram's `getMe` endpoint from Rust (keeps the token out of webview
 * memory and avoids a CSP relaxation). Rejects on network errors, bad
 * tokens, or missing username — callers should fall back to a generic
 * Telegram link silently.
 */
export async function deriveTelegramBotUrl(token: string): Promise<TelegramBot> {
  return invoke<TelegramBot>("derive_telegram_bot_url", { token });
}
