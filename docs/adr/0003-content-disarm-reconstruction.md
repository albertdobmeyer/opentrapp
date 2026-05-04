# ADR-0003 — Content Disarm and Reconstruction for skills

**Status:** Accepted
**Decision date:** 2026-04-02 (CDR pipeline design); reaffirmed 2026-04-15 (architecture v2 redesign)
**Implemented by:** [`components/clawhub-forge/tools/skill-cdr.sh`](../../components/clawhub-forge/tools/skill-cdr.sh); [`components/clawhub-forge/tools/lib/cdr-{intent,parse,prefilter,reconstruct,validate}.{sh,py}`](../../components/clawhub-forge/tools/lib/); [`components/clawhub-forge/tools/lib/patterns.sh`](../../components/clawhub-forge/tools/lib/patterns.sh) (87-pattern catalogue); [`components/clawhub-forge/tools/lib/line-classifier.sh`](../../components/clawhub-forge/tools/lib/line-classifier.sh)
**Verified by:** Forge self-test (`make self-test`); [`components/clawhub-forge/tests/scanner-self-test/`](../../components/clawhub-forge/tests/scanner-self-test/)
**Empirical motivation:** ClawHavoc study (2026-Q1) — 341 of 2,857 published ClawHub skills classified as malicious

---

## Context

OpenClaw skills are downloaded from ClawHub, a third-party registry, as bundles of files: a primary `SKILL.md` (the agent-facing instruction document) plus optional supporting files (shell scripts, Python helpers, YAML configurations, image assets). The agent loads the bundle on demand and treats it as authoritative input.

The empirical baseline is unforgiving. The ClawHavoc study (2026-Q1) sampled 2,857 published ClawHub skills and classified 341 (11.9 %) as malicious — credential exfiltration, persistence mechanisms, command-and-control beacons, payload droppers, all distributed under names that resembled legitimate developer tooling. The `moltbook-ay` skill, for example, masqueraded as a Moltbook-integration helper but contained instructions to download and execute malware via password-protected archives. Several malicious skills reached the top of ClawHub's discovery rankings before being removed.

A pattern-blocklist scanner is the obvious first defense. The clawhub-forge module ships an 87-pattern catalogue mapped to MITRE ATT&CK techniques, scanning every file in the skill bundle (not just `SKILL.md` — many of the malicious skills hid the payload in supporting files). The catalogue covers credential access, exfiltration, persistence, privilege escalation, container escape, supply-chain attacks, environment-injection, prompt-injection, and a dozen other categories.

Pattern blocklisting has a structural ceiling. It detects *known* attacks. Pattern N+1 — a novel obfuscation, a new persistence technique, a category of injection nobody has yet enumerated — slips through. Adding a zero-trust line-classifier on top (every line in every file must match a known-safe pattern, otherwise the skill is quarantined) raises the bar significantly, but at the cost of producing false positives on legitimate skills that legitimately use unusual line shapes (regex-heavy skills, skills that reference security patterns for educational purposes). Both layers are necessary; neither is sufficient.

A more fundamental limitation: both layers ask the same question — *"is this artefact safe?"* — and an answer to that question is structurally only as good as the catalogue of known badness or the catalogue of known goodness it draws on.

## Decision

The clawhub-forge pipeline applies **Content Disarm and Reconstruction** to every skill that passes the static layers. The original artefact is treated as a parse target, not as the deliverable. The pipeline asks a different question — *can this artefact's intent be re-expressed in a known-safe form?* — and if the answer is yes, the original artefact is discarded and the rebuilt version is delivered.

The full pipeline runs inside `vault-forge` (network-isolated from `vault-agent`) and produces, for every input skill, either a clean rebuilt artefact + signed clearance report, or a quarantine record + reason:

```
       download                   parse intent             rebuild from intent
ClawHub ────────► quarantine ────► structural model ────► clean skill artefact
                  (vault-forge)    (parser, in-container) (generator, in-container)
                       │                                            │
                       ▼                                            ▼
                  static scanner                              line verifier
                  (87 patterns,                               (every line classified;
                   MITRE ATT&CK)                               failure quarantines)
                       │                                            │
                       └─── sign clearance report ─── deliver to vault-agent ───►
                                              (write-only shared volume; SHA-256;
                                               vault-agent reads, verifies,
                                               loads only on signature match)
```

Five properties of this design are the design's substance:

