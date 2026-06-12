use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub identity: Identity,
    #[serde(default)]
    pub status: Option<Status>,
    #[serde(default)]
    pub commands: Vec<Command>,
    #[serde(default)]
    pub configs: Vec<Config>,
    #[serde(default)]
    pub health: Vec<HealthProbe>,
    #[serde(default)]
    pub prerequisites: Option<Prerequisites>,
    #[serde(default)]
    pub workflows: Vec<Workflow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisites {
    #[serde(default)]
    pub container_runtime: bool,
    #[serde(default)]
    pub setup_command: Option<String>,
    #[serde(default)]
    pub config_files: Vec<PrereqConfigFile>,
    #[serde(default)]
    pub check_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrereqConfigFile {
    pub path: String,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub role: Role,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Runtime,
    Toolchain,
    Network,
    Placeholder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    #[serde(default)]
    pub states: Vec<State>,
    #[serde(default)]
    pub probes: Vec<StatusProbe>,
    #[serde(default)]
    pub default_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusProbe {
    pub command: String,
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_probe_timeout")]
    pub timeout_seconds: u64,
    pub rules: Vec<ProbeRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeRule {
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub stdout_contains: Option<String>,
    #[serde(default)]
    pub stdout_regex: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_group")]
    pub group: CommandGroup,
    #[serde(default = "default_command_type")]
    pub r#type: CommandType,
    #[serde(default = "default_danger")]
    pub danger: Danger,
    pub command: String,
    #[serde(default)]
    pub args: Vec<Arg>,
    #[serde(default)]
    pub output: Option<Output>,
    #[serde(default)]
    pub available_when: Vec<String>,
    #[serde(default = "default_sort_order")]
    pub sort_order: i32,
    #[serde(default = "default_tier")]
    pub tier: CommandTier,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommandTier {
    User,
    Advanced,
}

fn default_tier() -> CommandTier {
    CommandTier::Advanced
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommandGroup {
    Lifecycle,
    Operations,
    Monitoring,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommandType {
    Action,
    Query,
    Stream,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Danger {
    Safe,
    Caution,
    Destructive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arg {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub r#type: ArgType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub options_from: Option<OptionsFrom>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ArgType {
    String,
    Enum,
    Boolean,
    Number,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsFrom {
    pub command: String,
    #[serde(default = "default_probe_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    #[serde(default = "default_output_format")]
    pub format: OutputFormat,
    #[serde(default = "default_output_display")]
    pub display: OutputDisplay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Text,
    Ansi,
    Json,
    Jsonl,
    Sarif,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum OutputDisplay {
    Log,
    Table,
    Badge,
    Checklist,
    CardGrid,
    Terminal,
    Report,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub path: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub format: ConfigFormat,
    #[serde(default = "default_true")]
    pub editable: bool,
    #[serde(default = "default_danger")]
    pub danger: Danger,
    #[serde(default)]
    pub schema: Option<serde_json::Value>,
    #[serde(default)]
    pub line_list: Option<LineListMeta>,
    #[serde(default)]
    pub restart_required: bool,
    #[serde(default)]
    pub restart_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigFormat {
    Yaml,
    Json,
    Json5,
    Env,
    LineList,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineListMeta {
    #[serde(default)]
    pub item_label: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub example: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthProbe {
    pub id: String,
    pub name: String,
    pub command: String,
    #[serde(default = "default_health_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_probe_timeout")]
    pub timeout_seconds: u64,
    pub parse: HealthParse,
    #[serde(default)]
    pub thresholds: Option<HealthThresholds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthParse {
    pub r#type: ParseType,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ParseType {
    Regex,
    JsonPath,
    LineCount,
    ExitCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthThresholds {
    #[serde(default)]
    pub green: Option<String>,
    #[serde(default)]
    pub yellow: Option<String>,
    #[serde(default)]
    pub red: Option<String>,
}

// ─── Workflow types ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub user_description: Option<String>,
    #[serde(default = "default_workflow_trigger")]
    pub trigger: WorkflowTrigger,
    #[serde(default = "default_danger")]
    pub danger: Danger,
    #[serde(default = "default_shell_requirement")]
    pub shell_requirement: ShellRequirement,
    pub steps: Vec<WorkflowStep>,
    #[serde(default)]
    pub inputs: Vec<WorkflowInput>,
    #[serde(default)]
    pub output: Option<WorkflowOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowTrigger {
    Manual,
    OnDemand,
    Automatic,
    Scheduled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ShellRequirement {
    Hard,
    Split,
    Soft,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub args: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub depends_on: Option<String>,
    #[serde(default = "default_true")]
    pub abort_on_failure: bool,
    #[serde(default)]
    pub success_condition: Option<SuccessCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCondition {
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub stdout_contains: Option<String>,
    #[serde(default)]
    pub stdout_regex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub id: String,
    pub r#type: WorkflowInputType,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowInputType {
    String,
    Url,
    Enum,
    Boolean,
    Number,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOutput {
    #[serde(default = "default_workflow_display")]
    pub display: WorkflowDisplayMode,
    #[serde(default)]
    pub summary_step: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowDisplayMode {
    Log,
    Checklist,
    Report,
    Badge,
}

// Default value functions
fn default_workflow_trigger() -> WorkflowTrigger { WorkflowTrigger::Manual }
fn default_shell_requirement() -> ShellRequirement { ShellRequirement::Any }
fn default_workflow_display() -> WorkflowDisplayMode { WorkflowDisplayMode::Checklist }
fn default_interval() -> u64 { 10 }
fn default_probe_timeout() -> u64 { 5 }
fn default_health_interval() -> u64 { 30 }
fn default_group() -> CommandGroup { CommandGroup::Operations }
fn default_command_type() -> CommandType { CommandType::Action }
fn default_danger() -> Danger { Danger::Safe }
fn default_sort_order() -> i32 { 100 }
fn default_timeout() -> u64 { 60 }
fn default_output_format() -> OutputFormat { OutputFormat::Text }
fn default_output_display() -> OutputDisplay { OutputDisplay::Log }
fn default_true() -> bool { true }
