# OpenTrApp — common operator commands.
#
# Conventions:
#   - Targets group by area (test gates, dogfood, audits, perimeter ops).
#   - Each target prints what it's doing before it runs.
#   - Long-running targets print a budget hint where applicable.
#
# Tested on the maintainer's dev machine (Ubuntu 24.04, podman, ~7 GB RAM).
# On Windows: run from a WSL2 terminal (bash + GNU make are available there;
# podman uses the same WSL2 machine so container operations work transparently).
# CI runs the same commands directly; this Makefile is for local convenience.

.PHONY: help \
        test-rust test-frontend test-tsc test-e2e orchestrator-check verify-all \
        dogfood-tier-a dogfood-tier-b dogfood-tier-c dogfood-tier-d dogfood-full \
        dogfood-fresh-sessions dogfood-restore-sessions \
        audit-rust audit-npm audit-deny audit-all \
        perimeter-up perimeter-down perimeter-status profile-memory \
        sync-core-embedded daemon daemon-run

help:
	@echo "OpenTrApp common targets:"
	@echo ""
	@echo "  Test gates (also run by CI on every PR):"
	@echo "    test-rust          cargo test --lib (~30s warm)"
	@echo "    test-frontend      vitest (~15s)"
	@echo "    test-tsc           TypeScript strict-mode check (~10s)"
	@echo "    test-e2e           Playwright tests (~60s)"
	@echo "    orchestrator-check 42-check manifest validation (~5s)"
	@echo "    verify-all         all five gates in sequence"
	@echo ""
	@echo "  Dogfood test (Karen end-to-end run):"
	@echo "    dogfood-tier-a     happy-path scenarios (~35 min, ~\$$0.30)"
	@echo "    dogfood-tier-b     adversarial scenarios (~10 min, ~\$$0.10)"
	@echo "    dogfood-tier-c     state-coverage scenarios (operator-driven)"
	@echo "    dogfood-tier-d     termination-path scenarios (operator-driven)"
	@echo "    dogfood-full       all 27 scenarios in arc order (~70 min)"
	@echo "    dogfood-fresh-sessions    move existing bot sessions aside"
	@echo "                              (use before re-testing prompt changes)"
	@echo "    dogfood-restore-sessions  restore the .bak session files"
	@echo ""
	@echo "  Supply-chain audits:"
	@echo "    audit-rust         cargo audit (vulnerabilities)"
	@echo "    audit-npm          npm audit (vulnerabilities)"
	@echo "    audit-deny         cargo deny check (advisories+licenses+bans+sources)"
	@echo "    audit-all          all three in sequence"
	@echo ""
	@echo "  Perimeter operations:"
	@echo "    perimeter-up       podman compose up -d"
	@echo "    perimeter-down     podman compose down"
	@echo "    perimeter-status   four-container health snapshot"
	@echo "    profile-memory     per-container RSS + host RAM/swap + image sizes"
	@echo ""
	@echo "  Headless daemon (GUI-free, lean ~30-60 MB; ADR-0019):"
	@echo "    daemon             build only the perimeter daemon (no GUI / no WebKit)"
	@echo "    daemon-run         build + run the headless daemon (owns the perimeter)"

# ── Test gates ───────────────────────────────────────────────────────────────

test-rust:
	@echo "→ cargo test --lib"
	cd app/src-tauri && cargo test --lib

test-frontend:
	@echo "→ npm test (vitest)"
	cd app && npm test -- --run

test-tsc:
	@echo "→ npx tsc --noEmit"
	cd app && npx tsc --noEmit

test-e2e:
	@echo "→ npx playwright test"
	cd app && npx playwright test

orchestrator-check:
	@echo "→ tests/orchestrator-check.sh"
	bash tests/orchestrator-check.sh

verify-all: orchestrator-check test-rust test-frontend test-tsc test-e2e
	@echo "✓ all five gates passed"

# ── Dogfood ──────────────────────────────────────────────────────────────────
# The dogfood harness lives at tests/dogfood/ but its conftest fixtures (bot,
# proxy_log, budget) come from tests/e2e-telegram/conftest.py. Pytest must run
# from the e2e-telegram dir for fixture discovery; we cd there + invoke pytest
# against the dogfood module path.
#
# The 'fresh-sessions' / 'restore-sessions' targets handle the session-cache
# caveat documented in tests/dogfood/CHECKLIST.md §0a — required when re-testing
# after a system-prompt change so the bot doesn't self-mimic stale jargon.

