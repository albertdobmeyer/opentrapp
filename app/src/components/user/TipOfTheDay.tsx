import { open as openUrl } from "@tauri-apps/plugin-shell";
import { Lightbulb } from "lucide-react";
import { useNavigate } from "react-router-dom";

import { USE_CASES } from "@/content/use-cases";
import { useSettings } from "@/hooks/useSettings";

/**
 * Daily rotating tip — deterministic pick from the use-case gallery
 * (day-of-year mod gallery size) so Karen sees the same tip whenever she
 * opens the app on a given day. "Try this" opens Telegram with the prompt
 * prefilled; "Explore more ideas" routes to /discover.
 */
export default function TipOfTheDay() {
  const { settings } = useSettings();
  const navigate = useNavigate();

  // Filter to use-cases that work today (ready or needs_fetch). The
  // "needs_calendar" / "needs_voice" tips would surface broken promises.
  const candidates = USE_CASES.filter(
    (uc) => uc.capability === "ready" || uc.capability === "needs_fetch",
  );
  if (candidates.length === 0) return null;
  const tip = candidates[dayOfYear() % candidates.length];

  async function tryPrompt() {
    const link = buildTelegramLink(
      tip.prompt,
      settings.telegramBotUrl,
      settings.telegramBotUsername,
    );
    try {
      await openUrl(link);
    } catch {
      window.open(link, "_blank", "noopener,noreferrer");
    }
  }

  return (
    <div className="card-raised mt-6">
      <div className="mb-2 flex items-center gap-2">
        <Lightbulb size={16} className="text-warning-400" />
        <span className="text-xs font-medium uppercase tracking-wider text-neutral-500">
          Tip of the day
        </span>
      </div>
      <p className="mb-1 text-sm font-semibold text-neutral-100">{tip.title}</p>
      <p className="mb-4 text-xs text-neutral-400">“{tip.prompt}”</p>
      <div className="flex flex-wrap items-center gap-3">
        <button
          type="button"
          onClick={tryPrompt}
          className="btn btn-sm btn-primary"
        >
          Try this
        </button>
        <button
          type="button"
          onClick={() => { void navigate("/discover"); }}
          className="text-xs text-primary-400 underline-offset-4 hover:text-primary-300 hover:underline"
        >
          Explore more ideas →
        </button>
      </div>
    </div>
  );
}

function dayOfYear(): number {
  const now = new Date();
  const start = new Date(now.getFullYear(), 0, 0);
  const diff = now.getTime() - start.getTime();
  return Math.floor(diff / 86_400_000);
}

function buildTelegramLink(
  prompt: string,
  cachedUrl: string | null,
  cachedUsername: string | null,
): string {
  const encoded = encodeURIComponent(prompt);
  if (cachedUsername) {
    return `https://t.me/${cachedUsername}?text=${encoded}`;
  }
  if (cachedUrl) {
    return cachedUrl.replace(/\?text=.*$/, `?text=${encoded}`);
  }
  return "https://telegram.org";
}
