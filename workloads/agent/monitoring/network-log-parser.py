#!/usr/bin/env python3
"""
OpenCli-Container: Network Log Parser — Anomaly Detection on Proxy Logs

Parses vault-proxy's requests.jsonl and flags security-relevant events.
This is the "always know what the agent is up to" tool for network activity.

Input: requests.jsonl (structured JSON, one entry per line)
       Written by vault-proxy.py inside the proxy container.

Output: Human-readable report to stdout. JSON mode available (--json).

Security notes:
  - All input is treated as UNTRUSTED (data comes from inside the container)
  - Uses json.loads() with no custom decoders
  - Never calls eval(), exec(), or subprocess
  - Malformed lines are counted and skipped, never crash the parser
  - No file writes — output to stdout only

Usage:
  python3 monitoring/network-log-parser.py                     # auto-detect log source
  python3 monitoring/network-log-parser.py --file <path>       # parse a specific file
  python3 monitoring/network-log-parser.py --json              # machine-readable output
  python3 monitoring/network-log-parser.py --threshold 51200   # custom large-payload threshold (bytes)
  python3 monitoring/network-log-parser.py --window 60         # frequency spike window (seconds)
  python3 monitoring/network-log-parser.py --spike 20          # requests per window to flag as spike
"""

import json
import os
import subprocess
import sys
from collections import Counter, defaultdict
from datetime import datetime

# --- Configuration ---

# Payload size threshold for flagging (default 50KB — well below the 1MB hard block)
DEFAULT_LARGE_PAYLOAD_THRESHOLD = 50 * 1024

# Time window for frequency spike detection (seconds)
DEFAULT_SPIKE_WINDOW = 60

# Requests per window to consider a spike
DEFAULT_SPIKE_THRESHOLD = 20

# Known valid action types from vault-proxy.py
KNOWN_ACTIONS = frozenset({
    "ALLOWED",
    "BLOCKED",
    "EXFIL_BLOCKED",
    "LARGE_RESPONSE_BLOCKED",
    "KEY_REFLECTED",
    "RESPONSE",
})


def parse_args():
    """Parse command-line arguments without argparse (keep dependencies minimal)."""
    args = {
        "file": None,
        "json_output": False,
        "threshold": DEFAULT_LARGE_PAYLOAD_THRESHOLD,
        "window": DEFAULT_SPIKE_WINDOW,
        "spike": DEFAULT_SPIKE_THRESHOLD,
    }
    i = 1
    while i < len(sys.argv):
        arg = sys.argv[i]
        if arg == "--file" and i + 1 < len(sys.argv):
            args["file"] = sys.argv[i + 1]
            i += 2
        elif arg == "--json":
            args["json_output"] = True
            i += 1
        elif arg == "--threshold" and i + 1 < len(sys.argv):
            args["threshold"] = int(sys.argv[i + 1])
            i += 2
        elif arg == "--window" and i + 1 < len(sys.argv):
            args["window"] = int(sys.argv[i + 1])
            i += 2
        elif arg == "--spike" and i + 1 < len(sys.argv):
            args["spike"] = int(sys.argv[i + 1])
            i += 2
        elif arg in ("--help", "-h"):
            print(__doc__.strip())
            sys.exit(0)
        else:
            print(f"Unknown argument: {arg}", file=sys.stderr)
            print("Use --help for usage.", file=sys.stderr)
            sys.exit(1)
    return args


