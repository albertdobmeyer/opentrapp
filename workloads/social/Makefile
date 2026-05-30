.PHONY: help scan scan-agent census census-trend checklist check-api export-patterns setup test verify observer researcher participant level-status

TOOLS_DIR := tools
TESTS_DIR := tests

# Default target
help: ## Show available commands
	@grep -E '^[a-z_-]+:.*## ' $(MAKEFILE_LIST) | \
		awk -F ':.*## ' '{printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

# ── Operations ──────────────────────────────────────────
scan: ## Scan recent feed (COUNT=n, default 50)
	@bash $(TOOLS_DIR)/feed-scanner.sh --recent $(or $(COUNT),50)

scan-agent: ## Scan specific agent (AGENT=handle)
	@bash $(TOOLS_DIR)/feed-scanner.sh --agent $(AGENT)

census: ## Pull current platform stats
	@bash $(TOOLS_DIR)/agent-census.sh

census-trend: ## Show trend data from saved snapshots
	@bash $(TOOLS_DIR)/agent-census.sh --trend

checklist: ## Run identity pre-flight checklist
	@bash $(TOOLS_DIR)/identity-checklist.sh

check-api: ## Check Moltbook API liveness
	@echo "Checking Moltbook API..."
	@curl -sf --max-time 10 https://api.moltbook.com/posts?limit=1 >/dev/null 2>&1 \
		&& echo "  API: UP (api.moltbook.com responds)" \
		|| echo "  API: DOWN or unreachable (api.moltbook.com)"

# ── Engagement Levels ──────────────────────────────────
observer: ## Switch to Level 1 (read-only, no API key)
	@bash scripts/engagement-control.sh --level observer --apply

researcher: ## Switch to Level 2 (registered, controlled interaction)
	@bash scripts/engagement-control.sh --level researcher --apply

participant: ## Switch to Level 3 (full interaction with guardrails)
	@bash scripts/engagement-control.sh --level participant --apply

level-status: ## Show current engagement level and config
	@bash scripts/engagement-control.sh --status

# ── Export ──────────────────────────────────────────────
export-patterns: ## Export injection patterns for vault-proxy consumption
	@mkdir -p data
	@python3 scripts/export-patterns.py

# ── Lifecycle ───────────────────────────────────────────
setup: ## Copy .env.example → .env, create data/
	@cp -n config/.env.example config/.env 2>/dev/null && \
		echo "Created config/.env from template" || \
		echo "config/.env already exists"
	@mkdir -p data
	@echo "Setup complete — edit config/.env to configure"

# ── Testing ─────────────────────────────────────────────
test: ## Run tool test suite
	@bash $(TESTS_DIR)/_framework/tool-runner.sh

# ── Verification ────────────────────────────────────────
verify: ## Verify workbench health (config, tools, patterns, engagement level)
	@bash scripts/verify.sh
