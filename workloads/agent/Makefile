.PHONY: help setup start stop kill nuclear verify test network-report session-report log-rotate tools-status tools-dry-run hard-shell split-shell soft-shell install-skill list-skills

SHELL := /bin/bash

# Runtime detection: prefer podman, fall back to docker
RUNTIME := $(shell command -v podman >/dev/null 2>&1 && echo podman || echo docker)
COMPOSE := $(RUNTIME) compose

help: ## Show available commands
	@echo ""
	@echo "  opencli-container"
	@echo "  =============="
	@echo ""
	@echo "  runtime: $(RUNTIME)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'
	@echo ""

setup: ## Build and configure the hardened container environment
	@bash scripts/setup.sh

start: ## Start the vault container and proxy sidecar
	@$(COMPOSE) up -d

stop: ## Gracefully stop the vault container
	@$(COMPOSE) stop

kill: ## Force stop the vault container immediately
	@bash scripts/kill.sh --hard

nuclear: ## Remove all containers, images, and volumes — full reset
	@bash scripts/kill.sh --nuclear

verify: ## Run 24-point security verification checks
	@bash scripts/verify.sh

test: ## Run all test scripts in tests/
	@bash scripts/run-tests.sh

tools-status: ## Show which tools are currently enabled/disabled
	@bash scripts/tool-control.sh --status

tools-dry-run: ## Preview a tool config (PRESET=hard|split)
	@bash scripts/tool-control.sh --preset $(PRESET) --dry-run

hard-shell: ## Switch to Hard Shell preset (maximum lockdown)
	@bash scripts/tool-control.sh --preset hard --apply

split-shell: ## Switch to Split Shell preset (workspace I/O with approval)
	@bash scripts/tool-control.sh --preset split --apply

soft-shell: ## Switch to Soft Shell preset (safari — broad autonomy, absolute moat)
	@bash scripts/tool-control.sh --preset soft --apply

install-skill: ## Install a vetted skill (SKILL=<dir> [CLEARANCE=<report.json>])
	@bash scripts/install-skill.sh $(SKILL) $(if $(CLEARANCE),--clearance $(CLEARANCE))

list-skills: ## List installed skills in the workspace
	@bash scripts/install-skill.sh --list

network-report: ## Analyze proxy logs for security anomalies
	@python3 monitoring/network-log-parser.py

session-report: ## Generate post-session summary of agent activity
	@python3 monitoring/session-report.py

log-rotate: ## Rotate proxy logs if over 10MB, check session transcript size
	@bash scripts/log-rotate.sh
