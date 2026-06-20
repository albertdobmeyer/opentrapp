# Skill install flow: current state, interim path, target end-state

> Audience: contributors and maintainers who need a straight answer to *"how
> does a user actually install a skill in this thing today, and what's the
> plan?"*. Companion to [`docs/skills-spotlight.md`](skills-spotlight.md) (the
> *why* forge exists) and [ADR-0003](adr/0003-content-disarm-reconstruction.md)
> (the *how* of CDR). Pinned by `tests/orchestrator-check.sh §15`.

## TL;DR

| Phase | Where it works | What the user does | Status |
|-------|----------------|--------------------|--------|
| **v0.6 (today)** | Host CLI only | Power user runs `make` targets in `workloads/skills/`; certified skills land in the agent's workspace via the shared volume | **shipped** |
| **v0.7 (interim GUI)** | Desktop app + bot | Bot says "I can't install skills directly; here is the exact command to paste into a terminal", or GUI shows a "Paste this command" panel; user runs it; new skill appears | designed below, not built |
| **v0.8 (target)** | Desktop app, end-to-end | "Skills" page in user mode: search → candidate cards → one-click install with progress | sketched below, not designed |

The shipped `install-skill` orchestrator workflow + forge's `safe-download` +
agent's `install-skill` command already implement the *plumbing*. What's
missing is the **GUI surface** that triggers the plumbing without requiring a
terminal. Until that ships, the bot must not pretend the surface exists.

## Why this needed deciding

Three independent observations forced the issue:

1. **Bot vs reality drift.** The agent's CONSTRAINTS.md (in
   `workloads/agent/scripts/entrypoint.sh`) used to instruct the bot to tell
   users to *"open the desktop app and use **Browse the Skill Library**"*.
   That feature does not exist in `app/src/`; there is no Skills page, no
   library browser, no install dialog. The bot was promising vaporware.
2. **A4 in the 2026-05-20 dogfood E2E.** The bot, asked to install a CSV
   skill, refused honestly: *"I can't browse ClawHub, no web access. No
   blind installs. I'll review it with you before installing."* This is
   correct security posture but the user had nowhere to go from there.
   `docs/specs/2026-05-20-dogfood-full-arc-findings.md` records the gap.
3. **Thread D's editorial spotlight** ([`docs/skills-spotlight.md`](skills-spotlight.md))
   pitches forge as the supply-chain defence, and the inevitable follow-up
   question is "how do I, the user, actually trigger it?". Until this doc
   exists, the spotlight is hollow.

## What's shipped today (v0.6 path)

`vault-skills` is fully functional via its host-side CLI. The shipped path:

```bash
# 1. Bring the perimeter up (or use the desktop app, which does the same).
podman compose up -d

# 2. Download + scan + rebuild a skill, end to end.
cd workloads/skills
bash tools/skill-cdr.sh <skill-name>

# 3. The certified rebuild lands at workloads/skills/exports/<skill-name>/
#    The agent reads it via the shared `skills-deliveries` volume (read-only
#    from the agent's side).
```

Each stage of `skill-cdr.sh` is independently runnable for debugging:

| Stage | Tool | What it does |
|-------|------|--------------|
| Download | `tools/skill-download.sh` | Pulls skill from ClawHub into the quarantine volume |
| Pre-filter | `tools/skill-scan.sh` | 87-pattern + 16-pattern scan, MITRE-mapped |
| Validate | `tools/lib/cdr-validate.py` | Schema + frontmatter + line-verifier check |
| Reconstruct | `tools/lib/cdr-reconstruct.py` | Parses intent → rebuilds artefact from clean templates |
| Quarantine | (any failure) | Original + rebuild dropped to `quarantine/`, never reaches the agent |

**Resolved (historical false-quarantine):** the Ollama-backed reconstruct glue
stage of `skill-cdr.sh` once failed closed on *clean* skills (would block a
legit skill, never passed a malicious one). That issue is fixed in current
HEAD: stages 4-7 now run inside the retry-repair loop, so the pipeline
completes end-to-end on legitimate skills.

### What the bot tells the user today (post-Zone-4b)

The CONSTRAINTS.md heredoc is updated to be honest:

> *"I can't install skills directly; installing requires running a setup
> command from a terminal on this computer. If you can ask whoever set this
> up to run `cd workloads/skills && bash tools/skill-cdr.sh <skill-name>`,
> the new skill will appear in my workspace and I'll let you know when I can
> see it."*

