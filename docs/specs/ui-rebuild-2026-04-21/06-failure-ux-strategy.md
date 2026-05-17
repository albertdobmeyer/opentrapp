# Failure UX Strategy

**Prerequisite reading:** `01-vision-and-personas.md`, `05-automation-strategy.md`
**Purpose:** Define the three-level failure cascade (self-heal → retry → contact-support) and the diagnostic export system. No user ever sees a stack trace.

---

## The Three Levels

Every failure in the app flows through this cascade:

```
┌─────────────────────────────────────────────┐
│  Level 1 — Self-Heal (silent)                │
│  The app retries automatically, no UI shown  │
└─────────────────┬───────────────────────────┘
                  │ if still failing
                  ▼
┌─────────────────────────────────────────────┐
│  Level 2 — Friendly Retry                    │
│  "Something isn't working — let's try again" │
│  Prominent "Try Again" button                │
│  Reassuring copy; no technical details       │
└─────────────────┬───────────────────────────┘
                  │ if still failing
                  ▼
┌─────────────────────────────────────────────┐
│  Level 3 — Contact Support                   │
│  "Still having trouble? Let's get help."     │
│  Copy diagnostics button, email link,        │
│  GitHub issue link                           │
│  Optional "Show technical details" toggle    │
└─────────────────────────────────────────────┘
```

---

## Level 1 — Self-Heal

**Visibility:** None. User sees either the normal loading state or (if retry takes >5s) a "Working on it..." message.

**Implementation:** See `05-automation-strategy.md` — auto-retry with backoff for known transient failures.

**What gets self-healed:**
- Container startup race conditions (vault-proxy timing bug)
- Network blips (single HTTP retry)
- Tauri IPC hiccups (if event listener fails, re-attach)
- File watch reconnects

**What does NOT get self-healed:**
- Invalid API keys (user must update)
- Missing Telegram token (user must add)
- Disk full
- Permission denied
- User-initiated stops

---

## Level 2 — Friendly Retry

**Shown when:** Self-heal failed ≥ 2 attempts, and the error is a known/expected class (connection, timeout, container build).

**Layout:**

```
┌─────────────────────────────────────────┐
│                                         │
│        [warm illustration:              │
│         small clouds clearing]          │
│                                         │
│     Something isn't working yet         │
│                                         │
│    This usually works on a retry.       │
│                                         │
│     [ Try Again ]  [ Skip for now ]     │
│                                         │
│     Still stuck? [ Get Help ]           │
│                                         │
└─────────────────────────────────────────┘
```

**Copy variants by context:**

| Context | Title | Body |
|---------|-------|------|
| Setup install | "Installation didn't finish" | "Let's try that again. These things are sometimes slow on the first go." |
| Assistant won't start | "Your assistant didn't start" | "Usually fixes itself on retry. Give it a moment." |
| API key validation fails | "Can't reach your AI" | "Let's check your connection and try once more." |
| Telegram not responding | "Telegram isn't responding" | "Let's check your connection and try once more." |

**Primary CTA**: "Try Again" — in primary blue, large. Retries the operation.
**Secondary CTA**: context-dependent (Skip for now, Use default, etc.)
**Tertiary (link-only)**: "Get Help" — expands to Level 3.

**No technical details visible by default.** A developer-mode user would toggle "Show technical details" via a small link below, but this is hidden in user mode.

**Timing:** If retry takes >10 seconds, show "Still trying..." sub-text. If >30 seconds, auto-escalate to Level 3.

---

## Level 3 — Contact Support

**Shown when:** Level 2 retry failed OR an unrecoverable error occurred.

**Layout:**

```
┌──────────────────────────────────────────────────┐
│                                                  │
│   [illustration: friendly support character      │
│    with a phone/email icon]                      │
│                                                  │
│       Still having trouble                       │
│                                                  │
│  We're sorry this isn't working. Here's how to   │
│  get help quickly:                               │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │ 📋 Copy diagnostic info                  │   │
│  │                                          │   │
│  │ We'll prepare a small text file with    │   │
│  │ everything our team needs to help you.  │   │
│  │ (No passwords or personal data.)         │   │
│  │                                          │   │
│  │    [ Copy to clipboard ]                 │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│  Then paste it into one of these:                │
│                                                  │
│  ✉️  [ Email support → ]                          │
│  💬 [ Post on GitHub → ]                         │
│                                                  │
│                                                  │
│  ▸ Show technical details                        │
│                                                  │
└──────────────────────────────────────────────────┘
```

**Key elements:**

1. **Copy diagnostic info button** — primary action. Generates a redacted bundle (see below) and copies to clipboard. Toast: "Copied! Now paste it in an email or GitHub issue."

2. **Email support link** — opens `mailto:` with:
   - `To: support@opentrapp.com` (or equivalent)
   - `Subject: OpenTrApp needs help [{app_version}]`
   - `Body: [Paste the copied diagnostic info here]\n\nWhat were you trying to do when this happened?`

