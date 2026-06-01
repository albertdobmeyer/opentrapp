import { Eye, Loader2, Search } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { useSentinelActivity } from "@/hooks/useSentinelActivity";
import type { SentinelRung } from "@/lib/types";

interface RungCopy {
  label: string;
  hint: string;
  icon: LucideIcon;
  /** Tailwind classes for the badge's accent. */
  tone: string;
  spin?: boolean;
}

/**
 * Plain-language copy for each rung. No jargon (the 28-term banned-vocabulary
 * rule applies to everything rendered). The user should always be able to tell,
 * at a glance, whether the on-device safety check is resting, taking a closer
 * look, or doing heavier work that might be slow.
 */
const COPY: Record<SentinelRung, RungCopy> = {
  watching: {
    label: "Watching",
    hint: "Quietly checking activity in the background, using almost no power.",
    icon: Eye,
    tone: "border-neutral-700 bg-neutral-800/60 text-neutral-300",
  },
  thinking: {
    label: "Thinking",
    hint: "Taking a closer look at something that wasn't clear-cut. This is brief.",
    icon: Loader2,
    tone: "border-primary-500/30 bg-primary-500/10 text-primary-300",
    spin: true,
  },
  deep_analysis: {
    label: "Deep analysis",
    hint: "Looking very carefully at a tricky case you asked about — this may be slow.",
    icon: Search,
    tone: "border-amber-500/30 bg-amber-500/10 text-amber-300",
  },
};

/**
 * A small, always-present indicator of the on-device AI safety check's current
 * state. Backed by `useSentinelActivity`; in browser mode it rests at
 * "Watching". The label is the user-facing surface of Sentinel's rung.
 */
export default function SentinelActivityBadge() {
  const activity = useSentinelActivity();
  const copy = COPY[activity.rung] ?? COPY.watching;
  const Icon = copy.icon;

  return (
    <div
      title={copy.hint}
      aria-label={`On-device safety check: ${copy.label}. ${copy.hint}`}
      className={`inline-flex items-center gap-2 rounded-full border px-3 py-1 text-xs font-medium ${copy.tone}`}
    >
      <Icon size={13} strokeWidth={2} className={copy.spin ? "animate-spin" : ""} />
      <span>{copy.label}</span>
    </div>
  );
}
