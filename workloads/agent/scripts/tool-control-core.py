#!/usr/bin/env python3
"""
OpenCli-Container: Tool Control Core — Config Generator

Reads the tool manifest and a user selection, then generates the correct
openclaw.json config and proxy allowlist. This is the security-critical
logic that determines what the agent can do.

All output is plain JSON (not JSON5). OpenClaw accepts both but JSON
is programmatically validatable.

This script is called by tool-control.sh. It should NOT be run directly
by users — the bash wrapper handles CLI, user interaction, and container ops.

Usage (called by tool-control.sh):
  python3 scripts/tool-control-core.py --manifest config/tool-manifest.yml --preset hard
  python3 scripts/tool-control-core.py --manifest config/tool-manifest.yml --preset split
  python3 scripts/tool-control-core.py --manifest config/tool-manifest.yml --preset split --enable web_search --disable exec
  python3 scripts/tool-control-core.py --manifest config/tool-manifest.yml --from-file tools.conf
  python3 scripts/tool-control-core.py --manifest config/tool-manifest.yml --preset split --status-json <current-config-json>

Output modes (to stdout):
  --output config    → generated openclaw.json (default)
  --output allowlist → generated proxy allowlist (one domain per line)
  --output risk      → risk assessment JSON
  --output status    → tool status table JSON (requires --status-json)

Security notes:
  - Enforces NEVER-enable and NEVER-safebins lists from the manifest
  - Validates all inputs against the manifest (unknown tools rejected)
  - Generates deny-by-default configs (only enabled tools survive)
  - All logic runs on the HOST, never inside the container
"""

import json
import sys

try:
    import yaml
except ImportError:
    print("ERROR: pyyaml not installed. Run: pip3 install pyyaml", file=sys.stderr)
    sys.exit(1)


def load_manifest(path):
    """Load and validate the tool manifest."""
    with open(path) as f:
        m = yaml.safe_load(f)
    if m.get("version") != 1:
        raise ValueError(f"Unsupported manifest version: {m.get('version')}")
    return m


def parse_selection(manifest, preset_name=None, enables=None, disables=None, from_file=None):
    """Parse user selection into a set of enabled tool names."""
    enables = enables or []
    disables = disables or []
    tools = manifest["tools"]
    never_enable = set(manifest["never_enable"])

    # Start from preset if specified
    if preset_name:
        preset = manifest["presets"].get(preset_name)
        if not preset:
            raise ValueError(f"Unknown preset: {preset_name}. Valid: {list(manifest['presets'].keys())}")
        enabled = set(preset["enabled_tools"])
    elif from_file:
        enabled = set()
        with open(from_file) as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                if line.startswith("-"):
                    disables.append(line[1:].strip())
                elif line.startswith("+"):
                    enables.append(line[1:].strip())
                else:
                    enables.append(line.strip())
    else:
        enabled = set()

    # Apply enables
    for tool in enables:
        if tool not in tools:
            raise ValueError(f"Unknown tool: {tool}. Valid tools: {sorted(tools.keys())}")
        if tool in never_enable:
            raise ValueError(
                f"SECURITY: Tool '{tool}' is in the NEVER-enable list and cannot be enabled. "
                f"NEVER-enable: {sorted(never_enable)}"
            )
        if tools[tool]["requires_profile"] == "never":
            raise ValueError(
                f"SECURITY: Tool '{tool}' has requires_profile=never and cannot be enabled."
            )
        enabled.add(tool)

    # Apply disables
    for tool in disables:
        if tool not in tools:
            raise ValueError(f"Unknown tool: {tool}. Valid tools: {sorted(tools.keys())}")
        enabled.discard(tool)

    # Final safety check: no NEVER-enable tools in the set
    violations = enabled & never_enable
    if violations:
        raise ValueError(f"SECURITY: NEVER-enable tools in selection: {sorted(violations)}")

    return enabled


def determine_profile(manifest, enabled_tools):
    """Determine the minimum profile that covers all enabled tools."""
    tools = manifest["tools"]

    # Check what profiles are needed
    needs_coding = False
    needs_messaging = False

    for name in enabled_tools:
        tool = tools[name]
        req = tool["requires_profile"]
        if req == "coding":
            needs_coding = True
        elif req == "messaging":
            needs_messaging = True

    # Profile hierarchy: minimal < messaging < coding < full
    # We use the minimum that covers everything
    if needs_coding:
        return "coding"
    if needs_messaging:
        return "messaging"
    return "minimal"


def build_deny_list(manifest, enabled_tools):
    """Build the deny list: everything NOT enabled gets denied."""
    tools = manifest["tools"]
    groups = manifest["groups"]
    never_enable = set(manifest["never_enable"])

    deny = set()

    # Deny every tool not in the enabled set
    for name in tools:
        if name not in enabled_tools:
            deny.add(name)

    # Always deny NEVER-enable tools
    deny.update(never_enable)

    # Add group denials where ALL members of a group are denied
    for group_name, members in groups.items():
        if all(m in deny for m in members):
            deny.add(f"group:{group_name}")

    return sorted(deny)


