#!/usr/bin/env python3
"""Export pioneer injection patterns for vault-proxy consumption.

Reads config/injection-patterns.yml via raw text parsing (the source file
uses YAML double-quoted strings with regex backslashes that conflict with
YAML escape rules — the bash scanner parses the same way).

Validates all regexes compile with Python re.compile(), exports minimal
YAML to data/patterns-export.yml.

Usage: python3 scripts/export-patterns.py
"""
import hashlib
import os
import re
import sys

import yaml

try:
    import re._parser as sre_parse
except ImportError:
    import sre_parse  # Python < 3.11 fallback

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
PATTERNS_FILE = os.path.join(PROJECT_ROOT, "config", "injection-patterns.yml")
EXPORT_FILE = os.path.join(PROJECT_ROOT, "data", "patterns-export.yml")


def check_redos(regex_str):
    """Check a regex string for ReDoS vulnerability via AST analysis.

    Returns a description string if vulnerable, None if safe.
    Detects: nested quantifiers, unbounded repetition on alternation groups.
    """
    try:
        parsed = sre_parse.parse(regex_str)
    except Exception:
        return None  # Can't parse = can't check; re.compile() catches syntax errors

    def walk(node_list, inside_quantifier=False):
        """Walk the AST looking for nested quantifiers."""
        for op, av in node_list:
            if op in (sre_parse.MAX_REPEAT, sre_parse.MIN_REPEAT):
                _min, _max, subpattern = av
                # A variable-length quantifier inside another quantifier = nested
                # Fixed-count ({n} where min==max) is safe — no backtracking ambiguity
                if inside_quantifier and _min != _max:
                    return "nested quantifiers detected"
                # Check for unbounded quantifier on alternation
                if _max == sre_parse.MAXREPEAT:
                    for sub_op, sub_av in subpattern:
                        if sub_op == sre_parse.SUBPATTERN and sub_av[3]:
                            for inner_op, inner_av in sub_av[3]:
                                if inner_op == sre_parse.BRANCH:
                                    return "unbounded repetition on alternation"
                        if sub_op == sre_parse.BRANCH:
                            return "unbounded repetition on alternation"
                # Recurse into the quantifier body
                result = walk(subpattern, inside_quantifier=True)
                if result:
                    return result
            elif op == sre_parse.SUBPATTERN:
                if av[3]:
                    result = walk(av[3], inside_quantifier)
                    if result:
                        return result
            elif op == sre_parse.BRANCH:
                for branch in av[1]:
                    result = walk(branch, inside_quantifier)
                    if result:
                        return result
        return None

    return walk(parsed)


# Complexity thresholds — calibrated so all 25 current patterns are SAFE
# Max current score: 23976 (enc-004). WARN at 30000 gives 25% headroom.
WARN_THRESHOLD = 30000
REJECT_THRESHOLD = 50000


def complexity_score(regex_str):
    """Score a regex for backtracking complexity.

    score = alternation_branches * max_quantifier_bound * nesting_depth

    Low scores are safe. High scores indicate patterns that could be slow
    on pathological inputs.

    Note: sre_parse compiles single-character alternations like (a|b|c)
    into character class IN nodes, not BRANCH nodes. These score as
    branches=1 because they match deterministically — this is correct.
    """
    try:
        parsed = sre_parse.parse(regex_str)
    except Exception:
        return 0

    def analyze(node_list, depth=1):
        """Walk AST and compute complexity metrics."""
        branches = 1
        max_bound = 1
        max_depth = depth

        for op, av in node_list:
            if op == sre_parse.BRANCH:
                branch_count = len(av[1])
                branches *= branch_count
                for branch in av[1]:
                    b, m, d = analyze(branch, depth)
                    branches *= b
                    max_bound = max(max_bound, m)
                    max_depth = max(max_depth, d)
            elif op in (sre_parse.MAX_REPEAT, sre_parse.MIN_REPEAT):
                _min, _max, subpattern = av
                bound = _max if _max != sre_parse.MAXREPEAT else 999
                max_bound = max(max_bound, bound)
                b, m, d = analyze(subpattern, depth + 1)
                branches *= b
                max_bound = max(max_bound, m)
                max_depth = max(max_depth, d)
            elif op == sre_parse.SUBPATTERN:
                if av[3]:
                    b, m, d = analyze(av[3], depth)
                    branches *= b
                    max_bound = max(max_bound, m)
                    max_depth = max(max_depth, d)

        return branches, max_bound, max_depth

    branches, max_bound, max_depth = analyze(parsed)
    return branches * max_bound * max_depth