def find_log_source():
    """Auto-detect the proxy log file. Try container exec first, then volume."""
    # Detect container runtime
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

    # Try reading from running proxy container
    try:
        result = subprocess.run(
            [runtime, "exec", "vault-proxy", "cat", "/var/log/vault-proxy/requests.jsonl"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass

    # Try reading from volume mount
    try:
        result = subprocess.run(
            [runtime, "volume", "inspect", "opencli-container_vault-proxy-logs",
             "--format", "{{.Mountpoint}}"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            vol_path = result.stdout.strip()
            log_path = os.path.join(vol_path, "requests.jsonl")
            if os.path.isfile(log_path):
                with open(log_path) as f:
                    return f.read()
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass

    return None


def parse_log_lines(raw_content):
    """Parse JSONL content into a list of entries. Skip malformed lines safely."""
    entries = []
    malformed = 0
    for line_num, line in enumerate(raw_content.splitlines(), 1):
        line = line.strip()
        if not line:
            continue
        # Strip the "[vault-proxy] " prefix that the stdout handler adds
        if line.startswith("[vault-proxy] "):
            line = line[len("[vault-proxy] "):]
        try:
            entry = json.loads(line)
            if not isinstance(entry, dict):
                malformed += 1
                continue
            entry["_line"] = line_num
            entries.append(entry)
        except (json.JSONDecodeError, ValueError):
            malformed += 1
    return entries, malformed


def parse_timestamp(ts_str):
    """Parse timestamp string to datetime. Returns None on failure."""
    if not isinstance(ts_str, str):
        return None
    # Format from vault-proxy.py: 2026-03-25T14:30:22+0100
    for fmt in ("%Y-%m-%dT%H:%M:%S%z", "%Y-%m-%dT%H:%M:%S"):
        try:
            return datetime.strptime(ts_str, fmt)
        except ValueError:
            continue
    return None


def analyze(entries, config):
    """Analyze log entries and produce findings."""
    findings = []
    stats = {
        "total_entries": len(entries),
        "by_action": Counter(),
        "domains_allowed": Counter(),
        "domains_blocked": Counter(),
        "total_request_bytes": 0,
        "total_response_bytes": 0,
        "timestamps": [],
    }

    for entry in entries:
        # Sanitize all string fields from untrusted log data.
        # This prevents terminal injection via crafted hostnames, URLs, etc.
        action = sanitize(entry.get("action", "UNKNOWN"))
        stats["by_action"][action] += 1

        host = sanitize(entry.get("host", ""))
        url = sanitize(entry.get("url", "?"))
        method = sanitize(entry.get("method", "?"))
        reason = sanitize(entry.get("reason", "no reason"))
        timestamp = sanitize(entry.get("timestamp", ""))
        ts = parse_timestamp(timestamp)
        if ts:
            stats["timestamps"].append(ts)

        # --- Check 1: Unknown action types ---
        if action not in KNOWN_ACTIONS:
            findings.append({
                "severity": "HIGH",
                "type": "unknown_action",
                "message": f"Unknown log action type: {action!r}",
                "line": entry.get("_line"),
                "timestamp": timestamp,
            })

        # --- Check 2: Blocked requests ---
        if action == "BLOCKED":
            stats["domains_blocked"][host] += 1
            findings.append({
                "severity": "MEDIUM",
                "type": "blocked_request",
                "message": f"Request blocked: {method} {url} — {reason}",
                "host": host,
                "timestamp": timestamp,
            })

        # --- Check 3: Exfiltration attempts ---
        if action == "EXFIL_BLOCKED":
            req_bytes = entry.get("request_bytes", 0)
            findings.append({
                "severity": "CRITICAL",
                "type": "exfiltration_attempt",
                "message": f"Exfiltration blocked: {req_bytes} bytes outbound to {url}",
                "bytes": req_bytes,
                "timestamp": timestamp,
            })

        # --- Check 4: Key reflection ---
        if action == "KEY_REFLECTED":
            findings.append({
                "severity": "CRITICAL",
                "type": "key_reflected",
                "message": f"API key reflected in response from {url} — redacted by proxy",
                "env_var": sanitize(entry.get("env_var", "?")),
                "timestamp": timestamp,
            })

        # --- Check 5: Large response blocked ---
        if action == "LARGE_RESPONSE_BLOCKED":
            resp_bytes = entry.get("response_bytes", 0)
            findings.append({
                "severity": "HIGH",
                "type": "large_response_blocked",
                "message": f"Oversized response blocked: {resp_bytes} bytes from {url}",
                "bytes": resp_bytes,
                "timestamp": timestamp,
            })

        # --- Check 6: Large outbound payloads (below hard block but suspicious) ---
        if action == "ALLOWED":
            stats["domains_allowed"][host] += 1
            req_bytes = entry.get("request_bytes", 0)
            if isinstance(req_bytes, int):
                stats["total_request_bytes"] += req_bytes
                if req_bytes > config["threshold"]:
                    findings.append({
                        "severity": "MEDIUM",
                        "type": "large_payload",
                        "message": f"Large outbound payload: {req_bytes} bytes to {host} ({url})",
                        "bytes": req_bytes,
                        "host": host,
                        "timestamp": timestamp,
                    })

        if action == "RESPONSE":
            resp_bytes = entry.get("response_bytes", 0)
            if isinstance(resp_bytes, int):
                stats["total_response_bytes"] += resp_bytes

    # --- Check 7: Request frequency spikes ---
    if stats["timestamps"]:
        sorted_ts = sorted(stats["timestamps"])
        window_seconds = config["window"]
        spike_threshold = config["spike"]
        # Sliding window: count requests in each window.
        # Skip ahead after each spike to avoid duplicate overlapping reports.
        skip_until = 0
        for i, ts in enumerate(sorted_ts):
            if i < skip_until:
                continue
            window_end = ts.timestamp() + window_seconds
            count = 0
            for j in range(i, len(sorted_ts)):
                if sorted_ts[j].timestamp() <= window_end:
                    count += 1
                else:
                    break
            if count >= spike_threshold:
                findings.append({
                    "severity": "MEDIUM",
                    "type": "frequency_spike",
                    "message": f"Request frequency spike: {count} requests in {window_seconds}s window starting at {ts.isoformat()}",
                    "count": count,
                    "window_seconds": window_seconds,
                    "timestamp": ts.isoformat(),
                })
                # Skip past this window to avoid overlapping reports
                skip_until = i + count

    return findings, stats


def sanitize(s):
    """Strip control characters from untrusted strings before terminal display.

    Prevents terminal injection via crafted hostnames or URLs in proxy logs.
    Allows printable ASCII and common Unicode but strips ANSI escapes, null bytes,
    and other control characters that could manipulate the terminal.
    """
    if not isinstance(s, str):
        return str(s)
    # Remove all ASCII control characters (0x00-0x1F, 0x7F) except tab and newline
    return "".join(c if (c >= " " and c != "\x7f") or c in ("\t", "\n") else "?" for c in s)


def format_bytes(n):
    """Format byte count for human display."""
    if not isinstance(n, (int, float)) or n < 0:
        return "0 B"
    if n < 1024:
        return f"{n} B"
    if n < 1024 * 1024:
        return f"{n / 1024:.1f} KB"
    return f"{n / (1024 * 1024):.1f} MB"


def print_report(findings, stats, malformed):
    """Print human-readable report to stdout."""
    # Severity colors
    colors = {
        "CRITICAL": "\033[1;31m",  # Bold red
        "HIGH": "\033[0;31m",      # Red
        "MEDIUM": "\033[0;33m",    # Yellow
        "LOW": "\033[0;36m",       # Cyan
    }
    nc = "\033[0m"
    bold = "\033[1m"

    print()
    print(f"{bold}OpenCli-Container: Network Log Analysis{nc}")
    print("=" * 40)

    # --- Summary ---
    print()
    print(f"{bold}Summary{nc}")
    print(f"  Total log entries:    {stats['total_entries']}")
    if malformed > 0:
        print(f"  {colors['MEDIUM']}Malformed lines:      {malformed}{nc}")
    print(f"  Requests allowed:     {stats['by_action'].get('ALLOWED', 0)}")
    print(f"  Requests blocked:     {stats['by_action'].get('BLOCKED', 0)}")
    print(f"  Responses logged:     {stats['by_action'].get('RESPONSE', 0)}")
    print(f"  Total outbound:       {format_bytes(stats['total_request_bytes'])}")
    print(f"  Total inbound:        {format_bytes(stats['total_response_bytes'])}")

    # --- Time range ---
    if stats["timestamps"]:
        sorted_ts = sorted(stats["timestamps"])
        duration = sorted_ts[-1] - sorted_ts[0]
        print(f"  Time range:           {sorted_ts[0].strftime('%Y-%m-%d %H:%M:%S')} → {sorted_ts[-1].strftime('%Y-%m-%d %H:%M:%S')}")
        hours = duration.total_seconds() / 3600
        if hours >= 1:
            print(f"  Duration:             {hours:.1f} hours")
        else:
            print(f"  Duration:             {duration.total_seconds():.0f} seconds")

    # --- Domains ---
    if stats["domains_allowed"]:
        print()
        print(f"{bold}Domains Contacted{nc}")
        for domain, count in stats["domains_allowed"].most_common():
            print(f"  {count:>5} requests → {domain}")

    if stats["domains_blocked"]:
        print()
        print(f"{bold}Blocked Domains{nc}")
        for domain, count in stats["domains_blocked"].most_common():
            print(f"  {colors['MEDIUM']}{count:>5} blocked  → {domain}{nc}")

    # --- Findings ---
    if findings:
        print()
        print(f"{bold}Security Findings ({len(findings)}){nc}")
        print()

        # Sort by severity: CRITICAL first
        severity_order = {"CRITICAL": 0, "HIGH": 1, "MEDIUM": 2, "LOW": 3}
        sorted_findings = sorted(findings, key=lambda f: severity_order.get(f["severity"], 99))

        for f in sorted_findings:
            sev = f["severity"]
            color = colors.get(sev, "")
            ts = f.get("timestamp", "")
            ts_display = f"  [{ts[:19]}]" if ts else ""
            print(f"  {color}[{sev}]{nc}{ts_display} {f['message']}")
    else:
        print()
        print(f"  \033[0;32mNo security findings — all activity within expected bounds.{nc}")

    print()
    print("=" * 40)
    print()


def print_json(findings, stats, malformed):
    """Print machine-readable JSON output."""
    output = {
        "summary": {
            "total_entries": stats["total_entries"],
            "malformed_lines": malformed,
            "by_action": dict(stats["by_action"]),
            "domains_allowed": dict(stats["domains_allowed"]),
            "domains_blocked": dict(stats["domains_blocked"]),
            "total_request_bytes": stats["total_request_bytes"],
            "total_response_bytes": stats["total_response_bytes"],
        },
        "findings": findings,
        "finding_count": len(findings),
        "has_critical": any(f["severity"] == "CRITICAL" for f in findings),
    }
    if stats["timestamps"]:
        sorted_ts = sorted(stats["timestamps"])
        output["summary"]["time_start"] = sorted_ts[0].isoformat()
        output["summary"]["time_end"] = sorted_ts[-1].isoformat()

    print(json.dumps(output, indent=2, default=str))


def main():
    args = parse_args()

    # --- Load log data ---
    if args["file"]:
        if not os.path.isfile(args["file"]):
            print(f"Error: file not found: {args['file']}", file=sys.stderr)
            sys.exit(1)
        with open(args["file"]) as f:
            raw = f.read()
    else:
        raw = find_log_source()
        if raw is None:
            print("Error: no proxy logs found.", file=sys.stderr)
            print("  Is the vault-proxy container running?", file=sys.stderr)
            print("  Try: python3 monitoring/network-log-parser.py --file <path-to-requests.jsonl>", file=sys.stderr)
            sys.exit(1)

    if not raw.strip():
        if args["json_output"]:
            print_json([], {"total_entries": 0, "by_action": Counter(),
                           "domains_allowed": Counter(), "domains_blocked": Counter(),
                           "total_request_bytes": 0, "total_response_bytes": 0,
                           "timestamps": []}, 0)
        else:
            print("Proxy log is empty — no network activity to analyze.")
        sys.exit(0)

    # --- Parse ---
    entries, malformed = parse_log_lines(raw)

    # --- Analyze ---
    config = {
        "threshold": args["threshold"],
        "window": args["window"],
        "spike": args["spike"],
    }
    findings, stats = analyze(entries, config)

    # --- Output ---
    if args["json_output"]:
        print_json(findings, stats, malformed)
    else:
        print_report(findings, stats, malformed)

    # Exit code: 2 if critical findings, 1 if any findings, 0 if clean
    if any(f["severity"] == "CRITICAL" for f in findings):
        sys.exit(2)
    if findings:
        sys.exit(1)
    sys.exit(0)


if __name__ == "__main__":
    main()
