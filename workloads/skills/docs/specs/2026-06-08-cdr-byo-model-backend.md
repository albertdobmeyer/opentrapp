# Spec — CDR: pin the small model, add BYO-model (OpenAI-compatible) backend, honest docs

**Status:** Proposed → implementing
**Date:** 2026-06-08
**Touches security boundary?** The CDR intent-extraction model call only. No change to
the 87-pattern scanner, the line classifier, the gated-publish pipeline, or the
"original file is never delivered" invariant.

## Why

An honest audit (workflow assessment, 2026-06-08) found three leanness/honesty gaps:

1. **Footgun:** `tools/lib/cdr-intent.sh` defaults `CDR_MODEL` to `qwen2.5-coder:7b`
   (~4.7 GB) when `config/cdr.conf` is not sourced, even though `cdr.conf` ships
   `1.5b` (~990 MB). A misconfiguration silently 5×'s the footprint.
2. **Not truly BYO-model:** the model request is hardcoded to Ollama's native
   `/api/generate` format (`{model, system, prompt, format, stream}`). A user who
   already runs an LLM (their own agent's model, an OpenAI-compatible server, a
   managed endpoint) **cannot** reuse it for CDR — they must run an Ollama-protocol
   server. Docs (`docs/forge-identity-and-design.md`) claim "works with any LLM
   backend," which is aspirational, not true today.
3. **Overclaims:** README/spotlight/ADR overstate novelty and determinism (see §Docs).

The product's value proposition is a **lean, modular, silent** background harness.
The offline scanner already is that (no model, on-demand). The CDR path is the only
heavy part, and it should (a) default small, (b) let users reuse a model they
already run instead of forcing a second download.

## Change 1 — pin the small model (leanness)

`tools/lib/cdr-intent.sh`: change the fallback default from `qwen2.5-coder:7b` to
`qwen2.5-coder:1.5b`, matching `cdr.conf`. The 1.5b is sufficient for intent
extraction (parser failures are schema-detectable and retry-recoverable per
ADR-0015). No one should land on the 4.7 GB model by accident.

## Change 2 — BYO-model: OpenAI-compatible request path

Add a backend selector so the same CDR step can talk to either an Ollama-native
endpoint OR any OpenAI-compatible chat-completions endpoint (which includes the
user's own agent model, LM Studio, vLLM, a managed API, or even Ollama's own
`/v1/chat/completions`).

- **`config/cdr.conf`** gains:
  - `CDR_API_FORMAT="ollama"` — `ollama` (default) or `openai`.
  - `CDR_API_KEY=""` — optional bearer token for the OpenAI-compatible endpoint.
  - Documented example for pointing at an OpenAI-compatible server.
- **`tools/lib/cdr-intent.sh`**:
  - Source the two new vars (defaults: `ollama`, empty key).
  - **Reachability check is format-aware:** `ollama` → `GET {host}/api/tags`;
    `openai` → `GET {host}/v1/models` (best-effort; on failure fall through to the
    existing cached-intent / error path, unchanged).
  - **Request builder branches on `CDR_API_FORMAT`:**
    - `ollama` (unchanged): POST `{model, system, prompt, format:"json", stream:false}`,
      read `result["response"]`.
    - `openai`: POST `{model, messages:[{role:"system",...},{role:"user",...}],
      response_format:{type:"json_object"}, stream:false}` with an `Authorization:
      Bearer <key>` header when `CDR_API_KEY` is set; read
      `result["choices"][0]["message"]["content"]`.
  - Everything downstream (JSON parse, `error` handling, caching, repair-hint loop)
    is identical — only the transport/format differs.

This is **bring-your-own-endpoint that now spans both protocols**, so a user can
reuse a model they already run rather than download a dedicated one.

**Both model-using paths are covered (update 2026-06-08):** the same two-protocol
treatment was applied to skill creation (`create-draft.sh`) — it shares
`CDR_API_FORMAT`/`CDR_ENDPOINT`/`CDR_API_KEY`, branches its request the same way
(no `response_format`, since it emits markdown not JSON), and its reachability probe
was also fixed to use the configured endpoint host instead of a hardcoded
`localhost`. Validated end-to-end: both paths generate a valid, Clean-scanning
`SKILL.md` via Ollama and via Ollama's OpenAI-compatible `/v1/chat/completions`.

## Change 3 — honest docs

- **README (root + `workloads/skills/README.md`):** lead the skills story with
  "offline scanner, on-demand, ~0 resting RAM"; state the CDR cost plainly
  ("the optional CDR rebuild needs an LLM endpoint — a local ~1 GB model, OR your
  own existing model via an OpenAI-compatible endpoint"). Recalibrate "five
  independent defences" to what's true (a blocklist + an allowlist + the CDR
  rebuild; the post-install re-scan reuses the same pattern set).
- **`docs/skills-spotlight.md`:** keep "first CDR applied to agent skills" but drop
  any implication the *concept* is novel; soften "zero-trust line verifier" to
  "line-by-line allowlist classifier."
- **ADR-0003:** fix the determinism claim — CDR uses a best-effort describe-with-
  repair loop; it is NOT guaranteed identical per input (the code proves this).
- **`docs/forge-identity-and-design.md`:** the "any LLM backend" claim becomes true
  for the CDR path via Change 2; make the wording precise ("any Ollama-native or
  OpenAI-compatible endpoint").

## Verification

- **Regression (Ollama path):** `make cdr FILE=<real opencode SKILL.md>` with default
  config → rebuilds clean, post-verify Clean (as today).
- **BYO-model (OpenAI path):** set `CDR_API_FORMAT=openai`,
  `CDR_ENDPOINT=http://localhost:11434/v1/chat/completions`, `CDR_MODEL=qwen2.5-coder:1.5b`
  → `make cdr` rebuilds clean. This proves the OpenAI-compatible path end-to-end
  against a real server (Ollama's own OpenAI-compat endpoint), demonstrating a user
  could point at their own model.
- `make self-test` unaffected (patterns.sh untouched); `make scan` unaffected (no model).
