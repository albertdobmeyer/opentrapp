# User Mode: Help & Support

**Prerequisite reading:** `01-vision-and-personas.md`, `02-design-system.md`, `04-visual-assets-plan.md`, `06-failure-ux-strategy.md`
**Screen:** `/help`
**Rubric target:** 9+/10 across all principles

---

## Purpose

Give Karen a place to:
1. Find answers to common questions without asking anyone
2. Get unstuck when something breaks (without calling her nephew)
3. Contact support with pre-packaged diagnostic info

FAQ is the primary surface. Contact is the escape hatch.

---

## User Story

> As Karen, when something confuses me or breaks, I want a page with pictures and simple answers. If that doesn't help, I want a way to get help without knowing technical stuff or writing long emails.

---

## Layout

```
┌──────────────────────────────────────────────────────────────┐
│  Help                                                        │
│                                                              │
│  What can we help you with?                                  │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                   │
│  │  [🏁]    │  │  [🔑]    │  │  [🛡️]    │                   │
│  │ Getting  │  │ Keys &   │  │ Security │                   │
│  │ started  │  │ accounts │  │          │                   │
│  └──────────┘  └──────────┘  └──────────┘                   │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                   │
│  │  [🔧]    │  │  [🔒]    │  │  [💳]    │                   │
│  │ Trouble- │  │ Privacy  │  │ Updates  │                   │
│  │ shooting │  │          │  │ & billing│                   │
│  └──────────┘  └──────────┘  └──────────┘                   │
│                                                              │
│  ─── Can't find what you need? ───                           │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ 💬 Contact us                                          │ │
│  │                                                        │ │
│  │ We'll help you figure it out. Your message will        │ │
│  │ include technical info about your setup — but no       │ │
│  │ passwords or personal data.                            │ │
│  │                                                        │ │
│  │ [ Copy diagnostic info ]                               │ │
│  │                                                        │ │
│  │ Then paste it into one of these:                       │ │
│  │                                                        │ │
│  │ [ ✉ Email support ]  [ 💬 Post on GitHub ]             │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ─── About ───                                               │
│                                                              │
│  OpenTrApp v0.2.0                                        │
│  By Albert Dobmeyer. Built on OpenClaw, ClawHub, and        │
│  Moltbook. Illustrations by unDraw.                          │
│                                                              │
│  [ Privacy policy ] [ Open source repos ] [ License: MIT ]  │
└──────────────────────────────────────────────────────────────┘
```

---

## Sections

### 1. FAQ Category Grid

Six illustrated category cards. Clicking one opens a drill-down screen (or modal) with 4–8 FAQs specific to that category.

### Categories & FAQs

#### 🏁 Getting started
- How do I get started?
- What does OpenTrApp do?
- Do I need a special computer?
- Why does it need to run in the background?
- Can I use it offline?

#### 🔑 Keys & accounts
- How do I get an Anthropic API key?
- How do I create a Telegram bot?
- How do I change my keys later?
- Does this cost money?
- What if my key stops working?

#### 🛡️ Security
- How safe is my assistant?
- Can it read my files?
- What gets blocked and why?
- Why is a website on my blocklist?
- How do I add a trusted website?

#### 🔧 Troubleshooting
- My assistant stopped — what do I do?
- Setup keeps failing
- Telegram isn't getting responses
- The app feels slow
- I want to start over

#### 🔒 Privacy
- What data leaves my computer?
- Where are my keys stored?
- Can anyone see my conversations?
- Can I delete everything?
- Is anything sent to the cloud?

#### 💳 Updates & billing
- How do I update the app?
- Why am I seeing spending warnings?
- How do I set a spending limit?
- Where does my money go?
- What happens at my limit?

### FAQ detail view

Each FAQ answer:
- Short plain-English paragraph
- Often a screenshot or illustration (from `help-screenshots/` or `illustrations/help/`)
- Step-by-step numbered list when applicable
- Related FAQ links at the bottom

Example — "How do I create a Telegram bot?":

