import { DollarSign, ExternalLink } from "lucide-react";

import { openUrl } from "@/lib/shell";

const CONSOLE_COST_URL = "https://console.anthropic.com/cost";

/**
 * Pure deep-link tile. Anthropic Console already shows spending data
 * better than we ever could — this card's job is just to point Karen
 * at it. Visually mirrors `StatTile` so it grids cleanly alongside
 * Security on Home.
 */
export default function SpendingTile() {
  return (
    <button
      type="button"
      onClick={() => {
        void openUrl(CONSOLE_COST_URL);
      }}
      className="card-interactive text-left"
      aria-label="View your spending on the Anthropic Console (opens in browser)"
    >
      <div className="mb-3 flex items-center gap-2">
        <DollarSign size={16} className="text-warning-400" />
        <span className="text-xs font-medium uppercase tracking-wider text-neutral-500">
          Spending
        </span>
      </div>
      <p className="mb-1 inline-flex items-center gap-1 text-xl font-semibold text-neutral-100">
        View on Anthropic
        <ExternalLink size={14} className="text-neutral-400" />
      </p>
      <p className="text-xs text-neutral-500">
        Anthropic shows your usage and spend in their Console.
      </p>
    </button>
  );
}
