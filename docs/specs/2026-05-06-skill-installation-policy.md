# Skill-Installation Policy — Open Question

**Created:** 2026-05-06
**Updated:** 2026-05-06 (decision + architectural correction)
**Status:** Accepted (Option B), with the architectural correction recorded below. First-pass implementation landed; full bot-mediated orchestration deferred (see *Implementation scope*).
**Trigger:** the 2026-05-05 dogfood run (see [`2026-05-05-dogfood-full-arc-findings.md`](2026-05-05-dogfood-full-arc-findings.md)) found that the keystone Tier-A scenario (A4 — *"install a CSV-formatting skill from ClawHub"*) **never reached the forge pipeline.** The bot refused at the conversation layer, so `forge.scan` + line-classifier + CDR + write-only-volume delivery + agent reload never ran for the Karen-flow user.

The conversation-layer refusal IS a defence — it kept Karen from blindly installing an unknown skill. But the *architectural* defence (forge is supposed to be the safety net, hence its existence) was hidden.

## The question

**When the user says *"find me a skill that does X and install it"*, who chooses the skill?**

- **Option A — current behaviour: the user must name the skill.** The bot refuses *"find me a CSV skill"* requests. Karen has to know the skill's exact name, copy-paste it, then the bot installs (and `forge.scan` runs at install time as designed).
- **Option B — defer skill choice to forge.** When the user says *"find me X"*, the bot asks `forge.explore` for candidate skills, runs `forge.scan` on each, presents the user with cleared options (with their clearance-report verdicts visible), and installs the user's pick. Forge is the safety net for both *which* skill and *whether* it's clean.

## What's at stake

| Dimension | Option A (status quo) | Option B (defer to forge) |
|---|---|---|
| Karen experience | She has to know skill names; the discovery happens elsewhere | She can ask in natural language; the architecture handles discovery + safety in one flow |
| Demonstrates forge | Only on user-supplied skill names (rare in practice) | Every Karen install request runs the full pipeline |
| Failure mode | User installs a malicious skill they typed the name of (forge.scan still catches it; the conversation-layer refusal added nothing) | Bot suggests something the user didn't ask for; user could miss a banner that says "rejected as malicious" if UX is sloppy |
| Trust narrative | "The bot is conservative; it won't install anything without explicit consent" | "The bot can browse the registry safely because forge is between the agent and the filesystem" |
| Fits the architecture | Suboptimal — duplicates forge's gatekeeping at a layer above forge | Direct — forge does the job it was designed for |

## My recommendation: Option B with explicit user confirmation per install

The architectural answer. Specifically:

1. User: *"find me a CSV-formatting skill"*
2. Bot calls `forge.explore "csv format"` (a discovery command — needs adding if not already present)
3. Bot calls `forge.scan` on the top 3–5 candidates
4. Bot presents the user with the cleared candidates and their clearance-report verdicts in plain language: *"I found three. The one that looks safest is X — its clearance report is clean. Want me to install it, or pick a different one?"*
5. User picks; bot installs (which under the hood is `vault.install-skill <X>` against the forge-deliveries volume)

**Why this is the right call:**

- It's what the architecture was *designed* for. The whole point of the four-container perimeter is that forge can sit between the user's intent and the agent's filesystem; making forge the discovery layer doesn't add new attack surface, it just exercises an existing one.
- The Karen experience is meaningfully better. *"I want a thing that does X"* is the natural way to ask; *"I want skill `csv-formatter-pro@1.2.3`"* is the developer's way.
- The conversation-layer refusal Karen currently sees adds zero defensive value. A user who wants to install something malicious will type the name; forge.scan catches it either way. Refusing the *natural-language* version of the same intent doesn't make it safer, it just makes the bot less useful.
- The "explicit confirmation per install" step preserves the bit that *does* matter: the user must affirmatively pick. That's the same model as `git push` — git could push automatically when files change; it doesn't, because asking is cheap and the user values the gate.

## Architectural correction (2026-05-06)

The original recommendation said *"Bot calls `forge.explore`, then `forge.scan`, then presents the user with cleared candidates."* Verification of the actual code surfaces shows the bot **cannot** invoke forge directly:

