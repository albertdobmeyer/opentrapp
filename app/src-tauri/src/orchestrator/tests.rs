#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::orchestrator::manifest::*;

    // =========================================================================
    // Manifest parsing tests
    // =========================================================================

    #[test]
    fn test_parse_minimal_manifest() {
        let yaml = r##"
identity:
  id: test-component
  name: Test Component
  version: "0.1.0"
  description: A test component
  role: placeholder
"##;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.identity.id, "test-component");
        assert_eq!(manifest.identity.role, Role::Placeholder);
        assert!(manifest.commands.is_empty());
        assert!(manifest.configs.is_empty());
        assert!(manifest.health.is_empty());
        assert!(manifest.status.is_none());
    }

    #[test]
    fn test_parse_full_manifest() {
        let yaml = r##"
identity:
  id: my-runtime
  name: My Runtime
  version: "1.2.3"
  description: A runtime component
  role: runtime
  icon: shield
  color: "#dc2626"
  repo: https://github.com/test/repo

status:
  states:
    - id: running
      label: Running
      icon: circle-check
      color: "#22c55e"
    - id: stopped
      label: Stopped
  probes:
    - command: echo running
      interval_seconds: 5
      timeout_seconds: 3
      rules:
        - stdout_contains: running
          state: running
        - exit_code: 1
          state: stopped
  default_state: stopped

commands:
  - id: start
    name: Start
    description: Start the service
    group: lifecycle
    type: action
    danger: safe
    command: make start
    available_when: [stopped]
    sort_order: 10
    timeout_seconds: 60
    output:
      format: ansi
      display: terminal

  - id: nuke
    name: Nuclear Kill
    group: lifecycle
    type: action
    danger: destructive
    command: make nuke
    sort_order: 50

  - id: scan
    name: Scan
    group: operations
    type: query
    danger: safe
    command: make scan SKILL=${skill}
    args:
      - id: skill
        name: Skill
        type: enum
        required: true
        options_from:
          command: ls skills/
          timeout_seconds: 5
    output:
      format: ansi
      display: checklist

configs:
  - path: .env
    name: Environment
    format: env
    editable: true
    danger: caution
    restart_required: true
    restart_command: start

  - path: allowlist.txt
    format: line-list
    line_list:
      item_label: domain
      pattern: "^[a-z.]+$"
      example: api.example.com

health:
  - id: status-badge
    name: Status
    command: echo running
    interval_seconds: 10
    timeout_seconds: 5
    parse:
      type: regex
      expression: "^(.+)$"
      format: "{value}"
    thresholds:
      green: "== running"
      red: "== stopped"
"##;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();

        // Identity
        assert_eq!(manifest.identity.id, "my-runtime");
        assert_eq!(manifest.identity.role, Role::Runtime);
        assert_eq!(manifest.identity.icon.as_deref(), Some("shield"));
        assert_eq!(manifest.identity.color.as_deref(), Some("#dc2626"));

        // Status
        let status = manifest.status.unwrap();
        assert_eq!(status.states.len(), 2);
        assert_eq!(status.probes.len(), 1);
        assert_eq!(status.probes[0].rules.len(), 2);
        assert_eq!(status.default_state.as_deref(), Some("stopped"));

        // Commands
        assert_eq!(manifest.commands.len(), 3);

        let start = &manifest.commands[0];
        assert_eq!(start.id, "start");
        assert_eq!(start.group, CommandGroup::Lifecycle);
        assert_eq!(start.r#type, CommandType::Action);
        assert_eq!(start.danger, Danger::Safe);
        assert_eq!(start.available_when, vec!["stopped"]);
        let output = start.output.as_ref().unwrap();
        assert_eq!(output.display, OutputDisplay::Terminal);

        let nuke = &manifest.commands[1];
        assert_eq!(nuke.danger, Danger::Destructive);

        let scan = &manifest.commands[2];
        assert_eq!(scan.args.len(), 1);
        assert_eq!(scan.args[0].r#type, ArgType::Enum);
        assert!(scan.args[0].options_from.is_some());
        assert_eq!(scan.args[0].options_from.as_ref().unwrap().command, "ls skills/");

        // Configs
        assert_eq!(manifest.configs.len(), 2);
        assert_eq!(manifest.configs[0].format, ConfigFormat::Env);
        assert!(manifest.configs[0].restart_required);
        assert_eq!(manifest.configs[0].restart_command.as_deref(), Some("start"));
        assert_eq!(manifest.configs[1].format, ConfigFormat::LineList);
        assert!(manifest.configs[1].line_list.is_some());

        // Health
        assert_eq!(manifest.health.len(), 1);
        assert_eq!(manifest.health[0].parse.r#type, ParseType::Regex);
        assert!(manifest.health[0].thresholds.is_some());
    }

    #[test]
    fn test_parse_all_roles() {
        for (role_str, expected) in [
            ("runtime", Role::Runtime),
            ("toolchain", Role::Toolchain),
            ("network", Role::Network),
            ("placeholder", Role::Placeholder),
        ] {
            let yaml = format!(
                "identity:\n  id: test\n  name: Test\n  version: '0.1.0'\n  description: Test\n  role: {}",
                role_str
            );
            let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(manifest.identity.role, expected);
        }
    }

    #[test]
    fn test_parse_all_output_displays() {
        let displays = [
            ("log", OutputDisplay::Log),
            ("table", OutputDisplay::Table),
            ("badge", OutputDisplay::Badge),
            ("checklist", OutputDisplay::Checklist),
            ("card-grid", OutputDisplay::CardGrid),
            ("terminal", OutputDisplay::Terminal),
            ("report", OutputDisplay::Report),
        ];
        for (display_str, expected) in displays {
            let yaml = format!(
                "identity:\n  id: t\n  name: T\n  version: '0.1.0'\n  description: T\n  role: runtime\ncommands:\n  - id: cmd\n    name: Cmd\n    command: echo\n    output:\n      display: {}",
                display_str
            );
            let manifest: Manifest = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(manifest.commands[0].output.as_ref().unwrap().display, expected);
        }
    }

    #[test]
    fn test_default_values() {
        let yaml = r##"
identity:
  id: test
  name: Test
  version: "0.1.0"
  description: Test
  role: runtime
commands:
  - id: cmd
    name: Cmd
    command: echo hello
"##;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        let cmd = &manifest.commands[0];
        assert_eq!(cmd.group, CommandGroup::Operations); // default
        assert_eq!(cmd.r#type, CommandType::Action); // default
        assert_eq!(cmd.danger, Danger::Safe); // default
        assert_eq!(cmd.sort_order, 100); // default
        assert_eq!(cmd.timeout_seconds, 60); // default
        assert!(cmd.available_when.is_empty()); // default
        assert!(cmd.args.is_empty()); // default
    }

    // =========================================================================
    // Real manifest parsing tests (against actual component.yml files)
    // =========================================================================

    #[test]
    fn test_parse_openclaw_vault_manifest() {
        let manifest_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../components/openclaw-vault/component.yml");
        if !std::path::Path::new(manifest_path).exists() {
            // Skip if running outside monorepo context
            return;
        }
        let content = std::fs::read_to_string(manifest_path).unwrap();
        let manifest: Manifest = serde_yaml::from_str(&content).unwrap();
        assert_eq!(manifest.identity.id, "openclaw-vault");
        assert_eq!(manifest.identity.role, Role::Runtime);
        assert!(!manifest.commands.is_empty());
        assert!(!manifest.configs.is_empty());
        assert!(!manifest.health.is_empty());
    }

    #[test]
    fn test_parse_clawhub_forge_manifest() {
        let manifest_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../components/clawhub-forge/component.yml");
        if !std::path::Path::new(manifest_path).exists() {
            return;
        }
        let content = std::fs::read_to_string(manifest_path).unwrap();
        let manifest: Manifest = serde_yaml::from_str(&content).unwrap();
        assert_eq!(manifest.identity.id, "clawhub-forge");
        assert_eq!(manifest.identity.role, Role::Toolchain);
        // Should have commands with options_from
        let has_options_from = manifest.commands.iter().any(|c| c.args.iter().any(|a| a.options_from.is_some()));
        assert!(has_options_from, "clawhub-forge should have commands with options_from");
    }

    #[test]
    fn test_parse_moltbook_pioneer_manifest() {
        let manifest_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../components/moltbook-pioneer/component.yml");
        if !std::path::Path::new(manifest_path).exists() {
            return;
        }
        let content = std::fs::read_to_string(manifest_path).unwrap();
        let manifest: Manifest = serde_yaml::from_str(&content).unwrap();
        assert_eq!(manifest.identity.id, "moltbook-pioneer");
        assert_eq!(manifest.identity.role, Role::Network);
        assert!(!manifest.commands.is_empty());
    }

    // =========================================================================
    // Prerequisites parsing tests
    // =========================================================================

    #[test]
    fn test_parse_manifest_without_prerequisites() {
        let yaml = r##"
identity:
  id: no-prereqs
  name: No Prerequisites
  version: "0.1.0"
  description: A component with no prerequisites section
  role: runtime
"##;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert!(manifest.prerequisites.is_none());
    }

    #[test]
    fn test_parse_manifest_with_prerequisites() {
        let yaml = r##"
identity:
  id: with-prereqs
  name: With Prerequisites
  version: "0.1.0"
  description: A component with prerequisites
  role: runtime

prerequisites:
  container_runtime: true
  setup_command: setup
  config_files:
    - path: .env
      template: .env.example
      description: Environment configuration
    - path: config.yml
  check_command: docker ps
"##;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        let prereqs = manifest.prerequisites.unwrap();
        assert!(prereqs.container_runtime);
        assert_eq!(prereqs.setup_command.as_deref(), Some("setup"));
        assert_eq!(prereqs.check_command.as_deref(), Some("docker ps"));
        assert_eq!(prereqs.config_files.len(), 2);

        let cf0 = &prereqs.config_files[0];
        assert_eq!(cf0.path, ".env");
        assert_eq!(cf0.template.as_deref(), Some(".env.example"));
        assert_eq!(cf0.description.as_deref(), Some("Environment configuration"));

        let cf1 = &prereqs.config_files[1];
        assert_eq!(cf1.path, "config.yml");
        assert!(cf1.template.is_none());
        assert!(cf1.description.is_none());
    }

    #[test]
    fn test_real_manifests_have_prerequisites() {
        for (component, expect_container) in [
            ("openclaw-vault", true),
            ("clawhub-forge", false),
            ("moltbook-pioneer", false),
        ] {
            let manifest_path = format!(
                "{}/../../components/{}/component.yml",
                env!("CARGO_MANIFEST_DIR"),
                component
            );
            if !std::path::Path::new(&manifest_path).exists() {
                continue;
            }
            let content = std::fs::read_to_string(&manifest_path).unwrap();
            let manifest: Manifest = serde_yaml::from_str(&content).unwrap();
            let prereqs = manifest.prerequisites.expect(
                &format!("{} should have prerequisites section", component),
            );
            assert_eq!(
                prereqs.container_runtime, expect_container,
                "{} container_runtime mismatch", component
            );
            assert!(
                prereqs.setup_command.is_some(),
                "{} should have setup_command", component
            );
            assert!(
                prereqs.check_command.is_some(),
                "{} should have check_command", component
            );
        }
    }

    // =========================================================================
    // Argument interpolation tests
    // =========================================================================

    #[test]
    fn test_interpolation_basic() {
        let cmd = "make scan SKILL=${skill}";
        let mut args = HashMap::new();
        args.insert("skill".to_string(), "my-skill".to_string());
        let result = super::super::runner::interpolate_args_for_test(cmd, &args);
        assert_eq!(result, "make scan SKILL='my-skill'");
    }

    #[test]
    fn test_interpolation_prevents_injection() {
        let cmd = "make scan SKILL=${skill}";
        let mut args = HashMap::new();
        args.insert("skill".to_string(), "foo; rm -rf /".to_string());
        let result = super::super::runner::interpolate_args_for_test(cmd, &args);
        // The malicious payload should be wrapped in single quotes
        assert_eq!(result, "make scan SKILL='foo; rm -rf /'");
        // Verify the value IS single-quoted (starts with ' after SKILL=)
        assert!(result.contains("SKILL='foo"), "Value must be single-quoted");
    }

    #[test]
    fn test_interpolation_with_single_quotes() {
        let cmd = "echo ${msg}";
        let mut args = HashMap::new();
        args.insert("msg".to_string(), "it's a test".to_string());
        let result = super::super::runner::interpolate_args_for_test(cmd, &args);
        assert_eq!(result, "echo 'it'\\''s a test'");
    }

    #[test]
    fn test_interpolation_no_args() {
        let cmd = "make verify";
        let args = HashMap::new();
        let result = super::super::runner::interpolate_args_for_test(cmd, &args);
        assert_eq!(result, "make verify");
    }

    // =========================================================================
    // Discovery tests
    // =========================================================================

    #[test]
    fn test_discover_components_in_monorepo() {
        // Try to find monorepo root
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let monorepo_root = manifest_dir.parent().unwrap(); // app/ -> opentrapp/

        if !monorepo_root.join("components").exists() {
            return;
        }

        let components = super::super::discovery::discover_components(monorepo_root).unwrap();
        assert!(components.len() >= 3, "Should find at least 3 component manifests");

        // Should be sorted by ID
        let ids: Vec<&str> = components.iter().map(|c| c.manifest.identity.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "Components should be sorted by ID");
    }

    // =========================================================================
    // Shell detection tests
    // =========================================================================

    #[test]
    fn test_find_bash() {
        let bash = crate::util::shell::find_bash();
        // On CI this might not exist, so just check the function doesn't panic
        if let Some(path) = bash {
            assert!(path.exists(), "Found bash path should exist");
            let path_str = path.to_string_lossy();
            assert!(
                path_str.contains("bash"),
                "Path should contain 'bash': {}",
                path_str
            );
        }
    }
}
