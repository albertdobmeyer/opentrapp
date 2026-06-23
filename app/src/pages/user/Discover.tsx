import { ExternalLink, Search, Star } from "lucide-react";
import { useMemo, useState } from "react";

import {
  USE_CASES,
  USE_CASE_CATEGORIES,
  type UseCase,
  type UseCaseCapability,
  type UseCaseCategory,
} from "@/content/use-cases";
import { useSettings } from "@/hooks/useSettings";
import { openUrl } from "@/lib/shell";

type CategoryFilter = UseCaseCategory | "all";

const CAPABILITY_HINT: Record<UseCaseCapability, string> = {
  ready: "",
  needs_fetch: "Works best when your assistant can browse the web.",
  needs_calendar: "Coming when calendars are connected.",
  needs_voice: "Coming when voice is connected.",
};

export default function Discover() {
  const { settings, update } = useSettings();
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<CategoryFilter>("all");

  const favorites = settings.favoriteUseCaseIds;

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return USE_CASES.filter((uc) => {
      if (category !== "all" && uc.category !== category) return false;
      if (!q) return true;
      return (
        uc.title.toLowerCase().includes(q) ||
        uc.prompt.toLowerCase().includes(q)
      );
    });
  }, [query, category]);

  // Show favorites first, then everything else, preserving filter order.
  const ordered = useMemo(() => {
    const favSet = new Set(favorites);
    const favs = filtered.filter((uc) => favSet.has(uc.id));
    const rest = filtered.filter((uc) => !favSet.has(uc.id));
    return [...favs, ...rest];
  }, [filtered, favorites]);

  function toggleFavorite(id: string) {
    const next = favorites.includes(id)
      ? favorites.filter((x) => x !== id)
      : [...favorites, id];
    void update({ favoriteUseCaseIds: next });
  }

  async function tryPrompt(uc: UseCase) {
    const link = buildTelegramLink(
      uc.prompt,
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
    <div className="mx-auto max-w-4xl px-4 py-8 animate-fade-in">
      <h1 className="mb-2 text-2xl font-semibold text-neutral-100">
        Discover what your assistant can do
      </h1>
      <p className="mb-6 text-sm text-neutral-400">
        Tap any card and the prompt opens in Telegram, ready to send.
      </p>

      <div className="mb-4 flex flex-col gap-3 sm:flex-row sm:items-center">
        <div className="relative flex-1">
          <Search
            size={16}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500"
            aria-hidden
          />
          <input
            type="search"
            value={query}
            onChange={(e) => { setQuery(e.target.value); }}
            placeholder="Search ideas…"
            aria-label="Search ideas"
            className="input pl-9"
          />
        </div>
      </div>

      <div className="mb-6 flex flex-wrap gap-2" role="tablist" aria-label="Category filter">
        {USE_CASE_CATEGORIES.map((cat) => {
          const active = category === cat.id;
          return (
            <button
              key={cat.id}
              type="button"
              role="tab"
              aria-selected={active}
              onClick={() => { setCategory(cat.id); }}
              className={
                active
                  ? "btn btn-sm btn-primary"
                  : "btn btn-sm btn-ghost border border-neutral-700"
              }
            >
              {cat.label}
            </button>
          );
        })}
      </div>

      {ordered.length === 0 ? (
        <div className="card-raised text-center text-sm text-neutral-500">
          No ideas match that search. Try a different word.
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {ordered.map((uc) => (
            <UseCaseCard
              key={uc.id}
              useCase={uc}
              isFavorite={favorites.includes(uc.id)}
              onToggleFavorite={() => { toggleFavorite(uc.id); }}
              onTry={() => tryPrompt(uc)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface UseCaseCardProps {
  useCase: UseCase;
  isFavorite: boolean;
  onToggleFavorite: () => void;
  onTry: () => void;
}

function UseCaseCard({ useCase, isFavorite, onToggleFavorite, onTry }: UseCaseCardProps) {
  const ready = useCase.capability === "ready" || useCase.capability === "needs_fetch";
  const hint = CAPABILITY_HINT[useCase.capability];

  return (
    <div className={`card-raised flex flex-col gap-3 ${ready ? "" : "opacity-70"}`}>
      <div className="flex items-start justify-between gap-2">
        <h3 className="text-sm font-semibold text-neutral-100">{useCase.title}</h3>
        <button
          type="button"
          onClick={onToggleFavorite}
          aria-label={isFavorite ? "Remove from favorites" : "Add to favorites"}
          aria-pressed={isFavorite}
          className="rounded p-1 text-neutral-500 transition-colors hover:text-warning-400"
        >
          <Star
            size={16}
            fill={isFavorite ? "currentColor" : "none"}
            className={isFavorite ? "text-warning-400" : ""}
          />
        </button>
      </div>

      <p className="flex-1 text-xs text-neutral-400">“{useCase.prompt}”</p>

      {hint && <p className="text-[11px] italic text-neutral-500">{hint}</p>}

      <button
        type="button"
        onClick={onTry}
        className="btn btn-sm btn-ghost w-full justify-center border border-neutral-700"
      >
        Try this
        <ExternalLink size={12} />
      </button>
    </div>
  );
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
    // cached URL is shaped `https://t.me/{username}?text=Hi` — swap the suffix.
    return cachedUrl.replace(/\?text=.*$/, `?text=${encoded}`);
  }
  return "https://telegram.org";
}
