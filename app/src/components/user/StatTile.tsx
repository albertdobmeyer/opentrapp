import { useNavigate } from "react-router-dom";

import type { LucideIcon } from "lucide-react";

export type TileTone = "neutral" | "warning" | "danger";

interface Props {
  icon: LucideIcon;
  iconTint: string;
  title: string;
  value: string;
  subline: string;
  href: string;
  tone?: TileTone;
}

const TONE_BORDER: Record<TileTone, string> = {
  neutral: "",
  warning: "border-warning-500/60",
  danger: "border-danger-500/60",
};

export default function StatTile({
  icon: Icon,
  iconTint,
  title,
  value,
  subline,
  href,
  tone = "neutral",
}: Props) {
  const navigate = useNavigate();
  const accent = TONE_BORDER[tone];

  return (
    <button
      type="button"
      onClick={() => { navigate(href); }}
      className={`card-interactive text-left ${accent}`}
      aria-label={`${title}: ${value}. Click for details.`}
    >
      <div className="mb-3 flex items-center gap-2">
        <Icon size={16} className={iconTint} />
        <span className="text-xs font-medium uppercase tracking-wider text-neutral-500">
          {title}
        </span>
      </div>
      <p className="mb-1 text-xl font-semibold text-neutral-100">{value}</p>
      <p className="text-xs text-neutral-500">{subline}</p>
    </button>
  );
}
