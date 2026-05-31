#!/usr/bin/env bash
# Skill test runner wrapper
# Usage: skill-test.sh [skill_name]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

exec bash "$REPO_ROOT/tests/_framework/runner.sh" "${1:-}"
