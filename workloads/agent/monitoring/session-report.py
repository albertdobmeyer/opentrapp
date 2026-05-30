#!/usr/bin/env python3
"""
OpenCli-Container: Session Report Generator — Post-Session Summary

Parses session transcript .jsonl files and produces a human-readable
summary of what happened during an agent session. This answers:
"What did Hum do?"

Input: Session transcript .jsonl files from inside the vault container.
       Located at ~/.openclaw/agents/main/sessions/*.jsonl

Output: Human-readable report to stdout. JSON mode available (--json).

Security notes:
  - All input is treated as UNTRUSTED (data comes from inside the container)
  - Uses json.loads() with no custom decoders
  - sanitize() strips control characters from all string fields
  - Never calls eval() or exec()
  - Malformed lines are counted and skipped
  - No file writes — output to stdout only

Usage:
  python3 monitoring/session-report.py                  # auto-detect from container
  python3 monitoring/session-report.py --file <path>    # parse a specific .jsonl file
  python3 monitoring/session-report.py --dir <path>     # parse all .jsonl files in directory
  python3 monitoring/session-report.py --json           # machine-readable output
"""

import json
import os
import subprocess
import sys
from collections import Counter
from datetime import datetime


def parse_args():
    """Parse command-line arguments without argparse (keep dependencies minimal)."""
    args = {
        "file": None,
        "dir": None,
        "json_output": False,
    }
    i = 1
    while i < len(sys.argv):
        arg = sys.argv[i]
        if arg == "--file" and i + 1 < len(sys.argv):
            args["file"] = sys.argv[i + 1]
            i += 2
        elif arg == "--dir" and i + 1 < len(sys.argv):
            args["dir"] = sys.argv[i + 1]
            i += 2
        elif arg == "--json":
            args["json_output"] = True
            i += 1
        elif arg in ("--help", "-h"):
            print(__doc__.strip())
            sys.exit(0)
        else:
            print(f"Unknown argument: {arg}", file=sys.stderr)
            print("Use --help for usage.", file=sys.stderr)
            sys.exit(1)
    return args


def sanitize(s):
    """Strip control characters from untrusted strings before terminal display.

    Prevents terminal injection via crafted content in session transcripts.
    """
    if not isinstance(s, str):
        return str(s)
    return "".join(c if (c >= " " and c != "\x7f") or c in ("\t", "\n") else "?" for c in s)