def build_safebins(manifest, preset_name, enabled_tools):
    """Build safeBins and safeBinProfiles from preset or defaults."""
    never_safebins = set(manifest["never_safebins"])

    # If we have a preset, use its safeBins
    if preset_name and preset_name in manifest["presets"]:
        preset = manifest["presets"][preset_name]
        safebins = list(preset.get("safeBins", []))
    else:
        # Default: no safeBins unless exec is enabled
        safebins = []

    # If exec is not enabled, no safeBins needed
    if "exec" not in enabled_tools:
        safebins = []

    # Safety check: no NEVER-safebins
    violations = set(safebins) & never_safebins
    if violations:
        raise ValueError(f"SECURITY: NEVER-safebins in selection: {sorted(violations)}")

    # Build matching profiles (empty = no argument restrictions)
    profiles = {b: {} for b in safebins}

    return safebins, profiles


def build_allowlist(manifest, enabled_tools, preset_name=None):
    """Build the proxy domain allowlist."""
    tools = manifest["tools"]
    domains = list(manifest["base_domains"])

    # Add per-tool extra domains
    for name in enabled_tools:
        tool = tools[name]
        for domain in tool.get("extra_domains", []):
            if domain not in domains:
                domains.append(domain)

    # Add preset-level extra domains
    if preset_name and preset_name in manifest["presets"]:
        for domain in manifest["presets"][preset_name].get("extra_domains", []):
            if domain not in domains:
                domains.append(domain)

    return domains


def compute_risk_score(manifest, enabled_tools):
    """Compute the risk score (0.0-0.9) based on enabled tools."""
    tools = manifest["tools"]
    score = 0.0
    for name in enabled_tools:
        tool = tools[name]
        score += tool.get("risk_score", 0)
    return min(round(score, 3), 0.9)


def build_also_allow(manifest, preset_name, enabled_tools):
    """Build the alsoAllow list from a preset's also_allow field.

    F11: Some OpenClaw tools (web_fetch, web_search, cron, canvas, message)
    have empty profiles in OpenClaw's tool catalog, so they're never
    auto-allowed by any profile. The preset's also_allow field names them
    explicitly. We intersect with enabled_tools as defense-in-depth: if a
    tool was disabled via --disable, it must not survive in alsoAllow.
    NEVER-enable tools are filtered out as a final safety check.
    """
    if not preset_name or preset_name not in manifest["presets"]:
        return []
    preset_also = manifest["presets"][preset_name].get("also_allow", []) or []
    never_enable = set(manifest["never_enable"])
    result = sorted(t for t in preset_also if t in enabled_tools and t not in never_enable)
    return result


def generate_config(manifest, enabled_tools, preset_name=None):
    """Generate the complete openclaw.json config."""
    invariants = manifest["invariants"]
    profile = determine_profile(manifest, enabled_tools)
    deny_list = build_deny_list(manifest, enabled_tools)
    safebins, safebin_profiles = build_safebins(manifest, preset_name, enabled_tools)
    also_allow = build_also_allow(manifest, preset_name, enabled_tools)
    exec_enabled = "exec" in enabled_tools

    # Get exec settings from preset or defaults
    if preset_name and preset_name in manifest["presets"]:
        preset = manifest["presets"][preset_name]
        exec_security = preset.get("exec_security", "deny")
        exec_ask = preset.get("exec_ask", "always")
        exec_host = preset.get("exec_host", "sandbox")
    elif exec_enabled:
        exec_security = "allowlist"
        exec_ask = "always"
        exec_host = "gateway"
    else:
        exec_security = "deny"
        exec_ask = "always"
        exec_host = "sandbox"

    config = {
        "agents": {
            "defaults": {
                "model": {
                    "primary": "anthropic/claude-haiku-4-5",
                },
                "sandbox": {
                    "mode": invariants["agents.defaults.sandbox.mode"],
                },
            },
        },
        "tools": {
            "profile": profile,
            "deny": deny_list,
            "exec": {
                "security": exec_security,
                "ask": exec_ask,
                # Note: askFallback is documented but not supported by OpenClaw 2026.2.26
                # (Zod rejects it as "Unrecognized key"). Omitted from generated config.
                "host": exec_host,
            },
            "elevated": {
                "enabled": invariants["tools.elevated.enabled"],
            },
            "fs": {
                "workspaceOnly": invariants["tools.fs.workspaceOnly"],
            },
        },
        "gateway": {
            "mode": invariants["gateway.mode"],
            "bind": invariants["gateway.bind"],
        },
        "session": {
            "dmScope": "per-channel-peer",
        },
        "channels": {
            "telegram": {
                "dmPolicy": invariants["channels.telegram.dmPolicy"],
                "proxy": "http://vault-proxy:8080",
            },
            "whatsapp": {
                "enabled": invariants["channels.whatsapp.enabled"],
                "dmPolicy": "pairing",
            },
        },
        "logging": {
            "redactSensitive": invariants["logging.redactSensitive"],
        },
    }

    # Add safeBins only if exec is enabled and we have bins
    if safebins:
        config["tools"]["exec"]["safeBins"] = safebins
        config["tools"]["exec"]["safeBinProfiles"] = safebin_profiles

    # Emit alsoAllow only when the preset declared one (F11).
    if also_allow:
        config["tools"]["alsoAllow"] = also_allow

    return config


