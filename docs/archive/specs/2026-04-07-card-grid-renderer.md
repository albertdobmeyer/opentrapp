# Spec: Card-Grid Renderer

**Date:** 2026-04-07
**Phase:** G (Finalization Roadmap v4)
**Depends on:** Nothing
**Blocks:** Nothing (but completes the schema contract)

---

## Problem

The `OutputDisplay` enum includes `"card-grid"` as a valid display type across all three alignment layers:

- Schema: `schemas/component.schema.json` (in `output.display` enum)
- Rust: `app/src-tauri/src/orchestrator/manifest.rs` (`OutputDisplay::CardGrid`)
- TypeScript: `app/src/lib/types.ts` (`OutputDisplay = "card-grid"`)

However, `OutputRenderer.tsx:27-29` aliases `card-grid` to `ReportRenderer`:

```typescript
case "card-grid":
  // Card grid renders each line/section as a separate card
  return <ReportRenderer content={content} exitCode={result.exit_code} />;
```

No component.yml currently uses `display: card-grid`. The alias means the enum value is dead — it behaves identically to `report`.

---

## Design Goal

Card-grid should render **structured data** (key-value pairs, status items, inventory lists) as a **responsive grid of cards** — one card per logical item. This is distinct from:

- **ReportRenderer** — renders narrative/section-based text output (separated by headers)
- **TableRenderer** — renders tabular data (rows and columns)
- **BadgeRenderer** — renders a single status value

Card-grid is appropriate for commands that produce **a list of discrete items with properties** — for example, skill inventory, census results, tool status reports, or multi-item health summaries.

---

## Input Format Detection

The renderer must handle command stdout without knowing what component produced it. Detection order:

### 1. JSON Array (preferred)

If `content` parses as a JSON array of objects, each object becomes one card.

```json
[
  { "name": "git-workflows", "status": "certified", "risk": 0.12, "tests": "8/8" },
  { "name": "sql-toolkit", "status": "certified", "risk": 0.08, "tests": "6/6" }
]
```

Each object key becomes a label; each value becomes the displayed content.

### 2. JSON Object with Array Value

If `content` parses as a JSON object with a single array-valued key, extract the array.

```json
{ "skills": [ { "name": "...", ... }, { "name": "...", ... } ] }
```

### 3. Newline-Delimited JSON (JSONL)

If each non-empty line parses as a JSON object, treat as an array of objects.

### 4. Section-Header Fallback

If JSON parsing fails, fall back to section-header detection (similar to `ReportRenderer`) — split on header-like lines (`## Title`, `=== Title`, all-caps lines) and render each section as a card.

---

## Card Anatomy

Each card contains:

```
┌──────────────────────────────┐
│  Title                  Icon │  ← title: first key named "name", "id", "title", or "skill"; or section header
│  ─────────────────────────── │
│  key1: value1                │  ← body: remaining key-value pairs
│  key2: value2                │
│  key3: value3                │
│                              │
│  [status badge]              │  ← optional: if a key named "status" exists, render as colored badge
└──────────────────────────────┘
```

### Title Resolution

Try keys in this order for the card title: `name`, `id`, `title`, `skill`, `tool`. If none found, use `Card N` (1-indexed).

### Status Badge

If a key named `status` (case-insensitive) exists, render it as a colored badge:
- Green: `certified`, `passing`, `pass`, `ok`, `healthy`, `ready`, `clean`, `safe`
- Yellow: `warning`, `warn`, `caution`, `partial`, `pending`
- Red: `error`, `fail`, `failing`, `critical`, `blocked`, `malicious`
- Gray: everything else

### Value Formatting

- **Numbers:** Render as-is with monospace font
- **Booleans:** Render as green checkmark / red X
- **Strings:** Render as-is, truncate at 200 chars with ellipsis

---

## Layout

Tailwind CSS Grid, responsive:

```
mobile  (< 640px):  grid-cols-1
tablet  (sm:):      grid-cols-2
desktop (lg:):      grid-cols-3
wide    (xl:):      grid-cols-4
```

Gap: `gap-4`. Cards: `bg-gray-900 rounded-lg p-4 border border-gray-800 hover:border-gray-700 transition-colors`.

This matches the existing dark design system used by `ReportRenderer` (`.card` class, `bg-gray-900` palette).

---

## Files

| Action | Path |
|--------|------|
| **Create** | `app/src/components/renderers/CardGridRenderer.tsx` |
| **Create** | `app/src/components/renderers/CardGridRenderer.test.tsx` |
| **Modify** | `app/src/components/OutputRenderer.tsx` — replace alias with real import |

### OutputRenderer.tsx Change

```typescript
// Add import
import CardGridRenderer from "./renderers/CardGridRenderer";

// Replace case
case "card-grid":
  return <CardGridRenderer content={content} exitCode={result.exit_code} />;
```

---

## Reusable Patterns

- `stripAnsi` from `app/src/components/renderers/ansi.ts` — strip ANSI before parsing
- `ReportRenderer.tsx` section-header detection — reuse as fallback parser
- `BadgeRenderer.tsx` styling — reuse badge color logic for status fields
- Tailwind tokens: `bg-gray-900`, `border-gray-800`, `text-gray-200`, `text-gray-400` — consistent with all existing renderers

---

## Adoption

After implementation, switch at least one component command to use `display: card-grid`:

**Candidate:** `clawhub-forge` command `stats` (currently `display: table`). The stats output lists per-skill metrics (name, status, tests, risk) — a natural fit for card-grid display.

**Alternative:** `moltbook-pioneer` command `agent-census` (currently `display: report`). Census data lists agents with properties — also a natural card layout.

Changing `display` in a component.yml is non-breaking (GUI renders whatever the manifest says). The command's stdout format may need adjustment to output JSON for best results.

---

## Testing

### Unit Tests (`CardGridRenderer.test.tsx`)

1. Renders JSON array as grid of cards
2. Renders JSONL as grid of cards
3. Falls back to section-header parsing for non-JSON
4. Extracts title from `name` key
5. Renders status badge with correct color
6. Handles empty content gracefully
7. Handles malformed JSON gracefully (fallback to text)

### Manual Verification

Run the adopted command in the GUI and confirm:
- Cards display in a responsive grid
- Resizing the window changes column count
- Status badges show correct colors
- Long values truncate with ellipsis
