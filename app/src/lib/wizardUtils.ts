/**
 * Small, pure helpers shared across the 4-step wizard. Kept separate from
 * any individual step so they can be exercised by unit tests without
 * mounting React.
 */

/**
 * Wrap a transient-failure-prone operation in retries with linear backoff.
 * Matches the pattern in `docs/specs/ui-rebuild-2026-04-21/user-mode/05-automation-strategy.md`:
 * the user sees only the friendly step label for the entire duration; retries
 * are silent.
 *
 * @param op  the idempotent operation to run
 * @param maxRetries  max retries *after* the initial attempt (default 2 → 3 total attempts)
 * @param onRetry  optional callback invoked with the attempt index when a retry kicks in
 */
export async function withRetry<T>(
  op: () => Promise<T>,
  maxRetries = 2,
  onRetry?: (attempt: number) => void,
): Promise<T> {
  let lastErr: unknown;
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await op();
    } catch (error) {
      lastErr = error;
      if (attempt === maxRetries) break;
      onRetry?.(attempt + 1);
      // Linear backoff: 2s, 4s. Exponential is overkill at maxRetries=2.
      await delay(2000 * (attempt + 1));
    }
  }
  throw lastErr;
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Extract the two keys the wizard cares about from a `.env` file's
 * contents. Returns null values for missing or placeholder entries.
 */
export function parseEnvKeys(envText: string): {
  anthropicKey: string | null;
  telegramToken: string | null;
} {
  const lines = envText.split("\n");
  let anthropicKey: string | null = null;
  let telegramToken: string | null = null;

  for (const line of lines) {
    const match = /^([A-Z_]+)=(.*)$/.exec(line);
    if (!match) continue;
    const [, key, rawValue] = match;
    const value = rawValue.replace(/^["']|["']$/g, "").trim();
    if (!value || value.includes("REPLACE")) continue;
    if (key === "ANTHROPIC_API_KEY") anthropicKey = value;
    if (key === "TELEGRAM_BOT_TOKEN") telegramToken = value;
  }

  return { anthropicKey, telegramToken };
}

/** Mask a secret to `••••••••<last 4>` — shown in Change flows. */
export function maskKey(value: string): string {
  if (!value) return "";
  if (value.length <= 4) return "••••";
  return `••••••••${value.slice(-4)}`;
}

/**
 * Classify a pasted value by shape. Lenient: only reports a type when the
 * format is unambiguous; returns null when a field could reasonably hold it.
 *
 * - "anthropic" → starts with `sk-ant-`
 * - "telegram"  → `{bot_id}:{secret}` pattern per Telegram's documented format
 */
export function identifyPastedKey(value: string): "anthropic" | "telegram" | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  if (trimmed.startsWith("sk-ant-")) return "anthropic";
  if (/^\d{6,12}:[\w-]{30,}$/.test(trimmed)) return "telegram";
  return null;
}

/** Inline regex check used for green-checkmark feedback. Non-blocking. */
export function isAnthropicKeyLike(value: string): boolean {
  return value.trim().startsWith("sk-ant-") && value.trim().length > 20;
}

/** Inline regex check used for green-checkmark feedback. Non-blocking. */
export function isTelegramTokenLike(value: string): boolean {
  return /^\d{6,12}:[\w-]{10,}$/.test(value.trim());
}

/**
 * Update or insert a key=value pair in `.env` content, preserving surrounding
 * lines and trailing newline discipline. Lifted verbatim from the old
 * ConfigStep so the behavior is unchanged.
 */
export function upsertEnvVar(content: string, key: string, value: string): string {
  const regex = new RegExp(`^${key}=.*$`, "m");
  const line = `${key}=${value}`;
  if (regex.test(content)) {
    return content.replace(regex, line);
  }
  return content.trimEnd() + "\n" + line + "\n";
}
