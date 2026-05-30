.PHONY: help setup new create create-noninteractive lint lint-one lint-all scan scan-one scan-json scan-sarif scan-summary scan-strict scan-all test test-one test-tools publish stats stats-trend stats-rank check check-all self-test verify verify-skill verify-all verify-report trust-all certify certify-all export download cdr cdr-download explore report clean

SHELL := /bin/bash
SKILLS_DIR := skills
TOOLS_DIR := tools
TESTS_DIR := tests

setup: ## Set up the workbench (verify tools and directories)
	@echo "[*] Setting up OpenSkill Forge workbench..."
	@mkdir -p $(SKILLS_DIR) $(TESTS_DIR)
	@bash $(TOOLS_DIR)/workbench-verify.sh
	@echo "[+] Setup complete — edit config/.env to configure"

help: ## Show available commands
	@echo ""
	@echo "  openskill-forge workbench"
	@echo "  ====================="
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'
	@echo ""

new: ## Scaffold new skill from template (SKILL=name TYPE=cli-tool|workflow|language-ref)
	@bash $(TOOLS_DIR)/skill-new.sh "$(SKILL)" "$(or $(TYPE),cli-tool)"

create: ## AI-assisted skill creation wizard (interactive)
	@bash $(TOOLS_DIR)/skill-create.sh

create-noninteractive: ## AI skill creation (non-interactive, for GUI)
	@bash $(TOOLS_DIR)/skill-create.sh --name "$(NAME)" --type "$(TYPE)" --description "$(DESC)" $(if $(COMMANDS),--commands "$(COMMANDS)") $(if $(TIPS),--tips "$(TIPS)")

lint: ## Lint all skills (frontmatter + structure + content)
	@bash $(TOOLS_DIR)/skill-lint.sh $(SKILLS_DIR)

lint-one: ## Lint single skill (SKILL=name)
	@bash $(TOOLS_DIR)/skill-lint.sh $(SKILLS_DIR)/$(SKILL)

scan: ## Offline security scan all skills
	@bash $(TOOLS_DIR)/skill-scan.sh $(SKILLS_DIR)

scan-one: ## Scan single skill (SKILL=name)
	@bash $(TOOLS_DIR)/skill-scan.sh $(SKILLS_DIR)/$(SKILL)

scan-json: ## Scan all skills, JSON output
	@bash $(TOOLS_DIR)/skill-scan.sh --json $(SKILLS_DIR)

scan-sarif: ## Scan all skills, SARIF 2.1.0 output
	@bash $(TOOLS_DIR)/skill-scan.sh --sarif $(SKILLS_DIR)

scan-summary: ## Scan all skills, one-line summary output
	@bash $(TOOLS_DIR)/skill-scan.sh --summary $(SKILLS_DIR)

scan-strict: ## Scan with --strict (HIGH findings block)
	@bash $(TOOLS_DIR)/skill-scan.sh --strict $(SKILLS_DIR)

lint-all: lint  ## Alias: lint all skills

scan-all: scan  ## Alias: scan all skills

self-test: ## Run scanner self-test (known-bad/clean/allowlisted)
	@bash $(TESTS_DIR)/scanner-self-test/run.sh

verify-skill: ## Zero-trust verify single skill (SKILL=name)
	@bash $(TOOLS_DIR)/skill-verify.sh $(SKILLS_DIR)/$(SKILL)

verify-all: ## Zero-trust verify all skills
	@bash $(TOOLS_DIR)/skill-verify.sh $(SKILLS_DIR)

verify-report: ## Verify with per-line report (SKILL=name)
	@bash $(TOOLS_DIR)/skill-verify.sh --report $(SKILLS_DIR)/$(SKILL)

trust-all: ## Generate .trust files for all verified skills
	@bash $(TOOLS_DIR)/skill-verify.sh --trust $(SKILLS_DIR)

certify: ## Generate security certificate (SKILL=name)
	@bash $(TOOLS_DIR)/skill-certify.sh "$(SKILL)"

certify-all: ## Generate certificates for all skills
	@for dir in $(SKILLS_DIR)/*/; do \
		skill=$$(basename "$$dir"); \
		bash $(TOOLS_DIR)/skill-certify.sh "$$skill" || exit 1; \
	done

export: ## Certify + package for vault transfer (SKILL=name)
	@bash $(TOOLS_DIR)/skill-export.sh "$(SKILL)"

download: ## Download skill from ClawHub to quarantine (SKILL=name)
	@bash $(TOOLS_DIR)/skill-download.sh "$(SKILL)"

cdr: ## CDR a local skill file (FILE=path/to/SKILL.md)
	@bash $(TOOLS_DIR)/skill-cdr.sh "$(FILE)"

cdr-download: ## Download from ClawHub + CDR (SKILL=name)
	@bash $(TOOLS_DIR)/skill-cdr.sh --download "$(SKILL)"

test: ## Run skill behavioral tests
	@bash $(TOOLS_DIR)/skill-test.sh

test-one: ## Test single skill (SKILL=name)
	@bash $(TOOLS_DIR)/skill-test.sh $(SKILL)

test-tools: ## Run tool behavioral tests
	@bash $(TESTS_DIR)/_framework/tool-runner.sh

publish: ## Lint + scan + test + publish (SKILL=name VERSION=x.y.z)
	@bash $(TOOLS_DIR)/skill-publish.sh "$(SKILL)" "$(VERSION)"

stats: ## Check adoption metrics from ClawHub API
	@bash $(TOOLS_DIR)/skill-stats.sh

stats-trend: ## Stats with growth deltas vs previous snapshots
	@bash $(TOOLS_DIR)/skill-stats.sh --trend

stats-rank: ## Our skills ranked against registry top 50
	@bash $(TOOLS_DIR)/skill-stats.sh --rank

explore: ## Browse registry top skills (QUERY=term SORT=downloads|trending|installs LIMIT=n)
	@bash $(TOOLS_DIR)/registry-explore.sh $(if $(QUERY),"$(QUERY)") $(if $(SORT),--sort=$(SORT)) $(if $(LIMIT),--limit=$(LIMIT))

verify: ## 12-point workbench health verification
	@bash $(TOOLS_DIR)/workbench-verify.sh

check: ## Full pipeline: lint + scan + test
	@echo ""
	@echo "=== Running full pipeline ==="
	@echo ""
	@bash $(TOOLS_DIR)/skill-lint.sh $(SKILLS_DIR)
	@echo ""
	@bash $(TOOLS_DIR)/skill-scan.sh $(SKILLS_DIR)
	@echo ""
	@bash $(TOOLS_DIR)/skill-test.sh
	@echo ""
	@echo "=== Pipeline complete ==="

check-all: ## Full pipeline + self-test + tool tests
	@echo ""
	@echo "=== Running full pipeline + all tests ==="
	@echo ""
	@bash $(TOOLS_DIR)/skill-lint.sh $(SKILLS_DIR)
	@echo ""
	@bash $(TOOLS_DIR)/skill-scan.sh $(SKILLS_DIR)
	@echo ""
	@bash $(TOOLS_DIR)/skill-test.sh
	@echo ""
	@bash $(TESTS_DIR)/scanner-self-test/run.sh
	@echo ""
	@bash $(TESTS_DIR)/_framework/tool-runner.sh
	@echo ""
	@echo "=== All checks complete ==="

report: ## Pipeline value report — what the workbench catches
	@bash $(TOOLS_DIR)/pipeline-report.sh

clean: ## Remove generated cache files
	@rm -rf .workbench-cache
	@echo "Cache cleaned."
