# Skill-Installation Policy — Open Question

**Created:** 2026-05-06
**Status:** Decision pending. Spec records the question + the two options + my recommendation; product owner makes the call.
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

## What this requires

- `forge.explore` (or equivalent): a discovery command that takes a query string and returns candidate skills with metadata. May already exist (forge has `explore`, `scan`, `scan-all` commands per its manifest); needs a session to verify the API matches.
- Bot system-prompt update (in `components/openclaw-vault/scripts/entrypoint.sh`'s CONSTRAINTS.md heredoc, or a new `SKILL_POLICY.md` workspace file): explicit guidance that *"find me a skill that does X"* is an acceptable intent, with the forge-mediated discovery flow described.
- A new dogfood scenario A4-extended that exercises the full discovery path end-to-end. The current A4 stops at the conversation-layer refusal; an updated A4 would assert that forge.explore + forge.scan + the user's pick + vault delivery all happen for a `"find me a CSV skill"` prompt.

## Cost / time

- Forge API verification: 1 hour (read forge's `component.yml` + tools/skill-explore.sh or equivalent)
- Bot prompt update: 2 hours (write the policy guidance; iterate against a fresh dogfood session to check it sticks)
- Dogfood update + re-run: 2 hours (extend A4 in `tests/dogfood/test_full_arc.py`, re-run, capture new findings)

Total: ~half day of focused work.

## Out of scope here

- Whether ClawHub itself should rank candidates by safety / popularity / recent maintenance — that's a registry concern, not ours.
- Whether the forge.explore step should be allowed in Hard Shell — it shouldn't (Hard Shell is chat-only). The new policy applies at Split Shell and Soft Shell only.
- Whether to surface the candidate list as inline Telegram options (with reply-to-this-message keyboard) vs free-form text — UX detail; either works.

## References

- 2026-05-05 dogfood findings: [`2026-05-05-dogfood-full-arc-findings.md`](2026-05-05-dogfood-full-arc-findings.md) §A4
- ADR-0003 (CDR): [`../adr/0003-content-disarm-reconstruction.md`](../adr/0003-content-disarm-reconstruction.md) — establishes that forge is the safety net for skill content
- Forge component: [`../../components/clawhub-forge/component.yml`](../../components/clawhub-forge/component.yml)
- Architecture: [`../trifecta.md`](../trifecta.md) §4.2 (clawhub-forge supply-chain defense)
