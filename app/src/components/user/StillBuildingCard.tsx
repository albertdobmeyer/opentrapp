import { MessageCircle, type LucideIcon } from "lucide-react";

import { useSettings } from "@/hooks/useSettings";
import { openUrl } from "@/lib/shell";

interface Props {
  icon: LucideIcon;
  title: string;
  /** One short paragraph in plain language. No spec paths, no "Coming in Phase X". */
  body: string;
}

/**
 * "Friendlier placeholder" pattern for the two surfaces that ship in Pass 7
 * (Security and Help). Tells Karen honestly that the surface isn't built yet
 * AND points her at the bot, which IS the working interface.
 */
export default function StillBuildingCard({ icon: Icon, title, body }: Props) {
  const { settings } = useSettings();

  const telegramLink =
    settings.telegramBotUrl ??
    (settings.telegramBotUsername
      ? `https://t.me/${settings.telegramBotUsername}?text=Hi`
      : "https://telegram.org");

  async function openTelegram() {
    try {
      await openUrl(telegramLink);
    } catch {
      window.open(telegramLink, "_blank", "noopener,noreferrer");
    }
  }

  return (
    <div className="mx-auto max-w-2xl px-4 py-12 animate-fade-in">
      <div className="card-hero text-center">
        <div className="mx-auto mb-6 flex h-20 w-20 items-center justify-center rounded-2xl bg-primary-500/10 text-primary-400">
          <Icon size={40} strokeWidth={1.5} />
        </div>
        <h1 className="mb-3 text-2xl font-semibold text-neutral-100">{title}</h1>
        <p className="mx-auto mb-8 max-w-md text-sm text-neutral-400">{body}</p>
        <button
          type="button"
          onClick={openTelegram}
          className="btn btn-md btn-primary"
        >
          <MessageCircle size={16} />
          Open Telegram
        </button>
      </div>
    </div>
  );
}
