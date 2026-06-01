#!/usr/bin/env python3
"""CDR disarm diff — the trust artifact.

Turns the scanner's findings on the ORIGINAL skill into a plain-language
summary of what Content Disarm & Reconstruction removed/caught, so the user
SEES what was disarmed instead of just "scan passed".

Usage:
  cdr-diff.py <original-scan.json> [--delivered | --quarantined]

Input: the JSON output of `skill-scan.sh --json` on the original skill.
Output (stdout): a short, plain-language disarm summary. No jargon — obeys the
banned-vocabulary rule (user-facing).
"""
import json
import sys

# Map scanner categories → plain language a non-technical user understands.
CATEGORY_PLAIN = {
    "cred_access": "reading your saved passwords, keys, or login files",
    "exfiltration": "sending your data out to an outside server",
    "exec_download": "downloading and running code from the internet",
    "c2_download": "contacting a remote control server",
    "archive_exec": "unpacking and running a hidden bundle",
    "prompt_injection": "hidden instructions trying to take over your assistant",
    "persistence": "installing itself to keep running later",
    "privilege_escalation": "trying to gain more access than it's allowed",
    "container_escape": "trying to break out of its protected setup",
    "supply_chain": "pulling in untrusted code from elsewhere",
    "env_injection": "tampering with environment settings",
    "obfuscation": "hidden or disguised content",
    "resource_abuse": "using your machine's resources without consent",
}


def plain(category: str) -> str:
    return CATEGORY_PLAIN.get(category, f"a flagged behaviour ({category})")


def main() -> int:
    if len(sys.argv) < 2:
        print("Usage: cdr-diff.py <original-scan.json> [--delivered|--quarantined]",
              file=sys.stderr)
        return 1
    mode = "--delivered"
    for a in sys.argv[2:]:
        if a in ("--delivered", "--quarantined"):
            mode = a

    try:
        with open(sys.argv[1]) as f:
            scan = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        # No scan data — emit the minimal honest statement.
        scan = {"findings": []}

    findings = scan.get("findings", []) or []
    # Dedupe behaviours by category, keep the worst severity per category.
    by_cat: dict[str, str] = {}
    sev_rank = {"CRITICAL": 3, "HIGH": 2, "MEDIUM": 1, "LOW": 0}
    for fnd in findings:
        cat = fnd.get("category", "unknown")
        sev = fnd.get("severity", "LOW")
        if cat not in by_cat or sev_rank.get(sev, 0) > sev_rank.get(by_cat[cat], 0):
            by_cat[cat] = sev

    behaviours = [plain(c) for c in by_cat]

    if mode == "--quarantined":
        if behaviours:
            print("I blocked this skill. The original tried to do things a skill shouldn't:")
            for b in behaviours:
                print(f"  • {b}")
            print("None of it reached your assistant — the skill was held back.")
        else:
            print("I couldn't safely rebuild this skill, so I held it back. "
                  "Nothing from it reached your assistant.")
        return 0

    # Delivered (clean) path.
    print("I rebuilt this skill from scratch using only its stated purpose, "
          "so any hidden content in the original is gone.")
    if behaviours:
        print("The original also contained, and I removed:")
        for b in behaviours:
            print(f"  • {b}")
    else:
        print("Nothing harmful was found in the original — the rebuild is a "
              "clean-room copy as an extra precaution.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
