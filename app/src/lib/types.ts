// Types matching the component.yml manifest schema and Rust structs

export type Role = "runtime" | "toolchain" | "network" | "placeholder";
export type CommandGroup = "lifecycle" | "operations" | "monitoring" | "maintenance";
export type CommandType = "action" | "query" | "stream";
export type Danger = "safe" | "caution" | "destructive";

/** ADR-0021 security axis, distinct from `danger`: does the op reduce the perimeter's protection? Fail-closed default is `weakening`. */
export type BoundaryImpact = "neutral" | "weakening";
export type OutputFormat = "text" | "ansi" | "json" | "jsonl" | "sarif";
export type OutputDisplay =
  | "log"
  | "table"
  | "badge"
  | "checklist"
  | "card-grid"
  | "terminal"
  | "report";
export type ConfigFormat = "yaml" | "json" | "json5" | "env" | "line-list";
export type ParseType = "regex" | "json_path" | "line_count" | "exit_code";

export interface Identity {
  id: string;
  name: string;
  version: string;
  description: string;
  role: Role;
  icon?: string;
  color?: string;
  repo?: string;
}

export interface StateDefinition {
  id: string;
  label: string;
  icon?: string;
  color?: string;
}

export interface ProbeRule {
  exit_code?: number;
  stdout_contains?: string;
  stdout_regex?: string;
  state: string;
}

export interface StatusProbe {
  command: string;
  interval_seconds: number;
  timeout_seconds: number;
  rules: ProbeRule[];
}

export interface StatusConfig {
  states: StateDefinition[];
  probes: StatusProbe[];
  default_state?: string;
}

export interface OptionsFrom {
  command: string;
  timeout_seconds: number;
}

export interface Arg {
  id: string;
  name: string;
  description?: string;
  type: "string" | "enum" | "boolean" | "number";
  required: boolean;
  default?: unknown;
  options: string[];
  options_from?: OptionsFrom;
}

export interface Output {
  format: OutputFormat;
  display: OutputDisplay;
}

export interface Command {
  id: string;
  name: string;
  description?: string;
  group: CommandGroup;
  type: CommandType;
  danger: Danger;
  boundary_impact: BoundaryImpact;
  command: string;
  args: Arg[];
  output?: Output;
  available_when: string[];
  sort_order: number;
  tier: "user" | "advanced";
  timeout_seconds: number;
}

export interface LineListMeta {
  item_label?: string;
  pattern?: string;
  example?: string;
}

export interface Config {
  path: string;
  name?: string;
  description?: string;
  format: ConfigFormat;
  editable: boolean;
  danger: Danger;
  schema?: Record<string, unknown>;
  line_list?: LineListMeta;
  restart_required: boolean;
  restart_command?: string;
}

export interface HealthParse {
  type: ParseType;
  expression?: string;
  format?: string;
}

export interface HealthThresholds {
  green?: string;
  yellow?: string;
  red?: string;
}

export interface HealthProbe {
  id: string;
  name: string;
  command: string;
  interval_seconds: number;
  timeout_seconds: number;
  parse: HealthParse;
  thresholds?: HealthThresholds;
}

export interface PrereqConfigFile {
  path: string;
  template?: string;
  description?: string;
}

export interface Prerequisites {
  container_runtime: boolean;
  setup_command?: string;
  config_files: PrereqConfigFile[];
  check_command?: string;
}

export interface Manifest {
  identity: Identity;
  status?: StatusConfig;
  commands: Command[];
  configs: Config[];
  health: HealthProbe[];
  prerequisites?: Prerequisites;
  workflows: Workflow[];
}

// ─── Workflow types ──────────────────────────────────────────────

export type WorkflowTrigger = "manual" | "on-demand" | "automatic" | "scheduled";
export type ShellRequirement = "hard" | "split" | "soft" | "any";
export type WorkflowInputType = "string" | "url" | "enum" | "boolean" | "number";
export type WorkflowDisplayMode = "log" | "checklist" | "report" | "badge";
export type StepStatus = "pending" | "running" | "passed" | "failed" | "skipped";
export type WorkflowStatus = "running" | "completed" | "failed" | "aborted";