- The bot runs inside `vault-agent`. Its capabilities are enumerated in [`components/openclaw-vault/scripts/entrypoint.sh`](../../components/openclaw-vault/scripts/entrypoint.sh) (the CONSTRAINTS.md heredoc): workspace file I/O, a small safe-bin set, memory search, Telegram, vision. **No outbound IPC** to other containers.
- The agent's [tool manifest](../../components/openclaw-vault/config/tool-manifest.yml) does not register a forge bridge.
- Network-wise, vault-agent and vault-forge are on different compose networks; the only shared surface is the `forge-deliveries` volume, which is *read-only on the agent side* and carries finished, vetted skills — not bidirectional RPC.

**The architectural reality is: the user is the bridge.** The flow is:

1. User: *"find me a CSV-formatting skill"* (Telegram)
2. Bot: *"I can't browse the skill library from in here, but your desktop app can. Open it and use **Browse the Skill Library** with `csv format` as the search term. It'll show me a list of clean, vetted candidates and I'll help you pick one."*
3. User runs the desktop action → forge does `registry-explore` → results render in the GUI.
4. User picks → desktop app runs the existing `safe-download` workflow → forge does CDR + safety scan + delivery to `forge-deliveries`.
5. Skill arrives in the bot's workspace; bot confirms it can use it; user proceeds with the original task.

This works with what already exists. The original spec's framing would have required new cross-container plumbing (a forge-RPC tool registered in OpenClaw's tool-manifest, a network path between agent-net and forge-net, additional hardening review). The user-bridge model gets the same Karen experience without that lift.

## Implementation scope (what landed)

| Piece | Where | Status |
|---|---|---|
| Working **Browse the Skill Library** action exposed to the desktop GUI | clawhub-forge#2 — repaired `id: explore` manifest entry; was broken (passed `SKILL=` to a Makefile target that reads `QUERY=`) | Landed |
| Bot policy guidance: *"find me a skill"* is acceptable; hand off to the desktop action; confirm after install | openclaw-vault#2 — new section in CONSTRAINTS heredoc | Landed |
| Submodule bumps + this spec status flip | this PR (lobster-trapp) | Landing |

## Implementation scope (deferred)

| Piece | Why deferred | Trigger to revisit |
|---|---|---|
| Dogfood scenario A4-extended | Needs a live API session; programmatic harness can't drive a desktop GUI click. The bot's *conversational* hand-off is testable on the next live run | Next operator-driven dogfood session |
| Inline candidate-list rendering in Telegram (vs free-form text from the desktop GUI) | UX detail; the spec already marks this as out of scope | Only if free-form text proves confusing in the next dogfood |
| Genuine bot → forge RPC | Requires a new OpenClaw tool registration, allowlist updates, and security review; user-bridge model is sufficient for the keystone flow | Only if the user-bridge model fails the dogfood read |

## Cost / time (actual)

- Forge manifest verification: 30 min (turned up the bug as a side effect — a quiet win)
- Bot prompt update: 30 min (the section in CONSTRAINTS heredoc)
- Spec correction + this rewrite: 30 min

Total: ~90 min, vs. the originally-estimated half-day. The deferred dogfood re-run is unchanged at ~2 h and stays on the operator queue.

## Out of scope here

- Whether ClawHub itself should rank candidates by safety / popularity / recent maintenance — that's a registry concern, not ours.
- Whether the forge.explore step should be allowed in Hard Shell — it shouldn't (Hard Shell is chat-only). The new policy applies at Split Shell and Soft Shell only.
- Whether to surface the candidate list as inline Telegram options (with reply-to-this-message keyboard) vs free-form text — UX detail; either works.

## References

- 2026-05-05 dogfood findings: [`2026-05-05-dogfood-full-arc-findings.md`](2026-05-05-dogfood-full-arc-findings.md) §A4
- ADR-0003 (CDR): [`../adr/0003-content-disarm-reconstruction.md`](../adr/0003-content-disarm-reconstruction.md) — establishes that forge is the safety net for skill content
- Forge component: [`../../components/clawhub-forge/component.yml`](../../components/clawhub-forge/component.yml)
- Architecture: [`../trifecta.md`](../trifecta.md) §4.2 (clawhub-forge supply-chain defense)
