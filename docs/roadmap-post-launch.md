# Post-launch roadmap

**Created:** 2026-05-04
**Status:** All eight items have a deliverable in `main` as of 2026-05-04. §7 (demo recording) is scaffolded but the recorded video itself is queued for a future maintainer session because it requires a clean recording environment.
**Predecessor:** Pass 8 pre-ship audit shipped v0.3.0 on 2026-05-02. The cleanup and academic-tone passes (commits `4108482`, `1bc288f`, `9b9f6c8`, plus per-submodule work) brought the published codebase to a consistent baseline. This roadmap covers the next layer of work: enriching the project with artifacts that elevate it from "shipped open-source tool" to "publishable security-research project."

The eight areas below are independent. Some are pure documentation; others touch CI or assets. Each block specifies deliverable, scope, dependencies, effort, and definition-of-done.

---

## 1. Formal threat model — **DONE 2026-05-04**

**Deliverable:** [`docs/threat-model.md`](threat-model.md) — STRIDE-classified attacker-capability matrix across six attacker categories (T1–T6), each row carrying capability / STRIDE class / mitigating layer / residual risk / empirical evidence / reference. Cross-referenced from `README.md`, `SECURITY.md`, and `trifecta.md` §7.

**Scope:**

A single document enumerating every attacker capability the perimeter is designed to address, the specific perimeter layer that mitigates each, and the residual risk that remains. The structure should be a STRIDE-style analysis or an attacker-capability matrix — whichever the author finds clearest. Categories at a minimum:

