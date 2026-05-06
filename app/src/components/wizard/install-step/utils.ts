import { deriveTelegramBotUrl, readConfig } from "@/lib/tauri";
import { parseEnvKeys } from "@/lib/wizardUtils";

export type SubStepId = "check" | "download" | "build" | "safety";
export type SubStepStatus = "pending" | "running" | "succeeded" | "failed";

export interface SubStep {
  id: SubStepId;
  label: string;
  status: SubStepStatus;
  startedAt: number | null;
  durationMs: number | null;
  retryAttempt: number;
  technicalLog: string[];
}

export const INITIAL_STEPS: SubStep[] = [
  { id: "check", label: "Check your computer", status: "pending", startedAt: null, durationMs: null, retryAttempt: 0, technicalLog: [] },
  { id: "download", label: "Download the AI parts", status: "pending", startedAt: null, durationMs: null, retryAttempt: 0, technicalLog: [] },
  { id: "build", label: "Build your assistant", status: "pending", startedAt: null, durationMs: null, retryAttempt: 0, technicalLog: [] },
  { id: "safety", label: "Test safety checks", status: "pending", startedAt: null, durationMs: null, retryAttempt: 0, technicalLog: [] },
];

const STEP_ESTIMATES_MS: Record<SubStepId, number> = {
  check: 2_000,
  download: 30_000,
  build: 120_000,
  safety: 20_000,
};

/**
 * Best-effort remaining-time estimate for the running pipeline. Returns null
 * once everything is done; never returns negative values.
 */
export function estimateRemaining(steps: SubStep[]): number | null {
  let remaining = 0;
  for (const step of steps) {
    if (step.status === "succeeded") continue;
    if (step.status === "running" && step.startedAt) {
      const elapsed = Date.now() - step.startedAt;
      remaining += Math.max(0, STEP_ESTIMATES_MS[step.id] - elapsed);
    } else if (step.status === "pending") {
      remaining += STEP_ESTIMATES_MS[step.id];
    }
  }
  return remaining > 0 ? remaining : null;
}

/** Format milliseconds as `Ms` or `Mm Ss`. */
export function formatElapsed(ms: number): string {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${String(s)}s`;
  const m = Math.floor(s / 60);
  return `${String(m)}m ${String(s % 60)}s`;
}

/** Redact API keys and tokens from streamed build output before rendering. */
export function sanitizeLine(text: string): string {
  return text
    .replace(/(ANTHROPIC_API_KEY=|sk-ant-api\d{2}-)[^\s"']+/g, "$1[REDACTED]")
    .replace(/(OPENAI_API_KEY=|sk-)[\w-]{10,}/g, "$1[REDACTED]")
    .replace(/(TELEGRAM_BOT_TOKEN=)\S+/g, "$1[REDACTED]")
    .replace(/(-e\s+ANTHROPIC_API_KEY=)\S+/g, "$1[REDACTED]")
    .replace(/(-e\s+TELEGRAM_BOT_TOKEN=)\S+/g, "$1[REDACTED]");
}

export type Platform = "mac" | "linux" | "windows" | "other";

/**
 * Best-effort host platform detection. Reads `navigator.userAgent`
 * (`navigator.platform` is deprecated). Used only to choose which install
 * recipe to display when the user is missing a container runtime; an
 * incorrect guess just means a slightly less specific guide.
 */
export function detectPlatform(): Platform {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("mac")) return "mac";
  if (ua.includes("linux")) return "linux";
  if (ua.includes("win")) return "windows";
  return "other";
}

type SettingsPatch = Partial<{
  telegramBotUrl: string | null;
  telegramBotUsername: string | null;
}>;

/**
 * Best-effort prefetch of the Telegram bot's deep link before showing the
 * Ready card. A miss falls back to a generic message; never throws to the
 * caller.
 */
export async function prefetchTelegramUrl(
  update: (patch: SettingsPatch) => Promise<void>,
): Promise<void> {
  try {
    const envContent = await readConfig("openclaw-vault", ".env").catch(() => "");
    const { telegramToken } = parseEnvKeys(envContent);
    if (!telegramToken) {
      await update({ telegramBotUrl: null, telegramBotUsername: null });
      return;
    }
    const bot = await deriveTelegramBotUrl(telegramToken);
    await update({ telegramBotUrl: bot.url, telegramBotUsername: bot.username });
  } catch {
    // Silently accept failure — Ready falls back to a generic message.
    await update({ telegramBotUrl: null, telegramBotUsername: null });
  }
}