DOGFOOD_PYTEST = cd tests/e2e-telegram && . .venv/bin/activate && pytest ../dogfood/test_full_arc.py

dogfood-tier-a:
	@echo "→ Tier A: 5 happy-path scenarios (~35 min, ~\$$0.30)"
	$(DOGFOOD_PYTEST) -m dogfood_tier_a -v

dogfood-tier-b:
	@echo "→ Tier B: 8 adversarial scenarios (~10 min, ~\$$0.10)"
	$(DOGFOOD_PYTEST) -m dogfood_tier_b -v

dogfood-tier-c:
	@echo "→ Tier C: 7 AssistantStatus-state scenarios (operator-driven)"
	$(DOGFOOD_PYTEST) -m dogfood_tier_c -v

dogfood-tier-d:
	@echo "→ Tier D: 7 termination-path scenarios (operator-driven)"
	$(DOGFOOD_PYTEST) -m dogfood_tier_d -v

dogfood-full:
	@echo "→ Full arc: 27 scenarios (~70 min, ~\$$0.40)"
	$(DOGFOOD_PYTEST) -m dogfood_full -v

dogfood-fresh-sessions:
	@echo "→ Moving vault-agent bot sessions aside (renamed, not deleted)"
	@podman exec vault-agent sh -c '\
		cd /home/vault/.openclaw/agents/main/sessions/ && \
		for f in sessions.json *.jsonl; do \
			[ -f "$$f" ] || continue; \
			mv "$$f" "$${f}.dogfood-fix-$$(date -u +%Y-%m-%d).bak"; \
		done && \
		ls -la \
	'
	@echo "→ Restarting vault-agent so it spawns fresh sessions"
	@podman restart vault-agent
	@echo "✓ Fresh sessions ready in ~25s. Wait before running dogfood."

dogfood-restore-sessions:
	@echo "→ Restoring sessions/*.dogfood-fix-*.bak files to original names"
	@podman exec vault-agent sh -c '\
		cd /home/vault/.openclaw/agents/main/sessions/ && \
		for f in *.dogfood-fix-*.bak; do \
			[ -f "$$f" ] || continue; \
			mv "$$f" "$${f%.dogfood-fix-*}"; \
		done && \
		ls -la \
	'
	@podman restart vault-agent
	@echo "✓ Sessions restored. Bot may pull cached vocabulary until they age out."

# ── Supply-chain audits ─────────────────────────────────────────────────────

audit-rust:
	@echo "→ cargo audit (Rust deps)"
	cd app/src-tauri && cargo audit

audit-npm:
	@echo "→ npm audit (frontend deps)"
	cd app && npm audit

audit-deny:
	@echo "→ cargo deny check (advisories + licenses + bans + sources)"
	cd app/src-tauri && cargo deny --all-features check

audit-all: audit-rust audit-npm audit-deny
	@echo "✓ all three audits passed"

# ── Perimeter operations ────────────────────────────────────────────────────
# Prefer the native podman-compose when present. Bare `podman compose` (no
# hyphen) delegates to the docker-compose CLI plugin where that plugin is
# installed, which is INCOMPATIBLE with this compose file on podman: it inlines
# `security_opt: seccomp=<file>` as JSON (podman's API then rejects it as a path
# → "file name too long") and mismatches compose network labels. Native
# podman-compose passes `--security-opt seccomp=<path>` straight to `podman run`.
# Falls back to `podman compose` only where podman-compose is absent.
# Verified on the 7.2 GB Linux laptop (2026-06-16): full from-scratch build +
# the live five-container perimeter ran with ~3.6 GB free, no swap-storm, and
# T0 boundary-selftest = exit 0 cold AND resumed.
COMPOSE := $(shell command -v podman-compose >/dev/null 2>&1 && echo podman-compose || echo podman compose)

perimeter-up:
	@echo "→ $(COMPOSE) up -d  (secrets via .env passthrough — never on the argv/echo, #75)"
	@set -a; [ -f .env ] && . ./.env; set +a; $(COMPOSE) up -d

perimeter-down:
	@echo "→ $(COMPOSE) down"
	$(COMPOSE) down

perimeter-status:
	@echo "── perimeter health snapshot ──"
	@podman ps --filter "name=vault-" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" 2>/dev/null \
		|| echo "(no containers; run 'make perimeter-up')"

