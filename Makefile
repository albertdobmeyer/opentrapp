# Lobster-TrApp — common operator commands.
#
# Conventions:
#   - Targets group by area (test gates, dogfood, audits, perimeter ops).
#   - Each target prints what it's doing before it runs.
#   - Long-running targets print a budget hint where applicable.
#
# Tested on the maintainer's dev machine (Ubuntu 24.04, podman, ~7 GB RAM).
# CI runs the same commands directly; this Makefile is for local convenience.

.PHONY: help \
        test-rust test-frontend test-tsc test-e2e orchestrator-check verify-all \
        dogfood-tier-a dogfood-tier-b dogfood-tier-c dogfood-tier-d dogfood-full \
        dogfood-fresh-sessions dogfood-restore-sessions \
        audit-rust audit-npm audit-deny audit-all \
        perimeter-up perimeter-down perimeter-status

help:
	@echo "Lobster-TrApp common targets:"
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

perimeter-up:
	@echo "→ podman compose up -d"
	podman compose up -d

perimeter-down:
	@echo "→ podman compose down"
	podman compose down

perimeter-status:
	@echo "── perimeter health snapshot ──"
	@podman ps --filter "name=vault-" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" 2>/dev/null \
		|| echo "(no containers; run 'make perimeter-up')"