def parse_patterns(filepath):
    """Parse injection-patterns.yml via raw text extraction.

    Mirrors the bash scanner's approach: read line-by-line, strip quotes,
    extract fields. This avoids YAML escape conflicts in regex strings.
    """
    patterns = []
    current = {}

    with open(filepath) as f:
        for line in f:
            stripped = line.strip()

            if stripped.startswith("- id:"):
                if current.get("id") and current.get("severity") and current.get("regex"):
                    patterns.append(current)
                current = {"id": stripped.split(":", 1)[1].strip()}

            elif stripped.startswith("severity:"):
                current["severity"] = stripped.split(":", 1)[1].strip()

            elif stripped.startswith("regex:"):
                raw = stripped.split(":", 1)[1].strip()
                # Strip surrounding quotes (same as bash scanner)
                if raw.startswith('"') and raw.endswith('"'):
                    raw = raw[1:-1]
                elif raw.startswith("'") and raw.endswith("'"):
                    raw = raw[1:-1]
                current["regex"] = raw

    # Don't forget the last pattern
    if current.get("id") and current.get("severity") and current.get("regex"):
        patterns.append(current)

    return patterns


def main():
    patterns = parse_patterns(PATTERNS_FILE)

    if not patterns:
        print("ERROR: No patterns found in", PATTERNS_FILE, file=sys.stderr)
        sys.exit(1)

    # Validate and extract
    exported = []
    failures = 0

    for p in patterns:
        pid = p["id"]
        severity = p["severity"]
        regex = p["regex"]

        try:
            re.compile(regex)
        except re.error as e:
            print(f"  FAIL: {pid} — {e}", file=sys.stderr)
            failures += 1
            continue

        # ReDoS static analysis
        redos_issue = check_redos(regex)
        if redos_issue:
            print(f"  REJECT: {pid} — {redos_issue}", file=sys.stderr)
            failures += 1
            continue

        # Complexity scoring
        score = complexity_score(regex)
        if score >= REJECT_THRESHOLD:
            print(f"  REJECT: {pid} — complexity score {score} exceeds {REJECT_THRESHOLD}", file=sys.stderr)
            failures += 1
            continue
        if score >= WARN_THRESHOLD:
            print(f"  WARN: {pid} — complexity score {score} (threshold {WARN_THRESHOLD})", file=sys.stderr)

        exported.append({"id": pid, "severity": severity, "regex": regex})

    # Ensure output directory exists
    os.makedirs(os.path.dirname(EXPORT_FILE), exist_ok=True)

    # Compute integrity hash over regex content (sorted by ID)
    hash_input = "\n".join(
        p["regex"] for p in sorted(exported, key=lambda x: x["id"])
    )
    integrity = hashlib.sha256(hash_input.encode()).hexdigest()

    # Write export file with header comment
    header = (
        "# Generated by: make export-patterns\n"
        "# Source: config/injection-patterns.yml\n"
        f"# Count: {len(exported)} patterns\n"
        f"# Integrity: sha256:{integrity}\n"
    )
    body = yaml.dump(
        {"patterns": exported},
        default_flow_style=False,
        sort_keys=False,
        allow_unicode=True,
    )

    with open(EXPORT_FILE, "w") as f:
        f.write(header)
        f.write(body)

    # Report
    print(f"Exported {len(exported)} patterns to {EXPORT_FILE}")
    if failures:
        print(f"WARNING: {failures} pattern(s) failed to compile", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