**(a) The original artefact is never delivered.** Whatever side effects the original skill was supposed to produce (intentionally or by accident) are replaced by the side effects of the rebuilt version. Payloads hidden in formatting tricks (zero-width Unicode, trailing whitespace, layered base-64), in `<!-- HTML comments -->`, in unconventional line shapes, in encodings the static scanner does not normalise — none survive the parse-and-rebuild step.

**(b) The rebuilt artefact is signed.** A SHA-256 clearance report accompanies every delivery. `vault-agent` rejects skill artefacts that do not match the signed hash. A skill that the user side-loads into the workspace bypassing forge will fail this check and be refused by the agent. This converts a user-side workflow mistake (manually copying a skill in to bypass the slow scan) into a structural failure rather than a silent compromise.

**(c) The pipeline is deterministic per input.** Re-running CDR on the same source produces the same rebuilt artefact (modulo intentional rebases when the parser improves; those are versioned and tracked). This makes the pipeline auditable: a reviewer can verify post-hoc which input produced which output.

**(d) The pipeline runs offline.** No network access during scan, classify, parse, or rebuild. The pattern catalogue, the safe-line classifier, and the parser are all in-image. The only external interaction is the initial download into quarantine.

**(e) Failure quarantines.** A line that fails classification, a parse that fails completion, or a static-scanner hit at `CRITICAL` severity all result in the same outcome: the skill is held in quarantine and not delivered. The agent learns nothing about the failed skill except that it failed; the failure mode is uniform.

The implementation lives in `components/clawhub-forge/tools/skill-cdr.sh` (the pipeline driver) and the `tools/lib/cdr-*` scripts (the per-stage primitives — pre-filter, intent extraction, parser, reconstructor, validator).

## Consequences

### Positive

- **Novel attacks are mitigated structurally.** An attack that the 87-pattern catalogue does not yet recognise still fails CDR if it relies on the original artefact's bytes reaching the agent. Whitespace-tricks, comment-hiding, unicode-confusables, layered encoding, polyglot files — none of these survive parse-and-rebuild.
- **The pipeline composes cleanly with the existing scanner and line classifier.** Three layers, each asking a different question: blocklist (does this match known-bad?), allowlist (does every line match known-safe?), reconstruct (can we re-express intent in known-safe form?). A skill must pass all three to be delivered.
- **The clearance report is a portable trust artefact.** Other consumers of ClawHub skills could consume the same clearance report (not implemented; available as an integration point). The format is documented in [`components/clawhub-forge/docs/archive/specs/2026-04-02-security-certificate-system.md`](../../components/clawhub-forge/docs/archive/specs/2026-04-02-security-certificate-system.md).
- **The pipeline is auditable.** Every quarantine record, every clearance report, and every CDR rebuild is logged. A reviewer can reconstruct the pipeline's per-skill decisions post-hoc.
- **Operationally cheap per skill.** Typical CDR runtime is 1–3 seconds per skill; the dominant cost is the static scanner's pattern matching across all files. The CDR rebuild itself is template-driven and fast.

### Negative

