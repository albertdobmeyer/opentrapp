import { CloudFog, RotateCw, ChevronDown, ChevronUp } from "lucide-react";
import { useState } from "react";

import type { ClassifiedError } from "@/lib/errors";

interface FriendlyRetryProps {
  classified: ClassifiedError;
  onRetry: () => void;
  /** Optional secondary action — e.g. "Skip for now" or "Use default". */
  secondary?: { label: string; action: () => void };
  /** Escalates to Level 3 ContactSupport. */
  onGetHelp?: () => void;
  /** Override for context-specific copy when the classifier's text is too generic. */
  titleOverride?: string;
  bodyOverride?: string;
}

/**
 * Level 2 of the failure cascade per spec 06. Shown after silent retry has failed
 * for known-recoverable errors (connectivity, transient). Reassuring tone, prominent
 * Try Again, no technical details by default.
 */
export default function FriendlyRetry({
  classified,
  onRetry,
  secondary,
  onGetHelp,
  titleOverride,
  bodyOverride,
}: FriendlyRetryProps) {
  const [showTechnical, setShowTechnical] = useState(false);

  return (
    <div className="mx-auto max-w-md py-12 px-4 text-center animate-slide-in">
      <div
        className="mx-auto mb-6 flex h-20 w-20 items-center justify-center rounded-full bg-warning-500/10 text-warning-400"
        aria-hidden
      >
        <CloudFog size={40} strokeWidth={1.5} />
      </div>

      <h2 className="mb-2 text-2xl font-semibold text-neutral-100">
        {titleOverride ?? classified.title}
      </h2>
      <p className="mb-8 text-sm text-neutral-400">
        {bodyOverride ?? classified.suggestedAction}
      </p>

      <div className="flex flex-wrap items-center justify-center gap-3">
        <button
          type="button"
          onClick={onRetry}
          className="btn btn-md btn-primary"
        >
          <RotateCw size={16} />
          Try again
        </button>
        {secondary && (
          <button
            type="button"
            onClick={secondary.action}
            className="btn btn-md btn-ghost"
          >
            {secondary.label}
          </button>
        )}
      </div>

      {onGetHelp && (
        <p className="mt-6 text-xs text-neutral-500">
          Still stuck?{" "}
          <button
            type="button"
            onClick={onGetHelp}
            className="text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
          >
            Get help
          </button>
        </p>
      )}

      <button
        type="button"
        onClick={() => { setShowTechnical((v) => !v); }}
        className="mt-8 inline-flex items-center gap-1 text-[11px] text-neutral-600 hover:text-neutral-400"
      >
        {showTechnical ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
        {showTechnical ? "Hide" : "Show"} technical details
      </button>
      {showTechnical && (
        <pre className="mt-3 rounded-md bg-neutral-950 p-3 text-left font-mono text-[11px] text-neutral-400 whitespace-pre-wrap break-all">
          {classified.technicalDetails}
        </pre>
      )}
    </div>
  );
}
