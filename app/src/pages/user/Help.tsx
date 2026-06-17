import { open as openUrl } from "@tauri-apps/plugin-shell";
import {
  ExternalLink,
  KeyRound,
  LifeBuoy,
  MessageCircle,
  ShieldCheck,
  Wrench,
} from "lucide-react";

import { useSettings } from "@/hooks/useSettings";

interface Section {
  icon: typeof LifeBuoy;
  title: string;
  body: string;
  cta?: { label: string; href: string };
}

const SECTIONS: Section[] = [
  {
    icon: KeyRound,
    title: "Where are my keys stored?",
    body:
      "Your Anthropic API key and Telegram bot token are stored in a file on this computer that only your user account can read. The assistant never sees the actual key. A secure gateway adds it to each request, so even a compromised assistant can't leak it.",
  },
  {
    icon: Wrench,
    title: "Something broke. How do I fix it?",
    body:
      "Open Preferences and try \"Restart the secure environment\". If that doesn't help, re-run the setup wizard. It's safe to run again and won't lose your data, and your conversation history stays on this computer.",
    cta: { label: "Open Preferences", href: "/preferences" },
  },
  {
    icon: ShieldCheck,
    title: "How do I know it's safe?",
    body:
      "The Security page shows the five protective layers around your assistant: what's running, what's been blocked, and what's been scanned. Each layer works on its own, so even if one fails, the others still hold.",
    cta: { label: "Open Security", href: "/security" },
  },
];

/** Open an external link via the shell, falling back to a new browser tab. */
async function open(href: string) {
  try {
    await openUrl(href);
  } catch {
    window.open(href, "_blank", "noopener,noreferrer");
  }
}

export default function Help() {
  const { settings } = useSettings();

  const telegramLink =
    settings.telegramBotUrl ??
    (settings.telegramBotUsername
      ? `https://t.me/${settings.telegramBotUsername}?text=Hi`
      : null);

  return (
    <div className="mx-auto max-w-3xl px-4 py-10 animate-fade-in">
      <header className="mb-8">
        <div className="mb-3 flex h-12 w-12 items-center justify-center rounded-xl bg-primary-500/10 text-primary-400">
          <LifeBuoy size={24} strokeWidth={1.75} />
        </div>
        <h1 className="text-2xl font-semibold text-neutral-100">Help & support</h1>
        <p className="mt-2 text-sm text-neutral-400">
          The quickest way to fix something is usually to ask your assistant.
          Message it on Telegram and describe what&apos;s happening. The answers
          below cover the most common problems.
        </p>
      </header>

      <div className="grid gap-3">
        {SECTIONS.map((s) => (
          <article
            key={s.title}
            className="rounded-xl border border-neutral-800 bg-neutral-900/60 p-5"
          >
            <div className="flex items-start gap-4">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-neutral-800 text-primary-400">
                <s.icon size={20} strokeWidth={1.75} />
              </div>
              <div className="flex-1">
                <h2 className="mb-1 text-base font-semibold text-neutral-100">
                  {s.title}
                </h2>
                <p className="text-sm text-neutral-400">{s.body}</p>
                {s.cta && (
                  <a
                    href={s.cta.href}
                    className="mt-3 inline-flex items-center gap-1.5 text-sm font-medium text-primary-400 hover:text-primary-300"
                  >
                    {s.cta.label}
                    <ExternalLink size={14} />
                  </a>
                )}
              </div>
            </div>
          </article>
        ))}
      </div>

      <footer className="mt-8 rounded-xl border border-neutral-800 bg-neutral-900/40 p-5">
        <h2 className="mb-2 text-sm font-semibold text-neutral-200">
          Still stuck?
        </h2>
        <p className="mb-3 text-sm text-neutral-400">
          Two ways to get a real human or your assistant on the case:
        </p>
        <div className="flex flex-wrap gap-2">
          {telegramLink && (
            <button
              type="button"
              onClick={() => void open(telegramLink)}
              className="btn btn-sm btn-primary"
            >
              <MessageCircle size={14} />
              Ask on Telegram
            </button>
          )}
          <button
            type="button"
            onClick={() =>
              void open("https://github.com/albertdobmeyer/opentrapp/issues/new")
            }
            className="btn btn-sm btn-secondary"
          >
            <ExternalLink size={14} />
            Report a problem on GitHub
          </button>
        </div>
      </footer>
    </div>
  );
}