That's an honest answer the bot can give. It names a concrete next action
(ask the operator), it names the exact command, and it sets expectations
about confirmation arriving via Telegram once the install completes. No
promise of a GUI feature that doesn't exist.

## v0.7 interim: "Paste this command" GUI

The next step is to lift the operator-runs-a-CLI-command path into the GUI
without building the full search UI. Two viable shapes:

**A. Bot-initiated, GUI displays.** The bot continues to refuse direct
install. When asked, it emits a structured marker (e.g.
`{"action": "install-skill", "name": "csv-pipeline"}`) which the desktop app
detects in the Telegram stream and surfaces as a banner: *"Your assistant
asked to install **csv-pipeline**. [Approve] [Show command] [Reject]"*. The
Approve button runs the existing `install-skill` orchestrator workflow.

**B. GUI-initiated, bot confirms.** A minimal "Skills" page in user mode
with one input (skill name) + one button (Install). The button triggers the
same workflow. When the skill lands in the agent's workspace, the bot
detects it and confirms via Telegram.

**Recommendation: A.** Keeps the conversational flow that's already
working (bot is the discovery surface), avoids building a separate
"navigate to Skills page" mental model, and the marker-message pattern is
reusable for any future bot→GUI handoff (export, share, settings change).
Shape B can ship later as a power-user fallback if discovery via the bot
is too slow.

What needs to be built for option A:

- A frontend hook that listens to the Telegram stream for the
  `install-skill` marker (or any future structured-action marker).
- A confirmation banner component (action + reject + show-command).
- The `executeWorkflow("install-skill", { skill_name })` call (already
  exists in `app/src/lib/tauri.ts`).
- A progress indicator (the bootstrap pipeline's step-indicator pattern
  shipped in Zone 1 / `useBootstrapProgress` is the obvious reuse).
- CONSTRAINTS.md updated to instruct the bot to emit the marker when the
  user agrees to an install.

Estimate: one focused dev day. The CDR pipeline now completes end-to-end on
legitimate skills (the former false-quarantine issue is resolved), so the
workflow this GUI invokes is unblocked.

## v0.8 target: full Skills page

Once the v0.7 install path is exercised and stable, the natural follow-up
is a dedicated "Skills" page in the user-mode sidebar (sixth icon, between
Discover and Preferences) with:

- A search box that queries the skill registry via forge (forge already
  has `tools/registry-explore.sh`).
- Candidate cards showing: name, description, author, install count, the
  forge scan verdict.
- One-click install (which runs the v0.7 workflow).
- An "Installed skills" tab showing what's currently in the agent's
  workspace + a "Remove" button for each.

This is the "Browse the Skill Library" surface the old CONSTRAINTS.md
hallucinated. It requires actual design work (the UX rubric in
`docs/specs/2026-04-20-ux-principles-rubric.md` applies); it is not
priority for v0.7.

## Decisions captured

- **Skill install does not require Soft Shell.** It runs out-of-band of
  the agent's shell level, via the orchestrator workflow. Split Shell
  remains the default; the agent never gains direct registry-access
  capability.
- **The agent never initiates an install autonomously.** Every install
  goes through user approval: today via a terminal, in v0.7 via the
  GUI banner, in v0.8 via a one-click button. The Approve gate is
  non-negotiable.
- **The bot's role is discovery, not execution.** It can suggest, it
  can refuse, it can emit a structured request for the GUI to surface,
  but it never reaches forge directly. The "agent cannot influence the
  inspection" property of [`docs/skills-spotlight.md`](skills-spotlight.md)
  depends on this.
- **The CLI path is supported indefinitely.** Even after v0.8, the host
  CLI remains the documented power-user / CI / debugging path. Forge's
  `Makefile` is the source of truth; GUI is a convenience layer over it.

## Cross-references

- [`docs/skills-spotlight.md`](skills-spotlight.md): why forge exists at all.
- [ADR-0003](adr/0003-content-disarm-reconstruction.md): CDR as the third
  supply-chain defence.
- [`workloads/skills/README.md`](../workloads/skills/README.md): the
  toolchain reference for the CLI path.
- [`config/orchestrator-workflows.yml`](../config/orchestrator-workflows.yml)
  `install-skill`: the workflow definition the v0.7 GUI banner will
  invoke.
- `MISSION.md` Thread D (gitignored, multi-session plan): this doc
  closes step 2 of Thread D's 5 steps.
- A4 + B5 in [`docs/specs/2026-05-20-dogfood-full-arc-findings.md`](specs/2026-05-20-dogfood-full-arc-findings.md):
  the dogfood evidence that forced this decision.