3. **GitHub issue link** — opens `https://github.com/albertdobmeyer/opentrapp/issues/new?template=bug.md&title=...` pre-filled with app version.

4. **"Show technical details" toggle** — collapsed by default. When expanded, shows:
   - Error message (one-line)
   - Component involved
   - Timestamp
   - Link to the full diagnostic bundle file (saved to `~/.opentrapp/diagnostics/{timestamp}.txt`)

5. **"Try Again" link** at the bottom — always available as a fallback (maybe it's transient).

---

## The Diagnostic Bundle

A text file containing everything a support person needs to understand the issue, with all sensitive data redacted.

### Contents

```
=== LOGO-TRAPP DIAGNOSTICS ===
Generated: 2026-04-21T15:30:00Z
App version: 0.2.0
OS: Linux Ubuntu 24.04.2 LTS
Arch: x86_64

=== COMPONENT STATUS ===
openclaw-vault (runtime):   running        v0.1.0
clawhub-forge (toolchain):  ready          v0.1.0
moltbook-pioneer (network): placeholder    v0.0.1

=== CONTAINER STATUS ===
openclaw-vault:  Up 2 hours
vault-proxy:     Up 2 hours
vault-forge:     Up 2 hours

=== LAST 5 SECURITY AUDIT RESULTS ===
2026-04-21 15:00  PASS  24/24 checks
2026-04-21 12:00  PASS  24/24 checks
2026-04-21 09:00  PASS  24/24 checks
2026-04-20 20:00  FAIL  23/24 (check #2: proxy not ready)
2026-04-20 20:01  PASS  24/24 checks (after retry)

=== RECENT ERRORS (last 20) ===
[2026-04-21 15:29] setup.vault.setup   exit=1   message="Build failed: RUN step 3 exited with 137"
[2026-04-21 15:27] stream.start        timeout  after 30s
[...]

=== CURRENT ERROR CONTEXT ===
Screen: Setup wizard → Installing
Step: Building vault container
Attempts: 3
Last stderr (truncated, keys redacted):
    ...
    [REDACTED: API_KEY]
    ...

=== USER PREFERENCES (non-sensitive) ===
mode: user
autostart: true
refresh_interval: 10s
spending_limit: $20/mo

=== CONNECTIVITY ===
anthropic.com:  reachable (243ms)
api.telegram.org: reachable (180ms)
github.com:     reachable (92ms)

=== NOT INCLUDED (by design) ===
- API keys
- Telegram bot token
- User's workspace files
- Agent conversation history
- IP addresses
- Username
```

### Redaction rules

The bundle generator **MUST** redact:

- Any string matching `sk-ant-[a-zA-Z0-9-_]+` → `[REDACTED_ANTHROPIC_KEY]`
- Any string matching `[0-9]+:[a-zA-Z0-9_-]{30,}` → `[REDACTED_TELEGRAM_TOKEN]`
- The contents of `components/openclaw-vault/.env` → always skipped
- IP addresses → `[REDACTED_IP]` (or keep only first octet for geographic debugging)
- Home directory paths → `~/...`
- Usernames → `[user]`
- Chat messages, agent outputs → never included

The generator should err on the side of over-redaction.

### Implementation

New Rust command in `app/src-tauri/src/commands/`:

```rust
#[tauri::command]
pub async fn generate_diagnostic_bundle() -> Result<String, String> {
    // Collect status, errors, component versions, connectivity
    // Apply redaction
    // Return formatted text
}
```

Frontend:

```tsx
async function copyDiagnostics() {
  const bundle = await invoke<string>('generate_diagnostic_bundle');
  await navigator.clipboard.writeText(bundle);
  addToast({
    type: 'success',
    title: 'Copied!',
    message: 'Now paste it in an email or GitHub issue.',
  });
}
```

**Use Tauri clipboard plugin** if native clipboard isn't sufficient.

---

## Error Classification

Build `app/src/lib/errors.ts` (existing file, expand it) with a taxonomy:

```ts
export type ErrorCategory =
  | 'transient'       // retryable; usually resolves
  | 'connectivity'    // offline / network issue
  | 'authentication'  // API key / token issue
  | 'configuration'   // missing / malformed config
  | 'permissions'    // OS permission denied
  | 'resource'        // disk full, memory low
  | 'user-input'      // invalid user input
  | 'platform'        // OS-specific issue
  | 'unknown';        // unclassified

export interface ClassifiedError {
  category: ErrorCategory;
  userMessage: string;     // friendly, non-technical
  suggestedAction: string; // what user can do
  technicalDetails: string; // raw error, for devs
  retryable: boolean;
}

export function classifyError(err: unknown): ClassifiedError {
  // Match on known patterns:
  // - "ECONNREFUSED", "ETIMEDOUT" → connectivity
  // - "401 Unauthorized" → authentication
  // - "ENOENT", "missing file" → configuration
  // - "EACCES" → permissions
  // - Anything else → unknown
}
```

Use `ClassifiedError` everywhere:

```tsx
const classified = classifyError(err);
showFailureScreen({
  title: classified.userMessage,
  action: classified.suggestedAction,
  retry: classified.retryable,
  technical: classified.technicalDetails,
});
```

---

## Error States Per Screen

Each screen spec lists specific error states. Here's the general mapping:

| Error source | Typical category | Level shown |
|--------------|------------------|-------------|
| Tauri IPC unavailable | platform | Usually never in prod; dev-only |
| Container build fails | configuration or resource | Level 2 → 3 |
| API key invalid | authentication | Level 2 (inline) |
| Telegram token invalid | authentication | Level 2 (inline) |
| Proxy timing race | transient | Level 1 (silent retry) |
| Disk full | resource | Level 3 immediately |
| Network down | connectivity | Level 2 with offline banner |
| Unknown | unknown | Level 3 |

---

## The "Show Technical Details" Toggle

Every Level 2 and Level 3 failure screen includes a collapsible "Show technical details" link. When expanded, it reveals:

```
▼ Show technical details

  Screen: Setup wizard → Installing
  Component: openclaw-vault
  Command: setup
  Exit code: 137
  Duration: 124s
  
  Last error line:
  make: *** [Makefile:21: setup] Error 137

  Full logs saved to:
  ~/.opentrapp/diagnostics/2026-04-21_15-29.txt
```

Not a stack trace. A structured, human-readable summary that a tech-savvy user can copy-paste.

---

## Tone Guidelines

### Do

- Use words like "we", "let's", "try"
- Apologize when appropriate ("We're sorry this isn't working")
- Explain what happens next
- Offer a clear action
- Stay calm even when the system isn't

### Don't

- Blame the user ("You entered an invalid key")
- Use scary words ("fatal", "critical", "failed", "error", "exception")
- Show red color for transient issues (amber is for concern, red only for serious)
- Hide the fact that there IS an issue — just soften the framing

### Examples

| Bad | Good |
|-----|------|
| "Error: ENOENT — file not found" | "We couldn't find that file" |
| "Fatal: container crashed" | "Your assistant stopped unexpectedly. We're checking why." |
| "Invalid API key" | "Your AI key isn't working. Let's update it." |
| "Stack trace: ..." | *(hidden by default)* |
| "Retry failed 3 times" | "Let's try a different approach." |

---

## ErrorBoundary Refactor

The current `app/src/components/ErrorBoundary.tsx` shows `error.message` directly. Refactor:

```tsx
// New flow:
// 1. classifyError(error)
// 2. Render Level 2 or Level 3 component based on severity
// 3. Offer Copy Diagnostics + Contact Support
// 4. Technical details behind toggle

export class ErrorBoundary extends Component<Props, State> {
  render() {
    if (!this.state.hasError) return this.props.children;

    const classified = classifyError(this.state.error);

    return (
      <ContactSupportScreen
        title={classified.userMessage}
        suggestion={classified.suggestedAction}
        onRetry={this.handleRetry}
        onHome={this.handleGoHome}
        technicalDetails={classified.technicalDetails}
      />
    );
  }
}
```

---

## Developer Mode Override

In dev mode, Level 3 is the DEFAULT. Developers want to see raw errors. The "Show technical details" toggle defaults to EXPANDED.

Additional dev-mode-only features:
- Full stack trace visible
- Link to component log file on disk
- Component state dump
- "Copy error to clipboard" button (separate from diagnostic bundle)

---

## Testing

### Unit tests

- `classifyError()` correctly categorizes 10+ known error patterns
- `generateDiagnosticBundle()` redacts all secrets (test with fixture data containing fake keys)
- `ClassifiedError.retryable` is correct for each category

### Playwright tests

- Trigger a simulated setup failure, verify Level 2 screen appears
- Click "Try Again", verify retry
- Click "Get Help", verify Level 3 screen
- Click "Copy diagnostic info", verify clipboard contains expected content
- Verify no stack trace visible in default state

### Manual verification

- Disconnect wifi mid-setup → verify "connectivity" category
- Enter an invalid API key → verify "authentication" category
- Fill disk (or simulate) → verify "resource" category
- Break permissions → verify "permissions" category

---

## Success Criteria

- **0 stack traces** visible to user-mode users
- **100% of errors** classified into one of the 9 categories
- **Every error screen** offers a clear next action
- **Diagnostic bundle** redacts 100% of known secret patterns (verified by unit test)
- **Contact-support flow** is 3 clicks from any error screen

---

## Next

Read `user-mode/07-onboarding.md` to see how the failure strategy applies to the setup wizard.