export interface SuccessCondition {
  exit_code?: number;
  stdout_contains?: string;
  stdout_regex?: string;
}

export interface WorkflowStep {
  id: string;
  command: string;
  name?: string;
  args: Record<string, string>;
  depends_on?: string;
  abort_on_failure: boolean;
  success_condition?: SuccessCondition;
}

export interface WorkflowInput {
  id: string;
  type: WorkflowInputType;
  label: string;
  description?: string;
  required: boolean;
  default?: unknown;
  options: string[];
}

export interface WorkflowOutput {
  display: WorkflowDisplayMode;
  summary_step?: string;
}

export interface Workflow {
  id: string;
  name: string;
  description?: string;
  user_description?: string;
  trigger: WorkflowTrigger;
  danger: Danger;
  shell_requirement: ShellRequirement;
  steps: WorkflowStep[];
  inputs: WorkflowInput[];
  output?: WorkflowOutput;
}

export interface StepResult {
  step_id: string;
  command_id: string;
  status: StepStatus;
  result?: CommandResult;
  error?: string;
}

export interface WorkflowResult {
  workflow_id: string;
  status: WorkflowStatus;
  steps: StepResult[];
  duration_ms: number;
}

export interface DiscoveredComponent {
  manifest: Manifest;
  component_dir: string;
}

export interface CommandResult {
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
}

export interface ComponentStatus {
  component_id: string;
  state_id: string;
}

export interface StreamLine {
  component_id: string;
  command_id: string;
  line: string;
  stream: "stdout" | "stderr";
}

export interface StreamEnd {
  component_id: string;
  command_id: string;
  exit_code: number;
}

// Group commands by their group field
export const COMMAND_GROUP_ORDER: CommandGroup[] = [
  "lifecycle",
  "operations",
  "monitoring",
  "maintenance",
];

export const COMMAND_GROUP_LABELS: Record<CommandGroup, string> = {
  lifecycle: "Lifecycle",
  operations: "Operations",
  monitoring: "Monitoring",
  maintenance: "Maintenance",
};

export const DANGER_STYLES: Record<Danger, string> = {
  safe: "btn-safe",
  caution: "btn-caution",
  destructive: "btn-destructive",
};

// ── Sentinel (the local-AI judgment layer) ───────────────────────────────
// Mirrors app/src-tauri/src/commands/sentinel.rs (serde snake_case).
export type SentinelRung = "watching" | "thinking" | "deep_analysis";

export interface SentinelActivity {
  rung: SentinelRung;
  /** Plain-language label (banned-vocabulary rule applies). */
  label: string;
  since_unix_ms: number;
}

/** The Verdict the judgment lib returns (mirrors sentinel/verdict-schema.json). */
export interface Verdict {
  decision: "allow" | "block" | "escalate";
  confidence: number;
  resolved_at_rung: number;
  reason: string;
}

/** A human decision on a gray-zone off-allowlist host (v0.6 Item A). */
export type AllowlistDecision = "always" | "deny";

/** One off-allowlist host awaiting a human decision, with the judge's reason. */
export interface PendingApproval {
  host: string;
  /** Plain-language reason from the judge (banned-vocabulary rule applies). */
  reason: string;
  judged_at_ms: number;
}

/**
 * One boundary-weakening control request the daemon has HELD for out-of-band
 * human approval (ADR-0021). The agent-writable control inbox can enqueue it but
 * cannot apply it; only the human two-tap here (`approveWeakening`) applies it.
 */
export interface PendingWeakening {
  /** Opaque handle to pass to `approveWeakening`. */
  id: string;
  /** The neutral control verb (`pause` | `shutdown`); the UI maps it to friendly copy. */
  verb: string;
}