```
┌────────────────────────────────────────────────────────────┐
│  ← Back to Help                                            │
│                                                            │
│  How do I create a Telegram bot?                           │
│                                                            │
│  A Telegram "bot" is a special chat account you create     │
│  that your assistant uses to talk to you. Here's how:      │
│                                                            │
│  1. Open Telegram.                                         │
│     [screenshot: Telegram app on phone]                    │
│                                                            │
│  2. Search for "BotFather" (that's the name of an          │
│     official bot that creates bots).                       │
│     [screenshot: search result showing BotFather]          │
│                                                            │
│  3. Tap "Start" to talk to it.                             │
│     [screenshot: chat with BotFather]                      │
│                                                            │
│  4. Send the message "/newbot"                             │
│     [screenshot: typing /newbot]                           │
│                                                            │
│  5. Follow the prompts to pick a name and username.        │
│                                                            │
│  6. BotFather will give you a token — copy that long       │
│     string and paste it into OpenTrApp.                │
│     [screenshot: the token in the chat]                    │
│                                                            │
│  That's it! Your bot is ready to be your assistant's       │
│  front door.                                               │
│                                                            │
│  Related:                                                  │
│  → How do I change my keys later?                          │
│  → Telegram isn't getting responses                        │
└────────────────────────────────────────────────────────────┘
```

### 2. Contact Support Card

Always visible below FAQs. Matches spec `06-failure-ux-strategy.md`.

- "Copy diagnostic info" button (primary)
- "Email support" button (opens mailto:)
- "Post on GitHub" button (opens new issue template)
- Explanatory text: no passwords included

### 3. About Section

Footer with:
- App version
- Author / credits
- Attributions (unDraw, Heroicons, Lucide)
- Privacy policy link
- Open source repos link
- License

---

## Implementation Notes

### FAQ data source

Store FAQ entries as structured data in `app/src/content/faqs.ts`:

```ts
export interface FAQ {
  id: string;
  category: FAQCategory;
  question: string;
  answer: FAQAnswerBlock[];  // supports paragraphs + lists + screenshots
  related: string[];  // ids of related FAQs
}

export interface FAQAnswerBlock {
  type: 'paragraph' | 'list' | 'screenshot' | 'callout';
  content: string | string[];
  imageSrc?: string;
  altText?: string;
}
```

This makes FAQs easy to update without touching React code.

### Internationalization readiness

Keep all FAQ text in a separate file (easier to translate later). Not doing i18n now, but don't hardcode strings in JSX.

### Search (v0.3.0+)

Future: add a search box above the category grid. For v0.2.0, categories are enough.

---

## Copy Bank

```json
{
  "page.title": "Help",
  "page.subtitle": "What can we help you with?",

  "category.gettingStarted": "Getting started",
  "category.keys": "Keys & accounts",
  "category.security": "Security",
  "category.troubleshooting": "Troubleshooting",
  "category.privacy": "Privacy",
  "category.updates": "Updates & billing",

  "contact.title": "Contact us",
  "contact.description": "We'll help you figure it out. Your message will include technical info about your setup — but no passwords or personal data.",
  "contact.copyBtn": "Copy diagnostic info",
  "contact.thenPaste": "Then paste it into one of these:",
  "contact.emailBtn": "Email support",
  "contact.githubBtn": "Post on GitHub",
  "contact.copied": "Copied! Now paste it in an email or GitHub issue.",

  "about.version": "OpenTrApp v{version}",
  "about.credits": "By {author}. Built on OpenClaw, ClawHub, and Moltbook. Illustrations by unDraw.",
  "about.privacy": "Privacy policy",
  "about.repos": "Open source repos",
  "about.license": "License: MIT"
}
```

---

## Acceptance Criteria

- [ ] 6 category cards load within 100ms
- [ ] Each FAQ detail renders with illustrations/screenshots
- [ ] "Copy diagnostic info" produces a redacted bundle matching spec 06
- [ ] Email link pre-fills subject with app version
- [ ] GitHub link opens to a pre-filled issue
- [ ] No developer terminology
- [ ] Rubric score ≥ 9/10

---

## Files to Change / Create

| Action | File | Notes |
|--------|------|-------|
| Create | `app/src/pages/user/Help.tsx` | Main help page |
| Create | `app/src/pages/user/FAQDetail.tsx` | Drill-down FAQ view |
| Create | `app/src/components/user/FAQCategoryCard.tsx` | Category tile |
| Create | `app/src/components/user/ContactSupportCard.tsx` | Copy diagnostics + links |
| Create | `app/src/components/user/AboutSection.tsx` | Footer |
| Create | `app/src/content/faqs.ts` | FAQ data |
| Create | `app/src-tauri/src/commands/generate_diagnostic_bundle.rs` | Already in spec 06 |

---

## Next

Read `12-use-case-gallery.md` — discovery of what the assistant can do.