profile-memory:
	@echo "→ bash tests/memory-profile.sh (per-container RSS; bring the perimeter up first)"
	@bash tests/memory-profile.sh

# Derive the live container names from the compose-service label so this works
# whatever the runner named them (podman-compose → opentrapp_vault-*_1,
# docker-compose → opentrapp-vault-*-1, or a bare vault-*). An empty result
# (container absent) falls back to the script's default and is reported as
# CANNOT ASSESS. Pass cold-start flags via ARGS:
#   make boundary-selftest ARGS=--record-baseline
boundary-selftest:
	@echo "→ bash tests/boundary-selftest.sh $(ARGS) (boundary holds? bring the perimeter up first)"
	@OPENTRAPP_AGENT_CTR=$$(podman ps --filter label=com.docker.compose.service=vault-agent --format '{{.Names}}' | head -1) \
	 OPENTRAPP_PROXY_CTR=$$(podman ps --filter label=com.docker.compose.service=vault-proxy --format '{{.Names}}' | head -1) \
	 OPENTRAPP_EGRESS_CTR=$$(podman ps --filter label=com.docker.compose.service=vault-egress --format '{{.Names}}' | head -1) \
	 bash tests/boundary-selftest.sh $(ARGS)

# Re-sync opentrapp-core's vendored copies of perimeter.yml + boundary-selftest.sh
# from their canonical sources. The canonical files (resources/, tests/) are the
# source of truth; core keeps in-crate copies so it is crates.io-publishable
# (ADR-0023). orchestrator-check.sh fails if they drift — run this after editing
# a canonical file.
sync-core-embedded:
	@cp app/src-tauri/resources/perimeter.yml app/src-tauri/crates/core/src/embedded/perimeter.yml
	@cp tests/boundary-selftest.sh app/src-tauri/crates/core/src/embedded/boundary-selftest.sh
	@echo "→ synced opentrapp-core/src/embedded/ from canonical resources/ + tests/"

proxy-soak:
	@echo "→ bash tests/proxy-memory-soak.sh $(ARGS) (vault-proxy RSS over load×time; perimeter up first)"
	@OPENTRAPP_PROXY_CTR=$$(podman ps --filter label=com.docker.compose.service=vault-proxy --format '{{.Names}}' | head -1) \
	 OPENTRAPP_LOAD_CTR=$$(podman ps --filter label=com.docker.compose.service=vault-agent --format '{{.Names}}' | head -1) \
	 bash tests/proxy-memory-soak.sh $(ARGS)

red-team:
	@echo "→ bash tests/red-team-breakout.sh $(ARGS) (adversarial breakout attempts; perimeter up first)"
	@OPENTRAPP_AGENT_CTR=$$(podman ps --filter label=com.docker.compose.service=vault-agent --format '{{.Names}}' | head -1) \
	 OPENTRAPP_PROXY_CTR=$$(podman ps --filter label=com.docker.compose.service=vault-proxy --format '{{.Names}}' | head -1) \
	 OPENTRAPP_EGRESS_CTR=$$(podman ps --filter label=com.docker.compose.service=vault-egress --format '{{.Names}}' | head -1) \
	 bash tests/red-team-breakout.sh $(ARGS)

# ── Headless daemon (GUI-free operation; ADR-0019 / ADR-0022) ────────────────
# Build and run ONLY the perimeter daemon, skipping the Tauri GUI entirely.
# opentrapp-daemon links only opentrapp-core + tokio (CI asserts the graph is
# WebKit-free), so `make daemon` does NOT compile the GTK3/WebKit toolchain and
# yields a lean (~30-60 MB) headless binary. This is the GUI-free way to operate
# the perimeter; see docs/headless.md. Operate it via:
#   opentrapp-daemon vault up|down|status|verify|pause|resume|restart
daemon:
	@echo "→ cargo build -p opentrapp-daemon --release (no GUI, no WebKit/GTK)"
	cd app/src-tauri && cargo build -p opentrapp-daemon --release
	@echo "✓ headless daemon: app/src-tauri/target/release/opentrapp-daemon"
	@echo "  operate it with: opentrapp-daemon vault up|status|verify|pause|resume|down"

daemon-run: daemon
	@echo "→ opentrapp-daemon (owns + supervises the perimeter; Ctrl-C to stop)"
	@echo "  NOTE: this brings the FULL perimeter up; on a <8 GB box close heavy"
	@echo "  apps first (see docs/headless.md)."
	app/src-tauri/target/release/opentrapp-daemon
