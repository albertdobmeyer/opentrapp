export type UseCaseCategory = "everyday" | "work" | "research" | "creative";

/**
 * Capability tag (matches docs/specs/ui-rebuild-2026-04-21/user-mode/12-use-case-gallery.md):
 *
 * - "ready"          ✅ Works today at any shell level — pure reasoning, bot's
 *                       own file ops, sensitive-advice routing.
 * - "needs_fetch"    🌐 Needs the `web_fetch` / `web_search` tool. At Split
 *                       Shell the bot will gracefully redirect to a web URL.
 *                       At Soft Shell the bot can fetch directly (F11 fix
 *                       2026-04-25 made this real).
 * - "needs_calendar" 📅 Needs the `vault-calendar` sidecar (planned v0.3.0+
 *                       per docs/specs/2026-04-25-voice-and-calendar-perimeter-extension.md).
 *                       Bot will redirect to phone's reminder app for now.
 * - "needs_voice"    📞 Needs the `vault-voice` sidecar (planned v0.4.0+).
 *
 * Discover.tsx renders "ready" cards as primary "Try this" entries. Other
 * tags should be visually de-emphasised with a "Coming in v0.3 — needs
 * <capability>" tooltip so the user sees the roadmap, not a broken promise.
 */
export type UseCaseCapability = "ready" | "needs_fetch" | "needs_calendar" | "needs_voice";

export interface UseCase {
  id: string;
  title: string;
  category: UseCaseCategory;
  /** Exact prompt text sent to Telegram via deep-link. */
  prompt: string;
  /** Filename stem under app/src/assets/illustrations/use-cases/ (e.g. "plan-a-trip" → plan-a-trip.svg). */
  illustration: string;
  altText: string;
  /** What it takes for this prompt to actually work end-to-end. */
  capability: UseCaseCapability;
}

export const USE_CASE_CATEGORIES: { id: UseCaseCategory | "all"; label: string }[] = [
  { id: "all", label: "All" },
  { id: "everyday", label: "Everyday" },
  { id: "work", label: "Work" },
  { id: "research", label: "Research" },
  { id: "creative", label: "Creative" },
];

/**
 * 19 curated use cases drawn from the user-mode gallery spec
 * (docs/specs/ui-rebuild-2026-04-21/user-mode/12-use-case-gallery.md).
 *
 * Capability tally:
 *   ready          → 12  (work at the default Split Shell)
 *   needs_fetch    →  6  (work at Soft Shell; redirect to a URL at Split)
 *   needs_calendar →  1  (vault-calendar; not yet shipped)
 *   needs_voice    →  0  (vault-voice; not yet shipped)
 */
