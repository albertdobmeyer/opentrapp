#!/usr/bin/env python3
"""Convert scanner JSON output to SARIF 2.1.0 format."""
import json
import sys

SEVERITY_MAP = {"CRITICAL": "error", "HIGH": "warning", "MEDIUM": "note"}


def to_sarif(data: dict) -> dict:
    rules = {}
    results = []

    for f in data.get("findings", []):
        rule_id = f"{f['category']}/{f['description']}"
        if rule_id not in rules:
            rules[rule_id] = {
                "id": rule_id,
                "shortDescription": {"text": f["description"]},
                "defaultConfiguration": {
                    "level": SEVERITY_MAP.get(f["severity"], "note")
                },
                "properties": {},
            }
            if f.get("mitreId"):
                rules[rule_id]["properties"]["mitre-attack"] = f["mitreId"]

        results.append(
            {
                "ruleId": rule_id,
                "level": SEVERITY_MAP.get(f["severity"], "note"),
                "message": {"text": f["description"]},
                "locations": [
                    {
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": f"skills/{f['skill']}/SKILL.md"
                            },
                            "region": {"startLine": f["line"]},
                        }
                    }
                ],
            }
        )

    return {
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "openagent-skills-skill-scan",
                        "version": "2.0.0",
                        "informationUri": "https://github.com/albertdobmeyer/openagent-skills",
                        "rules": list(rules.values()),
                    }
                },
                "results": results,
            }
        ],
    }


if __name__ == "__main__":
    data = json.load(sys.stdin)
    json.dump(to_sarif(data), sys.stdout, indent=2)
    print()