def generate_risk_assessment(manifest, enabled_tools):
    """Generate a risk assessment for the current tool selection."""
    tools = manifest["tools"]
    score = compute_risk_score(manifest, enabled_tools)

    assessment = {
        "risk_score": score,
        "enabled_count": len(enabled_tools),
        "total_tools": len(tools),
        "tools": {},
    }

    for name, tool in sorted(tools.items()):
        status = "ENABLED" if name in enabled_tools else "DENIED"
        if tool["requires_profile"] == "never":
            status = "NEVER"
        assessment["tools"][name] = {
            "status": status,
            "risk": tool["risk"],
            "description": tool["description"],
            "injection_vectors": tool["injection_vectors"],
        }

    return assessment


def generate_status(manifest, current_config_json):
    """Analyze a running config and report per-tool status."""
    tools = manifest["tools"]

    try:
        config = json.loads(current_config_json)
    except json.JSONDecodeError as e:
        raise ValueError(f"Invalid JSON in current config: {e}")

    profile = config.get("tools", {}).get("profile", "unknown")
    deny_list = set(config.get("tools", {}).get("deny", []))
    exec_security = config.get("tools", {}).get("exec", {}).get("security", "unknown")
    safebins = config.get("tools", {}).get("exec", {}).get("safeBins", [])

    status = {
        "profile": profile,
        "exec_security": exec_security,
        "safeBins_count": len(safebins),
        "tools": {},
    }

    enabled_tools = set()
    for name, tool in sorted(tools.items()):
        # A tool is denied if it's in the deny list OR its group is denied
        group = tool.get("group", "none")
        is_denied = name in deny_list or f"group:{group}" in deny_list

        if tool["requires_profile"] == "never":
            tool_status = "NEVER"
        elif is_denied:
            tool_status = "DENIED"
        else:
            tool_status = "ENABLED"
            enabled_tools.add(name)

        status["tools"][name] = {
            "status": tool_status,
            "risk": tool["risk"],
            "description": tool["description"],
        }

    status["risk_score"] = compute_risk_score(manifest, enabled_tools)
    status["enabled_count"] = len(enabled_tools)

    return status


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Tool Control Core — Config Generator")
    parser.add_argument("--manifest", required=True, help="Path to tool-manifest.yml")
    parser.add_argument("--preset", help="Shell preset (hard, split)")
    parser.add_argument("--enable", action="append", default=[], help="Enable a tool (repeatable)")
    parser.add_argument("--disable", action="append", default=[], help="Disable a tool (repeatable)")
    parser.add_argument("--from-file", help="Read tool selection from file")
    parser.add_argument("--output", choices=["config", "allowlist", "risk", "status"],
                        default="config", help="Output mode")
    parser.add_argument("--status-json", help="Current config JSON (for --output status)")

    args = parser.parse_args()

    # Load manifest
    try:
        manifest = load_manifest(args.manifest)
    except Exception as e:
        print(f"ERROR: Failed to load manifest: {e}", file=sys.stderr)
        sys.exit(1)

    # Status mode: analyze existing config
    if args.output == "status":
        if not args.status_json:
            print("ERROR: --status-json required for --output status", file=sys.stderr)
            sys.exit(1)
        try:
            status = generate_status(manifest, args.status_json)
            print(json.dumps(status, indent=2))
        except Exception as e:
            print(f"ERROR: {e}", file=sys.stderr)
            sys.exit(1)
        sys.exit(0)

    # Parse selection
    try:
        enabled = parse_selection(
            manifest,
            preset_name=args.preset,
            enables=args.enable,
            disables=args.disable,
            from_file=args.from_file,
        )
    except ValueError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        sys.exit(1)

    # Generate requested output
    if args.output == "config":
        config = generate_config(manifest, enabled, preset_name=args.preset)
        print(json.dumps(config, indent=2))

    elif args.output == "allowlist":
        domains = build_allowlist(manifest, enabled, preset_name=args.preset)
        header = "# Generated by tool-control. Manual edits are preserved until next make hard-shell or make split-shell.\n"
        header += f"# Preset: {args.preset or 'custom'}, Tools: {len(enabled)}\n"
        print(header)
        for domain in domains:
            print(domain)

    elif args.output == "risk":
        assessment = generate_risk_assessment(manifest, enabled)
        print(json.dumps(assessment, indent=2))


if __name__ == "__main__":
    main()