export const USE_CASES: UseCase[] = [
  {
    id: "plan-a-trip",
    title: "Plan a trip",
    category: "everyday",
    prompt: "I need help planning a weekend in Chicago with my grandkids.",
    illustration: "plan-a-trip",
    altText: "A map with a suitcase, representing trip planning",
    capability: "needs_fetch",
  },
  {
    id: "morning-briefing",
    title: "Morning briefing",
    category: "everyday",
    prompt: "Summarize today's news in 3 bullet points.",
    illustration: "morning-briefing",
    altText: "A coffee cup next to a folded newspaper",
    capability: "needs_fetch",
  },
  {
    id: "call-mom",
    title: "Remember to call mom",
    category: "everyday",
    prompt: "Remind me every Sunday at 6pm to call my mom.",
    illustration: "call-mom",
    altText: "A phone with a heart icon, representing a recurring reminder",
    capability: "needs_calendar",
  },
  {
    id: "organize-desktop",
    title: "Organize my desktop",
    category: "everyday",
    prompt: "Help me sort these downloaded files into folders.",
    illustration: "organize-desktop",
    altText: "A tidy stack of folders with labels",
    capability: "ready",
  },
  {
    id: "summarize-article",
    title: "Summarize article",
    category: "work",
    prompt: "Summarize this link in plain English: [URL]",
    illustration: "summarize-article",
    altText: "A long document compressed into a short bullet list",
    capability: "needs_fetch",
  },
  {
    id: "draft-email",
    title: "Draft an email",
    category: "work",
    prompt: "Draft a polite email declining a meeting.",
    illustration: "draft-email",
    altText: "A pencil composing on an open envelope",
    capability: "ready",
  },
  {
    id: "research-topic",
    title: "Research a topic",
    category: "research",
    prompt: "Tell me everything about the Mediterranean diet.",
    illustration: "research-topic",
    altText: "An open book with a magnifying glass",
    capability: "needs_fetch",
  },
  {
    id: "compare-products",
    title: "Compare products",
    category: "research",
    prompt: "Compare the iPhone 17 vs Pixel 11 for a senior user.",
    illustration: "compare-products",
    altText: "Two product cards side by side on a balance scale",
    capability: "needs_fetch",
  },
  {
    id: "write-poem",
    title: "Write me a poem",
    category: "creative",
    prompt: "Write a poem about my dog Coco.",
    illustration: "write-poem",
    altText: "A quill writing flowing lines on parchment",
    capability: "ready",
  },
  {
    id: "brainstorm",
    title: "Brainstorm",
    category: "creative",
    prompt: "Help me brainstorm gift ideas for my daughter's 30th.",
    illustration: "brainstorm",
    altText: "A lightbulb surrounded by smaller idea sparks",
    capability: "ready",
  },
  {
    id: "translate",
    title: "Translate",
    category: "work",
    prompt: "Translate 'where is the bathroom' into French, Spanish, and Italian.",
    illustration: "translate",
    altText: "A speech bubble showing three different languages",
    capability: "ready",
  },
  {
    id: "local-weather",
    title: "Local weather",
    category: "everyday",
    prompt: "What's the weather like in my area this weekend?",
    illustration: "local-weather",
    altText: "A sun, cloud, and rain drop together",
    capability: "needs_fetch",
  },
  {
    id: "plan-a-meal",
    title: "Plan a meal",
    category: "everyday",
    prompt: "Suggest a dinner I can make with chicken and rice.",
    illustration: "plan-a-meal",
    altText: "A chef's hat over a steaming plate",
    capability: "ready",
  },
  {
    id: "track-activity",
    title: "Track activity",
    category: "work",
    prompt: "Track what I've been asking you about this week.",
    illustration: "track-activity",
    altText: "A simple checklist with completed items",
    capability: "ready",
  },
  {
    id: "random-fact",
    title: "Random fact",
    category: "creative",
    prompt: "Tell me something interesting I don't know.",
    illustration: "random-fact",
    altText: "A speech bubble with a curiosity question mark",
    capability: "ready",
  },
  {
    id: "sensitive-health",
    title: "Health urgency advice",
    category: "everyday",
    prompt: "I have chest tightness and arm numbness. What should I do?",
    illustration: "sensitive-health",
    altText: "A heart icon with a phone receiver, representing emergency guidance",
    capability: "ready",
  },
  {
    id: "sensitive-financial",
    title: "Financial advice",
    category: "work",
    prompt: "Should I put my retirement in crypto?",
    illustration: "sensitive-financial",
    altText: "A piggy bank with a question mark above it",
    capability: "ready",
  },
  {
    id: "sensitive-legal",
    title: "Legal next steps",
    category: "work",
    prompt: "Landlord won't return my deposit. What are my next steps?",
    illustration: "sensitive-legal",
    altText: "A stylised gavel next to a folded letter",
    capability: "ready",
  },
  {
    id: "difficult-conversation",
    title: "Difficult conversation",
    category: "everyday",
    prompt: "Help me draft a firm letter to my ex about visitation.",
    illustration: "difficult-conversation",
    altText: "Two speech bubbles with a small bridge between them",
    capability: "ready",
  },
];
