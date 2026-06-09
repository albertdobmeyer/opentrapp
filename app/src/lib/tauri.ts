import { invoke as tauriInvoke } from "@tauri-apps/api/core";

import type {
  DiscoveredComponent,
  CommandResult,
  ComponentStatus,
  Workflow,
  WorkflowResult,
  SentinelActivity,
  Verdict,
  PendingApproval,
  AllowlistDecision,
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
  timeoutSeconds = 5,
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
  timeoutSeconds = 10,
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
  submodules: {
    id: string;
    name: string;
    cloned: boolean;
    has_manifest: boolean;
  }[];
  components: {
    component_id: string;
    component_name: string;
    needs_container_runtime: boolean;
    missing_config_files: {
      path: string;
      template: string | null;
      description: string | null;
    }[];
    check_passed: boolean | null;
  }[];
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
 * Bootstrap axis — state of the 3-container security shell
 * (proxy + forge + pioneer). Snake-case matches Rust serde rename.
 */
export type BootstrapState =
  | "installing"
  | "bootstrapping"
  | "shell_ready"
  | "shell_failed";

/**
 * Tenant axis — state of vault-agent (the OpenClaw runtime).
 * Snake-case matches Rust serde rename.
 */
export type TenantState =
  | "absent"
  | "activating"
  | "running"
  | "paused"
  | "errored";

export interface ContainerStatus {
  name: string;
  running: boolean;
}

/**
 * Live state of the 4-container perimeter. Updated by the Rust watchdog
 * every 30s and emitted as a `perimeter-state-changed` event on each
 * transition. The frontend can either read the latest cached value via
 * `getPerimeterState()` or subscribe to the event for push updates.
 *
 * Snake-case matches Rust's serde rename. See `app/src-tauri/src/lifecycle.rs`.
 */
export interface PerimeterStatus {
  bootstrap: BootstrapState;
  tenant: TenantState;
  containers: ContainerStatus[];
  /** Unix-millis timestamp of the last watchdog poll. 0 if watchdog hasn't ticked yet. */
  last_checked_unix_ms: number;
}

export async function getPerimeterState(): Promise<PerimeterStatus> {
  return invoke<PerimeterStatus>("get_perimeter_state");
}

/**
 * Aggregated user-facing status. Backend evaluator combines the
 * (BootstrapState, TenantState) pair with .env presence and an Anthropic
 * auth probe into a single value driving the Home hero state machine.
 * Snake-case matches Rust serde rename.
 */
export type AssistantStatus =
  | "installing"
  | "bootstrapping"
  | "shell_ready_absent"
  | "shell_failed"
  | "not_setup"
  | "starting"
  | "recovering"
  | "ok"
  | "error_perimeter"
  | "error_key"
  | "paused_by_user"
  | "dormant";

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

/**
 * Summary of a bootstrap pipeline failure. Populated only when
 * `status === "shell_failed"`. Lets the recovery card show cause-appropriate
 * copy without a separate IPC call.
 */
export interface BootstrapFailureSummary {
  cause: string;
  message: string;
  last_error: string | null;
}

export interface AssistantStatusSnapshot {
  status: AssistantStatus;
  alerts: BackendAlert[];
  last_checked_unix_ms: number;
  /** Populated only when status === "shell_failed". */
  bootstrap_failure: BootstrapFailureSummary | null;
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
  await invoke("restart_perimeter");
}

/**
 * Pause the perimeter on user request. Stops containers but keeps them
 * around (no destroy) so resume is fast (~3-5s). Persists across app
 * restarts via `~/.opentrapp/paused` so the user's intent survives.
 * Status aggregator reports `paused_by_user` while paused, suppressing
 * all "didn't recover" / "key not working" alerts.
 */
export async function pausePerimeter(): Promise<void> {
  await invoke("pause_perimeter");
}

/**
 * Bring the perimeter back online after a pause. Same `compose up -d`
 * path as restart since `compose stop` left the containers around.
 */
export async function resumePerimeter(): Promise<void> {
  await invoke("resume_perimeter");
}

/**
 * Re-run the bootstrap pipeline from scratch after a failure. Returns
 * immediately — the pipeline runs in the background. The frontend observes
 * progress via the `bootstrap-step-started` / `bootstrap-step-failed` events.
 */
export async function retryBootstrap(): Promise<void> {
  await invoke("retry_bootstrap");
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

// ─── Activation flow ─────────────────────────────────────────────

/**
 * Outcome of a live Anthropic key validation ping. Structured so the
 * frontend can show exact guidance per error class.
 */
export type ValidationOutcome =
  | "ok"
  | "auth_failure"
  | "billing"
  | "permission"
  | "rate"
  | "server_error"
  | "unknown";

/**
 * Live-pings Anthropic to verify the key is accepted. Makes a direct
 * host → api.anthropic.com request (not via vault-proxy) as a pre-flight
 * check. Returns a structured outcome — never throws on API-level errors.
 * Rejects only on complete network failure.
 */
export async function validateAnthropicKey(key: string): Promise<ValidationOutcome> {
  return invoke<ValidationOutcome>("validate_anthropic_key", { key });
}

/**
 * A Telegram update that contains a /start message.
 */
export interface TelegramUpdate {
  update_id: number;
  chat_id: number;
}

/** Clears any leftover webhook so subsequent getUpdates long-polls work. */
export async function telegramDeleteWebhook(token: string): Promise<void> {
  return invoke("telegram_delete_webhook", { token });
}

/**
 * Long-polls Telegram for the first /start message at or after `offset`.
 * Returns the update when found, or `null` if the poll timed out.
 * Rejects on network errors or HTTP 409 (conflict: another instance is polling).
 */
export async function telegramPollForStart(
  token: string,
  offset: number,
  timeoutSecs: number,
): Promise<TelegramUpdate | null> {
  return invoke<TelegramUpdate | null>("telegram_poll_for_start", {
    token,
    offset,
    timeoutSecs,
  });
}

/** Sends a text message to the given chat. Rejects with "conflict" on HTTP 409. */
export async function telegramSendMessage(
  token: string,
  chatId: number,
  text: string,
): Promise<void> {
  return invoke("telegram_send_message", { token, chatId, text });
}

/**
 * Advances the server-side getUpdates offset past `updateId` so vault-agent
 * doesn't re-process the /start on its first poll.
 */
export async function telegramAdvanceOffset(
  token: string,
  updateId: number,
): Promise<void> {
  return invoke("telegram_advance_offset", { token, updateId });
}

/**
 * Finalises activation: force-recreates vault-proxy (picks up new .env keys),
 * brings vault-agent up, and writes the activated + credentials-ok marker files.
 *
 * The frontend must write both keys via `saveCredentials` BEFORE calling this —
 * the commit is transactional from the user's perspective.
 */
export async function commitActivation(): Promise<void> {
  return invoke("commit_activation");
}

/**
 * Write the agent credentials to the runtime `.env` (`~/.opentrapp/.env`) —
 * where the bootstrap and the perimeter read them. Only non-empty keys are
 * upserted; existing vars are preserved. Use this (NOT `writeConfig`) for the
 * keys: `writeConfig` targets the component directory, which is read-only on a
 * packaged first-run.
 */
export async function saveCredentials(
  anthropicKey: string,
  telegramToken: string,
): Promise<void> {
  return invoke("save_credentials", { anthropicKey, telegramToken });
}

/** Read the runtime `.env` body (`~/.opentrapp/.env`); "" if it doesn't exist yet. */
export async function readRuntimeEnv(): Promise<string> {
  return invoke<string>("read_runtime_env");
}

// ── Sentinel bridge (the GUI's consumer of the shared judgment lib) ───────

/** Current Sentinel activity rung (drives the activity indicator). */
export async function getSentinelActivity(): Promise<SentinelActivity> {
  return invoke<SentinelActivity>("get_sentinel_activity");
}

/**
 * Run the rung-2 judge on an opaque request ({context, fragment, task_hint,
 * static_signal}). The same JSON the CLI path uses; passed to sentinel/judge.sh.
 */
export async function sentinelJudge(
  request: Record<string, unknown>,
): Promise<Verdict> {
  return invoke<Verdict>("sentinel_judge", { request });
}

// ── Egress allowlist approvals (the human-mediated loosening surface) ──────

/**
 * The gray-zone off-allowlist hosts awaiting a human decision, each with the
 * judge's plain-language reason. Read-only — listing never loosens anything.
 * Clear exfil and rebinding blocks are filtered out server-side.
 */
export async function listEgressApprovals(): Promise<PendingApproval[]> {
  return invoke<PendingApproval[]>("list_egress_approvals");
}

/**
 * Apply a human decision on a host. "always" adds it to the allowlist + reloads
 * the gate (the only loosening path — human-tap only). "deny" only remembers
 * the denial; it never writes the allowlist.
 */
export async function applyAllowlistDecision(
  host: string,
  decision: AllowlistDecision,
): Promise<void> {
  return invoke("apply_allowlist_decision", { host, decision });
}