- **CDR is not a panacea.** A skill whose semantic intent is itself malicious — *"prompt the user for their SSH passphrase"* — is structurally indistinguishable from a legitimate skill that asks for credentials, and CDR will faithfully reconstruct the malicious intent. The defense relies on the layered structure (the static scanner catches known patterns, the line classifier catches obfuscation, CDR catches novel-encoding attacks); it does not catch novel-intent attacks. Mitigation: the agent's tool surface (ADR-0002) and the proxy allowlist (ADR-0001) constrain what an installed-but-malicious skill can actually do.
- **Reconstruction may lose nuance.** A legitimate skill that uses unusual formatting for legitimate reasons (e.g. a Markdown skill that intentionally embeds shell quoting in a way the parser doesn't fully model) may produce a rebuilt artefact that is functionally inferior to the original. In practice this happens for fewer than 1 % of skills the forge has tested; it is observable as the skill failing its own behavioural test after CDR.
- **The parser is the trusted base.** A bug in the CDR parser that mis-interprets a skill could deliver a structurally different artefact than the user expected. This is a real risk; the parser is exercised by the forge self-test (`make self-test`) against `tests/scanner-self-test/known-bad.md` and `known-clean.md` fixtures, but the test corpus is not exhaustive. Treat parser bugs as security bugs (per `SECURITY.md`); report them via the documented vulnerability-disclosure path.
- **Skill maintainers must accept that their `SKILL.md` is not what users get.** Users get the rebuilt version. A skill maintainer who relies on a specific token-byte-level layout in their `SKILL.md` will be surprised when CDR's output differs. The skill template documentation explains the constraint; in practice this is rarely an issue because skills are intent-document-style, not byte-precise format-style.
- **The two-stage post-install defence requires both stages.** Both `skill-scan.sh --strict` and `skill-verify.sh --strict` must pass for the skill to leave quarantine. A configuration error that runs only one of them creates a silent gap. The `--strict` flag is enforced in production; the `make publish` pipeline gates on both.

### Neutral

- **Trust manifests for first-party skills.** The forge's own 25 reference skills carry `.trust` files with SHA-256 content hashes that allow them to skip re-verification when unchanged. External skills never carry trust manifests; they always go through full CDR. This is an operational optimisation, not a security posture; the trust manifests are purely a freshness check, not a substitute for verification.

## Alternatives considered

**(A) Static blocklist scanner only (the typical npm-audit-style approach).** Rejected because it has a known structural ceiling; novel attacks slip through.

**(B) Static blocklist + line classifier, no CDR.** This was the prior state (Phase 1 of the forge). Rejected because the layers ask the same kind of question — *is this artefact safe?* — and the answer is structurally only as good as the catalogues. CDR adds a different *kind* of question.

**(C) Sandboxed execution of skills before delivery (a dynamic-analysis approach).** Run the skill in a disposable nested container, observe its behaviour, deliver if behaviour is benign. Rejected because:
1. Many malicious skills are structured to behave benignly when they detect they are in a sandbox (sandbox-evasion is a well-studied attacker capability).
2. Running the skill requires interpreting it, which collapses the trust boundary CDR is designed to maintain.
3. The runtime cost (seconds per skill) and the operational complexity (nested-container resource accounting) are high.

**(D) Manual review of every skill.** Have the user (or a trusted CLI coordinator) review the source of every skill before installation. Rejected because the operational burden defeats the value proposition (non-developer users installing useful skills); mathematically also limited because review fatigue produces false negatives.

**(E) Only allow skills from a trusted list.** Have a vetted-allowlist of skill authors and only install from them. Rejected because:
1. The allowlist needs maintenance and someone needs to do the vetting (the same problem displaced one level).
2. ClawHub's discovery model is open; restricting to a private allowlist defeats the purpose of using ClawHub at all.
3. Empirically, several allowlisted authors have been compromised (account takeover) in adjacent ecosystems; allowlisting authors does not generalise to allowlisting *current state* of a published skill.

CDR composes with all of these (the pipeline could feed only into a trusted-authors allowlist, or could be augmented with sandboxed execution as a future research direction). What it does not require is any of them, and that decoupling is the architectural value.

## References

- Companion architecture document: [`docs/trifecta.md`](../trifecta.md) §4.2 (clawhub-forge supply-chain defense)
- Whitepaper: [`docs/whitepaper.md`](../whitepaper.md) §6 (Content Disarm and Reconstruction)
- Forge README: [`components/clawhub-forge/README.md`](../../components/clawhub-forge/README.md) — pipeline stages, pattern catalogue, verifier verdicts
- Self-test: `make self-test` from [`components/clawhub-forge/`](../../components/clawhub-forge/)
- Implementation: [`components/clawhub-forge/tools/skill-cdr.sh`](../../components/clawhub-forge/tools/skill-cdr.sh) and the `tools/lib/cdr-*` scripts
- Pattern catalogue source: [`components/clawhub-forge/tools/lib/patterns.sh`](../../components/clawhub-forge/tools/lib/patterns.sh)
- Line classifier source: [`components/clawhub-forge/tools/lib/line-classifier.sh`](../../components/clawhub-forge/tools/lib/line-classifier.sh)
- Empirical motivation: ClawHavoc study (2026-Q1); `moltbook-ay` trojanised skill (documented in [`components/clawhub-forge/docs/research/security-report.md`](../../components/clawhub-forge/docs/research/security-report.md))
- Historical: [`components/clawhub-forge/docs/archive/specs/2026-04-02-content-disarm-reconstruction.md`](../../components/clawhub-forge/docs/archive/specs/2026-04-02-content-disarm-reconstruction.md) (the original CDR design document, archived)