- *Prompt-injection author* (malicious content reaching the agent through Telegram, fetched URLs, or skill bodies)
- *Malicious skill author* (content uploaded to ClawHub that targets agent users)
- *Network man-in-the-middle* (between `vault-proxy` and Anthropic's API)
- *Compromised host* (the user's machine itself is partially compromised before installation)
- *Hostile end user* (the user themselves, or someone who gains access to their Telegram account, attempts to misuse the agent)
- *Side-channel observer* (someone with read access to logs, system metrics, or the host filesystem outside the perimeter)

For each row, the matrix records:

1. The capability the attacker has
2. The mitigating perimeter layer (vault-proxy allowlist, vault-forge scanner, vault-agent hardening, container isolation, etc.)
3. The residual risk that the mitigation does not address
4. The empirical evidence that the mitigation works (cite a verification step, a unit test, or — where unavailable — explicitly mark as untested)

**Dependencies:** None. This is the foundation other documents will cite.

**Effort:** ~1 focused day.

**Definition-of-done:**

- Every defense-in-depth row currently in `docs/trifecta.md` § 7 has a corresponding attacker-capability row in the threat model
- Every layer claims an empirical-evidence citation or is marked "untested"
- Cross-referenced from `README.md`, `SECURITY.md`, and `docs/trifecta.md`
- Reviewed against the OWASP Threat Modeling Process for completeness

---

## 2. Whitepaper

**Deliverable:** `docs/whitepaper.md` — written this session.

**Scope:** An ~8–12 page consolidated paper-style document covering problem statement, threat model overview, system design, defense layers, key innovations (adaptive shell levels and the CDR pipeline), implementation, empirical evaluation, limitations, related work, and future work. arXiv-cs.CR-readable register; cites this repository's ADRs and verification artifacts. Markdown rather than LaTeX so it remains diff-able and reviewable through GitHub.

**Dependencies:** None for v1; benefits from the threat model (§1) and ADRs (§3) being available for citation, but is written as a single document that stands alone.

**Effort:** 2–3 days for v1; an arXiv submission would take an additional pass to align with their author guidelines.

**Definition-of-done:**

- Document renders cleanly on GitHub
- Every empirical claim cites a reproducible source (test file, archive document, external study)
- Cross-referenced from `README.md` as the canonical introduction for security-research readers
- Optional: arXiv preprint identifier added to the README's badge row

---

## 3. Architecture Decision Records (ADRs) — **DONE 2026-05-04**

**Deliverable:** `docs/adr/` directory — eight records covering every distinctive architectural choice in the project, plus a README index.

**Scope:** Adopt the standard ADR format from [adr.github.io](https://adr.github.io/): status / context / decision / consequences / alternatives considered / references. Eight records now landed (formerly three):

- **ADR-0001:** Proxy-side API-key injection (the architectural cornerstone)
- **ADR-0002:** Adaptive shell levels (the capability-sequencing model)
- **ADR-0003:** Content Disarm & Reconstruction (the supply-chain defense pattern)
- **ADR-0004:** Parking openagent-social (the corporate-acquisition decision)
- **ADR-0005:** The "deserve-to-exist" scope test (the 2026-05-02 vision recheck)
- **ADR-0006:** Four-container compose vs. single-container vs. VM-level isolation
- **ADR-0007:** The manifest-driven generic backend
- **ADR-0008:** The choice of Tauri 2 over Electron / native / web-only

No additional ADRs currently queued. New decisions that meet the *When to write an ADR* criteria in [`adr/README.md`](adr/README.md) should be added with the next sequential number.

**Dependencies:** The format itself (templates) needs to be established once. After that each ADR is independent.

**Effort actual:** First three records in the morning session (~3–4 hours); five more in the same evening (~2 hours total at ~25 min each — faster than estimated once the template was set).

**Definition-of-done — DONE:**

- ✅ Eight records in `docs/adr/` with sequential numbering
- ✅ A `docs/adr/README.md` index covering all eight
- ✅ Each record references the live source files (`commands/lifecycle.rs`, `vault-proxy.py`, etc.) so the connection between record and current code is traceable
- ✅ Cross-referenced from `CLAUDE.md` "Key files" table

---

## 4. Reproducibility section + SLSA / SBOM in CI — **DONE 2026-05-04**

**Deliverable:** [`docs/reproduce.md`](reproduce.md) lists every numerical claim in `README.md` with the exact verification command, expected output, and runtime ceiling; [`docs/reproduce.sh`](reproduce.sh) is the executable companion (`--quick` mode runs the offline rows in under 5 seconds). [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) gains a tag-only attestation block: anchore/sbom-action for the CycloneDX SBOM, sigstore/cosign-installer + `cosign sign-blob` for keyless signatures, and actions/attest-build-provenance for the SLSA build-provenance attestation. Verification commands documented in `README.md`.

**Scope:**

`docs/reproduce.md` lists every numerical claim in the README and gives the exact command sequence to verify it independently:

- 87 malware patterns: `wc -l components/openskill-forge/tools/lib/patterns.sh` plus a script that summarises the categories
- 11.9% ClawHavoc rate: link to the underlying ClawHavoc study; document the assumptions
- 24-point verification: `bash components/opencli-container/scripts/verify.sh`
- 42-check orchestrator: `bash tests/orchestrator-check.sh`
- 28 banned terms: `grep BANNED_TERMS app/e2e/user-facing.spec.ts | wc`
- Cargo lib 56/56, vitest 74/74, playwright 25/25: explicit commands and expected outputs

Each command is presented with input, expected output, and the upper-bound runtime. A reader running `./docs/reproduce.sh` (a thin script that wraps the commands) can verify every README claim from a fresh clone.

CI changes:

- Generate an SBOM per release using `syft` (CycloneDX format)
- Generate SLSA Build Level 2 provenance via `slsa-github-generator`
- Sign release artifacts with `cosign` keyless signing (sigstore)
- Upload SBOM, provenance, and signatures as release-page assets

**Dependencies:** The CI build pipeline needs to be functional on each release tag (already true since the v0.3.0 release-build correction).

**Effort:** ~half day for `reproduce.md`, ~1 day for the CI work (most of it is configuring well-known actions; the `slsa-github-generator` flow has a Tauri-specific wrinkle around bundle output paths).

**Definition-of-done:**

- A reader can verify every claim in the README from a clean clone in under 10 minutes
- `cosign verify` works against any v0.3.x asset with the public sigstore identity
- The `syft` SBOM lists every npm and cargo dependency with version + license
- SLSA provenance attestation appears on each release as a `.intoto.jsonl` asset
- README and SECURITY.md updated to point readers at the verification script and the SLSA badge

---

## 5. Comparison with prior art — **DONE 2026-05-04**

**Deliverable:** [`docs/why-not-x.md`](why-not-x.md) — page-or-two differential against nine alternative containment strategies, plus a summary table comparing each on T1 / T2 / T3 / credential-isolation / cross-platform axes. Cross-referenced from `README.md` Limitations and from the threat model.

**Scope:** A page-or-two comparison of the perimeter design against alternative containment strategies a security-aware reader is likely to ask about:

- **OpenClaw's own `sandbox.mode` (Docker)** — what we use, why we layer on top
- **Firejail / bubblewrap** — process-level sandboxing without containers; why insufficient for the agent's tool surface
- **gVisor** — kernel-level sandbox; the next isolation tier we'd consider for VM-equivalent strength without VM overhead
- **Native macOS/Windows app sandboxes** — why platform sandboxes don't compose with the agent runtime's expectations
- **VM-only isolation (the original "just run it on a disposable cloud VM" recommendation)** — what it gives you that this design doesn't, and what this gives you that it doesn't
- **Static skill scanners only (no CDR)** — why CDR was added on top
- **Proxy-only (no container hardening)** — why both layers are needed
- **Disable tools at the OpenClaw config level (no perimeter)** — why insufficient

For each, one paragraph: what they offer, what they don't, the differential against this design.

**Dependencies:** None.

**Effort:** ~half day.

**Definition-of-done:**

- Each alternative has a citation to its source (project URL, paper, security advisory)
- Differential is empirical where possible (capability tables, attack-class coverage), not just rhetorical
- Cross-referenced from `README.md` Limitations section and from the threat model (§1) as the "why not X" justification for design choices

---

## 6. Visual architecture diagrams (Mermaid) — **DONE 2026-05-04**

**Deliverable:** [`docs/diagrams.md`](diagrams.md) collects all five Mermaid drawings (four-container topology, trust tiers, network-isolation matrix, agent-skill-loading flow, AssistantStatus state machine), each captioned and citing its source-of-truth file. `README.md` embeds the topology drawing in the Architecture summary; `trifecta.md` §3 embeds the topology drawing alongside the existing ASCII fallback.

**Scope:** GitHub renders Mermaid natively, so all diagrams live as code in the documents (no binary asset drift). Diagrams to add:

1. The four-container perimeter topology (replaces the ASCII tree in `trifecta.md` §3)
2. The trust-tier flow (replaces the ASCII column in `trifecta.md` §2)
3. The network-isolation matrix (a sequence diagram showing which container can reach which)
4. The agent-skill-loading flow (Karen → forge → forge.scan → forge.cdr → forge-deliveries volume → vault-agent — illustrates the CDR pipeline visually)
5. The state machine for `AssistantStatus` (the seven-state hero machine; useful for contributors touching `app/src-tauri/src/status_aggregator.rs`)

**Dependencies:** None.

**Effort:** ~half day to write all five; iteration to taste afterward.

**Definition-of-done:**

- Each diagram renders correctly on GitHub
- Each diagram is captioned and references the source-of-truth file (`compose.yml`, `status_aggregator.rs`, etc.) so it stays auditable
- ASCII fallbacks retained as `<details>` blocks (some readers will be on platforms without Mermaid rendering)

---

## 7. Demo recording on the landing page — **SCAFFOLDED 2026-05-04 (recording itself queued)**

**Deliverable:** A short video (≤30 seconds) on the `opentrapp.com` landing page showing the setup-wizard flow plus first Telegram chat. [`docs/demo/README.md`](demo/README.md) contains the shooting script (four scenes + a phone cut, 30 seconds total), the recording-environment recipe, the `ffmpeg` conversion commands (MP4 → GIF → WebM → poster), the size-cap targets, and the pre-publish checklist. [`docs/index.html`](index.html) carries a commented-out `<video>` block ready to enable when the assets land. The recording itself needs a clean machine and a maintainer session; that session is the only outstanding work for this item.

**Scope:**

- Record the wizard from `Welcome` → `System Check` → `Connect Your Accounts` → `Setting Up Your Assistant` → `Complete`. Optional: cut to a phone screen showing the first Telegram message and the bot's reply.
- Format: `.mp4` (H.264) at 1280×720; an animated `.gif` fallback under 5 MB for embedded preview.
- Record on a clean macOS or Linux machine to avoid leaking dev-machine state. Tooling: OBS Studio for the desktop capture; ffmpeg for the GIF conversion.
- Embed in the landing page hero area (above-the-fold) and as the GitHub repo's social-preview animated image where supported.

**Dependencies:** A clean recording environment with the v0.3.0 binary installed and a fresh Telegram bot configured for the demo. Cannot be produced inside this repo's CI; needs a human session.

**Effort:** ~half day total, dominated by re-takes to get a clean recording.

**Definition-of-done:**

- 30-second `.mp4` and `.gif` committed to `docs/demo/`
- `docs/index.html` hero block includes a `<video>` tag with the GIF as poster fallback
- Deployed to Hetzner; verified via curl that the asset is served
- Filed as the first asset in the v0.4.0 release (separate from the binary attachments)

---

## 8. CONTRIBUTING.md and CODE_OF_CONDUCT.md — **DONE 2026-05-04**

**Deliverable:** [`CONTRIBUTING.md`](../CONTRIBUTING.md), [`CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md), and [`.github/pull_request_template.md`](../.github/pull_request_template.md) at the repository root. CONTRIBUTING covers cloning with submodules, submodule discipline, the five test gates, the 28-reserved-term enforcement, the pull-request workflow, and security-sensitive-contribution handling. CODE_OF_CONDUCT is structured around the Contributor Covenant 2.1 framework with phrasing chosen for a calmer, more invitational register; the security-contact email matches `SECURITY.md`. The PR template walks contributors through type-of-change, test-gate confirmation, manifest-contract checkboxes, user-facing-surface checks, and documentation review.

**Scope:**

`CONTRIBUTING.md` covers:

- How to clone with submodules (`--recurse-submodules` plus the SSH-vs-HTTPS gotcha)
- The submodule-discipline rules (already documented in `CLAUDE.md` § 8 — concise restatement here)
- The five test gates that must stay green (`cargo test --lib`, `npm test`, `npx tsc --noEmit`, `npx playwright test`, `bash tests/orchestrator-check.sh`)
- The 28-banned-terms enforcement (and how to add new terms when needed)
- The PR template (issue first; sketch the change; then PR)
- How to run the perimeter locally for manual verification
- Contact channels for security-sensitive contributions (point at `SECURITY.md`)

`CODE_OF_CONDUCT.md`: adopt the [Contributor Covenant 2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/) verbatim, with the contact e-mail set to `albertdobmeyer@proton.me` (matches `SECURITY.md`).

**Dependencies:** None.

**Effort:** ~1–2 hours.

**Definition-of-done:**

- Both files render correctly on GitHub
- GitHub auto-detects both and surfaces them in the contributor flow (visible at `Insights → Community Standards`)
- The PR template at `.github/pull_request_template.md` references both files

---

## Recommended sequencing

1. **§3 ADRs** (this session) — establishes the format and seeds the citation graph
2. **§2 Whitepaper** (this session) — the consolidating document; cites ADRs
3. **§1 Threat model** — needs the most focused thinking; benefits from being written after the whitepaper has crystallised the design narrative
4. **§4 Reproducibility + SLSA / SBOM** — engineering rigor; mostly mechanical once the spec is written
5. **§5 Prior-art comparison** — quick win once §1 is done
6. **§8 CONTRIBUTING / CoC** — quick win, can be done at any point
7. **§6 Mermaid diagrams** — polish pass on existing docs
8. **§7 Demo recording** — last because it benefits from a stable v0.3.x release and a clean machine

§1, §3, §5, §8 can be done in parallel by different contributors. §6 and §7 are visual-polish work and should batch with whatever the next visible release is.
