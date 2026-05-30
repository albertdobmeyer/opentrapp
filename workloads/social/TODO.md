# OpenAgent-Social — TODO

Tracked gaps from the 2026-03-03 audit. All items resolved during Phases 1-3.

---

## No Automated Tests

- [x] 30 tests across 4 test files (Phase 2, `tool-runner.sh` framework ported from forge)

---

## safe_patterns Not Wired

- [x] Wired in Phase 1 (commit `512fd2e`)

---

## No Executable Bits on Fresh Clone

- [x] Fixed with `.gitattributes` in Phase 1 (commit `d5e50bd`)

---

## eval in curl (Minor Security Surface)

- [x] Replaced with array-based curl in Phase 1 (commit `86f4733`)

---

## API Availability Unknown

- [x] Offline mode added in Phase 3 — `--file` flag for census, fixture data for all tools, `make check-api` target
