# 05 — Bot First-Message Tutorial

**Status:** Draft
**Parent:** [`00-architectural-reframe`](00-architectural-reframe.md)
**Sibling:** [`03-activation-flow`](03-activation-flow.md)

## What this is

Karen's first message to her bot is the conversion moment — the first time she actually believes the product works. Today, the bot has no opinion about what to say first; it sits silent until the user asks something. The reframe makes the bot's first message a warm welcome with three tappable example prompts as a Telegram inline keyboard. The tutorial happens in Telegram, where Karen lives, not in a desktop docs page.

## Where this fits in the activation flow

The activation wizard's test-message ("Hi! I'm your new assistant. I'm working.") arrives during the activation flow itself, before vault-agent is running. It's a *health-check* message, not a tutorial.

The tutorial fires when:
1. The activation flow has completed successfully
2. vault-agent has started and OpenClaw has begun polling Telegram
3. The user sends their first message to the bot (typically `/start` from the deep-link in the wizard's "Open Telegram" button)

OpenClaw's grammY long-poller picks up the user's message; the bot's prompt instructs it to recognize "first message ever" and respond with the tutorial.

## The bot's tutorial response

```
Hey! I'm your assistant. Here's what I can do — tap one to try:

  [📅  Plan my Tuesday from this list of tasks]
  [✉️  Draft an email to my landlord]
  [📄  Summarize a PDF I send you]

Or just type whatever you need help with. I'm here.
```

Three buttons, rendered as a Telegram inline keyboard. Tapping a button sends the prompt as the user's next message — same affordance as the existing Discover-page "Try this" pattern. Karen experiences the tutorial as a conversation, not docs.

### Why these three prompts

Pulled from [`app/src/content/use-cases.ts`](../../../app/src/content/use-cases.ts) (the existing curated list), filtered to `capability: "ready"` (works at any shell level). Three picks balance:
- **One general productivity** ("Plan my Tuesday")
- **One real-world communication** ("Draft an email to my landlord")
- **One file-handling demonstration** ("Summarize a PDF I send you")

The exact three are configurable in the bot's prompt. The Discover desktop page can show all 19 use cases as a richer browser; the bot's tutorial is the curated welcome.

### Bot's first-message detection

The bot must distinguish "first message ever" from "any subsequent message." OpenClaw maintains session state in `~/vault/.openclaw/agents/main/sessions/`. The bot's prompt logic:

```
On every incoming user message:
  IF no session-history exists for this chat_id (no prior bot responses):
    Reply with the welcome tutorial (above)
  ELSE:
    Process the user's message normally
```

This is a prompt-level instruction, not a code change in OpenClaw itself. The new section in CONSTRAINTS.md (below) tells the bot how to behave.

## CONSTRAINTS.md addition

In [`components/openclaw-vault/scripts/entrypoint.sh`](../../../components/openclaw-vault/scripts/entrypoint.sh) lines 96-161, the CONSTRAINTS heredoc is written to `/home/vault/.openclaw/workspace/CONSTRAINTS.md` on first run. Today it has 7 sections; the new "WHEN A NEW USER FIRST MESSAGES YOU" section slots between "WHEN THE USER ASKS YOU TO FIND A SKILL" (lines 135-145, added in PR #43) and "How To Talk About This With The User" (line 147+).

Insert at line 146 (after the skill-search section, before the vocabulary guidance):

```markdown
## When A New User First Messages You

When the user sends you a message and you have no prior conversation history with them on this chat (no past assistant responses in your session), this is their very first interaction with you.

Respond with EXACTLY this welcome message and inline keyboard, formatted as a single message:

> Hey! I'm your assistant. Here's what I can do — tap one to try:
>
>   [📅 Plan my Tuesday from this list of tasks]
>   [✉️ Draft an email to my landlord]
>   [📄 Summarize a PDF I send you]
>
> Or just type whatever you need help with. I'm here.

The three options are inline-keyboard buttons. Each button's text is exactly as shown; tapping a button sends that text as the user's next message. Use Telegram's `reply_markup.inline_keyboard` field. Use `callback_data` matching the button text so your handler treats it identically to a typed message.

Do NOT send this welcome on subsequent messages. Detect "first message" by checking your session log for any prior assistant turn with this user; if none, this is the first.

If the user's first message contains a request (not a /start), still send the welcome first, then process their request after.
```

## Submodule PR (openclaw-vault)

This is a submodule change, not a parent-repo change. Per [`CLAUDE.md`](../../../CLAUDE.md) §8 ("submodule discipline"):

1. Branch in `components/openclaw-vault/`
2. Edit `scripts/entrypoint.sh` (insert the CONSTRAINTS section above)
3. Commit + push to openclaw-vault's GitHub remote
4. Open submodule PR; review focuses on whether the new section conflicts with existing constraints (it shouldn't — it's purely additive behavior)
5. Merge submodule PR
6. Bump submodule reference in parent repo: `cd /home/albertd/Repositories/lobster-trapp && git add components/openclaw-vault && git commit -m "Update openclaw-vault submodule reference"`
7. Open parent PR

The parent PR is small (one submodule SHA bump); the substance is in the submodule PR.

## Telegram inline keyboard mechanics

Telegram inline keyboards are JSON objects passed as `reply_markup` on `sendMessage`. Shape:

```json
{
  "inline_keyboard": [
    [{"text": "📅 Plan my Tuesday from this list of tasks", "callback_data": "tutorial:plan_tuesday"}],
    [{"text": "✉️ Draft an email to my landlord", "callback_data": "tutorial:email_landlord"}],
    [{"text": "📄 Summarize a PDF I send you", "callback_data": "tutorial:summarize_pdf"}]
  ]
}
```

When a button is tapped, Telegram sends a `callback_query` with the matching `callback_data`. OpenClaw's grammY handler maps `callback_data` starting with `tutorial:` to the corresponding prompt and processes it as if the user had typed it.

> **Implementation note:** OpenClaw's grammY-based bot already supports `callback_query` handling — confirmed via the `extensions/telegram` source documented in [`components/openclaw-vault/docs/openclaw-internals.md`](../../../components/openclaw-vault/docs/openclaw-internals.md). The CONSTRAINTS.md instructions above are sufficient; no OpenClaw code changes needed.

## Disambiguation: tutorial vs. wizard test message

The wizard's test message during activation ("Hi! I'm your new assistant. I'm working.") and the bot's first-message tutorial are **different messages from different senders**:

| Aspect | Wizard test message | First-message tutorial |
|---|---|---|
| Sender | The wizard (host process, direct Telegram API call) | OpenClaw inside vault-agent |
| When | During activation, before vault-agent runs | After vault-agent runs, on first user message |
| Purpose | Health check — proves token works, message reaches phone | Conversion — invites user to try the bot |
| Content | "Hi! I'm your new assistant. I'm working." | Three-option inline keyboard welcome |
| Trigger | Wizard polling sees user's /start | OpenClaw sees user's first message after activation |

Both can fire during the same activation event. Sequence:
1. Wizard polls, user sends /start, wizard sends test message ("I'm working.")
2. Wizard confirms by offset, releases polling
3. Activation commits, vault-agent starts, OpenClaw begins polling
4. User responds (or types something new); OpenClaw's first-message detection triggers the tutorial
5. From here on, normal conversation

Karen's experience is two messages back-to-back: the wizard's confirmation, then the bot's welcome. Both feel natural — the first is the "yes it's connected" moment, the second is the "here's what I can do" moment.

## Test coverage

Submodule unit tests in `components/openclaw-vault/tests/`:
- The CONSTRAINTS.md file ends up at `/home/vault/.openclaw/workspace/CONSTRAINTS.md` after entrypoint runs, with the new section present
- Read-only-config lock applied (per existing entrypoint.sh §4)

Integration test (manual, against a real test bot):
- Fresh activation → first user message → bot replies with the welcome and three buttons
- Tap a button → bot processes the prompt as if typed
- Send a second message → bot does NOT repeat the welcome
- Restart vault-agent → next message resumes normally without re-welcome (session history persists in `vault-data` volume)

E2E coverage of "first message triggers welcome" is hard to fully automate (requires a live Telegram bot). Mark as manual dogfood scenario in [`tests/dogfood/CHECKLIST.md`](../../../tests/dogfood/CHECKLIST.md) under a new Tier-A6 section.

## Out of scope

- **Adaptive welcome** based on user characteristics (work account vs personal, time of day, etc.) — single canonical welcome for v0.4
- **A/B testing different welcome copies** — no telemetry to support this in v0.4
- **Re-prompting the welcome after long inactivity** — once welcomed, always welcomed
- **Localizing the welcome** — English only for v0.4; i18n is a v1.0 conversation
- **Voice / video / sticker variants** of the welcome — text + inline keyboard only
- **Surfacing all 19 use-cases.ts entries in the bot** — the desktop Discover page is the richer surface; the bot's welcome is curated to three