def find_transcript_source():
    """Auto-detect session transcripts from the running container."""
    runtime = "podman"
    try:
        subprocess.run(
            ["podman", "--version"],
            capture_output=True,
            check=True,
            timeout=5,
        )
    except (FileNotFoundError, subprocess.CalledProcessError):
        runtime = "docker"

    try:
        result = subprocess.run(
            [runtime, "exec", "opencli-container", "sh", "-c",
             "cat /home/vault/.openclaw/agents/main/sessions/*.jsonl 2>/dev/null"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass

    return None


def load_from_dir(dir_path):
    """Load all .jsonl files from a directory."""
    raw_parts = []
    if not os.path.isdir(dir_path):
        print(f"Error: directory not found: {dir_path}", file=sys.stderr)
        sys.exit(1)
    for fname in sorted(os.listdir(dir_path)):
        if fname.endswith(".jsonl"):
            fpath = os.path.join(dir_path, fname)
            with open(fpath) as f:
                raw_parts.append(f.read())
    return "\n".join(raw_parts)


def parse_timestamp(ts_str):
    """Parse timestamp string to datetime. Returns None on failure."""
    if not isinstance(ts_str, str):
        return None
    for fmt in ("%Y-%m-%dT%H:%M:%S.%f%z", "%Y-%m-%dT%H:%M:%S%z",
                "%Y-%m-%dT%H:%M:%S.%f", "%Y-%m-%dT%H:%M:%S"):
        try:
            return datetime.strptime(ts_str[:26], fmt[:len(fmt)])
        except ValueError:
            continue
    # Fallback: try just the first 19 chars as ISO
    try:
        return datetime.strptime(ts_str[:19], "%Y-%m-%dT%H:%M:%S")
    except ValueError:
        return None


def extract_text(content):
    """Extract text from message content (string or array of blocks)."""
    if isinstance(content, str):
        return content.strip()
    if isinstance(content, list):
        parts = []
        for block in content:
            if isinstance(block, dict):
                if block.get("type") == "text":
                    parts.append(block.get("text", "").strip())
                elif block.get("type") in ("tool_use", "toolCall"):
                    # Tool call embedded in content blocks
                    # OpenClaw uses "toolCall", Anthropic API uses "tool_use"
                    parts.append(f"[tool: {block.get('name', '?')}]")
                elif block.get("type") in ("tool_result", "toolResult"):
                    parts.append("[tool_result]")
        return " ".join(parts)
    return ""


def analyze_transcripts(raw_content):
    """Parse and analyze session transcript content."""
    entries = []
    malformed = 0

    for line in raw_content.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            entry = json.loads(line)
            if not isinstance(entry, dict):
                malformed += 1
                continue
            entries.append(entry)
        except (json.JSONDecodeError, ValueError):
            malformed += 1

    if not entries:
        return None, malformed

    # --- Extract timestamps for duration ---
    timestamps = []
    for entry in entries:
        ts = parse_timestamp(entry.get("timestamp", ""))
        if ts:
            timestamps.append(ts)

    # --- Classify entries ---
    user_messages = 0
    assistant_messages = 0
    system_messages = 0
    tool_calls = Counter()         # tool_name → count
    exec_commands = []             # list of (timestamp, command) tuples
    approval_allowed = 0
    approval_denied = 0
    files_mentioned = set()
    heartbeats = 0
    total_entries = len(entries)

    for entry in entries:
        etype = entry.get("type", "")

        if etype == "message":
            msg = entry.get("message", {})
            role = msg.get("role", "")
            content = msg.get("content", "")
            text = extract_text(content)
            ts_str = sanitize(entry.get("timestamp", "")[:19])

            # Count by role
            if role == "user":
                # Check for heartbeats
                if "HEARTBEAT" in text[:50]:
                    heartbeats += 1
                    continue
                user_messages += 1
            elif role == "assistant":
                assistant_messages += 1

                # Extract tool call blocks from assistant content.
                # OpenClaw uses "toolCall" with "arguments"; Anthropic API
                # uses "tool_use" with "input". We handle both.
                if isinstance(content, list):
                    for block in content:
                        if not isinstance(block, dict):
                            continue
                        block_type = block.get("type", "")
                        if block_type not in ("tool_use", "toolCall"):
                            continue

                        tool_name = sanitize(block.get("name", "unknown"))
                        tool_calls[tool_name] += 1

                        # Get tool arguments (OpenClaw: "arguments", Anthropic: "input")
                        tool_args = block.get("arguments", {}) or block.get("input", {})
                        if not isinstance(tool_args, dict):
                            tool_args = {}

                        # Extract exec commands specifically
                        if tool_name in ("exec", "bash", "Bash"):
                            cmd = sanitize(tool_args.get("command", ""))
                            if cmd:
                                exec_commands.append((ts_str, cmd))

                        # Track file operations
                        if tool_name in ("read", "write", "edit", "apply_patch"):
                            path = tool_args.get("path", "") or tool_args.get("file_path", "")
                            if path:
                                files_mentioned.add(sanitize(path))

            elif role == "toolResult":
                # OpenClaw tool results are messages with role "toolResult".
                # isError captures all tool errors (denied, failed, timeout),
                # not just explicit user denials. Label accordingly.
                is_error = msg.get("isError", False)
                if is_error:
                    approval_denied += 1  # Tool error (denial, failure, or timeout)
                else:
                    approval_allowed += 1  # Tool succeeded

            elif role == "system":
                system_messages += 1

    # --- Build report ---
    report = {
        "total_entries": total_entries,
        "malformed_lines": malformed,
        "heartbeats_filtered": heartbeats,
        "duration": None,
        "time_start": None,
        "time_end": None,
        "messages": {
            "user": user_messages,
            "assistant": assistant_messages,
            "system": system_messages,
            "total": user_messages + assistant_messages + system_messages,
        },
        "tool_calls": dict(tool_calls.most_common()),
        "total_tool_calls": sum(tool_calls.values()),
        "exec_commands": exec_commands,
        "files_mentioned": sorted(files_mentioned),
        "approvals": {
            "allowed": approval_allowed,
            "denied": approval_denied,
        },
    }

    if timestamps:
        sorted_ts = sorted(timestamps)
        report["time_start"] = sorted_ts[0].strftime("%Y-%m-%d %H:%M:%S")
        report["time_end"] = sorted_ts[-1].strftime("%Y-%m-%d %H:%M:%S")
        duration = sorted_ts[-1] - sorted_ts[0]
        total_seconds = duration.total_seconds()
        if total_seconds >= 3600:
            report["duration"] = f"{total_seconds / 3600:.1f} hours"
        elif total_seconds >= 60:
            report["duration"] = f"{total_seconds / 60:.0f} minutes"
        else:
            report["duration"] = f"{total_seconds:.0f} seconds"

    return report, malformed


def format_report(report):
    """Print human-readable report to stdout."""
    bold = "\033[1m"
    nc = "\033[0m"
    green = "\033[0;32m"
    yellow = "\033[0;33m"
    cyan = "\033[0;36m"

    print()
    print(f"{bold}OpenCli-Container: Session Report{nc}")
    print("=" * 40)

    # --- Duration ---
    print()
    print(f"{bold}Session{nc}")
    if report["time_start"]:
        print(f"  Start:    {report['time_start']}")
        print(f"  End:      {report['time_end']}")
        print(f"  Duration: {report['duration']}")
    else:
        print("  No timestamps found")

    print(f"  Transcript entries: {report['total_entries']}")
    if report["malformed_lines"] > 0:
        print(f"  {yellow}Malformed lines: {report['malformed_lines']}{nc}")
    if report["heartbeats_filtered"] > 0:
        print(f"  Heartbeats filtered: {report['heartbeats_filtered']}")

    # --- Messages ---
    print()
    print(f"{bold}Conversation{nc}")
    print(f"  User messages:      {report['messages']['user']}")
    print(f"  Assistant responses: {report['messages']['assistant']}")
    if report["messages"]["system"] > 0:
        print(f"  System messages:    {report['messages']['system']}")
    print(f"  Total messages:     {report['messages']['total']}")

    # --- Tool usage ---
    print()
    print(f"{bold}Tool Usage{nc}")
    if report["tool_calls"]:
        print(f"  Total tool invocations: {report['total_tool_calls']}")
        print()
        for tool, count in report["tool_calls"].items():
            print(f"    {count:>4}x  {tool}")
    else:
        print(f"  {green}No tools were invoked this session.{nc}")

    # --- Exec commands ---
    if report["exec_commands"]:
        print()
        print(f"{bold}Commands Executed{nc}")
        for ts, cmd in report["exec_commands"]:
            # Truncate very long commands for display
            display_cmd = cmd[:200] + "..." if len(cmd) > 200 else cmd
            print(f"  [{ts}] {cyan}{display_cmd}{nc}")

    # --- Files ---
    if report["files_mentioned"]:
        print()
        print(f"{bold}Files Accessed{nc}")
        for fpath in report["files_mentioned"]:
            print(f"    {fpath}")

    # --- Tool Results ---
    if report["approvals"]["allowed"] > 0 or report["approvals"]["denied"] > 0:
        print()
        print(f"{bold}Tool Results{nc}")
        print(f"  Succeeded: {report['approvals']['allowed']}")
        if report["approvals"]["denied"] > 0:
            print(f"  {yellow}Failed/Denied: {report['approvals']['denied']}{nc}")

    # --- Assessment ---
    print()
    print(f"{bold}Assessment{nc}")
    issues = []
    if report["malformed_lines"] > 0:
        issues.append(f"{report['malformed_lines']} malformed transcript lines")
    if report["approvals"]["denied"] > 0:
        issues.append(f"{report['approvals']['denied']} tool calls failed or were denied")

    if issues:
        for issue in issues:
            print(f"  {yellow}NOTE: {issue}{nc}")
    else:
        print(f"  {green}Session completed normally. No anomalies detected.{nc}")

    print()
    print("=" * 40)
    print(f"  Tip: Run 'make network-report' for network activity analysis.")
    print()


def main():
    args = parse_args()

    # --- Load transcript data ---
    if args["file"]:
        if not os.path.isfile(args["file"]):
            print(f"Error: file not found: {args['file']}", file=sys.stderr)
            sys.exit(1)
        with open(args["file"]) as f:
            raw = f.read()
    elif args["dir"]:
        raw = load_from_dir(args["dir"])
    else:
        raw = find_transcript_source()
        if raw is None:
            print("Error: no session transcripts found.", file=sys.stderr)
            print("  Is the opencli-container container running?", file=sys.stderr)
            print("  Try: python3 monitoring/session-report.py --file <path-to-session.jsonl>", file=sys.stderr)
            sys.exit(1)

    if not raw.strip():
        if args["json_output"]:
            print(json.dumps({"error": "empty transcript"}, indent=2))
        else:
            print("Session transcript is empty — no activity to report.")
        sys.exit(0)

    # --- Analyze ---
    report, malformed = analyze_transcripts(raw)

    if report is None:
        if args["json_output"]:
            print(json.dumps({"error": "no valid entries", "malformed_lines": malformed}, indent=2))
        else:
            print(f"No valid entries in transcript ({malformed} malformed lines).")
        sys.exit(1)

    # --- Output ---
    if args["json_output"]:
        print(json.dumps(report, indent=2, default=str))
    else:
        format_report(report)

    sys.exit(0)


if __name__ == "__main__":
    main()
