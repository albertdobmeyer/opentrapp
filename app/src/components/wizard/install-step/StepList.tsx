import { Check, ChevronDown, ChevronRight, Circle, Loader2 } from "lucide-react";
import { useState } from "react";

import { formatElapsed } from "./utils";

import type { SubStep, SubStepStatus } from "./utils";

interface StepListProps {
  steps: SubStep[];
  /** Estimated remaining ms across the full pipeline, or null if unknown / done. */
  remainingMs: number | null;
  /** Bumped each second by the parent so elapsed times re-render live. */
  tick: number;
  /** True after `outcome.kind === "succeeded"`. */
  succeeded: boolean;
}

export function StepList({ steps, remainingMs, tick, succeeded }: StepListProps) {
  const [showDetails, setShowDetails] = useState(false);
  const runningStep = steps.find((s) => s.status === "running");

  return (
    <div className="animate-fade-in mx-auto max-w-xl px-4 py-6">
      <div className="mb-8 flex flex-col items-center text-center">
        <PulsingRings className="mb-6" />
        <h1 className="mb-2 text-2xl font-semibold text-neutral-100">
          Setting up your assistant
        </h1>
        <p className="text-sm text-neutral-400" aria-live="polite">
          {pipelineStatusLine(succeeded, runningStep)}
        </p>
        {remainingMs !== null && !succeeded && (
          <p className="mt-1 text-xs text-neutral-500">
            About {String(Math.max(1, Math.round(remainingMs / 60000)))} minute
            {remainingMs >= 120000 ? "s" : ""} remaining
          </p>
        )}
      </div>

      <ul className="card-raised mb-6 space-y-3">
        {steps.map((step) => (
          <StepRow key={step.id} step={step} tick={tick} />
        ))}
      </ul>

      <div className="text-center">
        <button
          type="button"
          onClick={() => { setShowDetails((v) => !v); }}
          className="inline-flex items-center gap-1 text-xs text-neutral-500 hover:text-neutral-300"
          aria-expanded={showDetails}
        >
          {showDetails ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          {showDetails ? "Hide" : "Show"} technical details
        </button>
      </div>

      {showDetails && (
        <pre className="mt-4 max-h-64 overflow-y-auto rounded-md bg-neutral-950 p-3 font-mono text-[11px] leading-relaxed text-neutral-400 whitespace-pre-wrap break-all">
          {steps
            .flatMap((s) =>
              s.technicalLog.length > 0
                ? [`─── ${s.label} ───`, ...s.technicalLog, ""]
                : [],
            )
            .join("\n") || "(no output yet)"}
        </pre>
      )}
    </div>
  );
}

function pipelineStatusLine(succeeded: boolean, running: SubStep | undefined): string {
  if (succeeded) return "All done. Taking you to the finish line…";
  if (running) return `${running.label}…`;
  return "This usually takes 2–3 minutes.";
}

const STEP_LABEL_CLASS: Record<SubStepStatus, string> = {
  succeeded: "text-neutral-300",
  running: "text-neutral-100",
  failed: "text-danger-400",
  pending: "text-neutral-500",
};

interface StepRowProps {
  step: SubStep;
  tick: number;
}

function StepRow({ step, tick }: StepRowProps) {
  const elapsed = step.startedAt
    ? (step.durationMs ?? Date.now() - step.startedAt)
    : null;

  return (
    <li className="flex items-center gap-3">
      <StepGlyph status={step.status} />
      <div className="flex-1">
        <p className={`text-sm ${STEP_LABEL_CLASS[step.status]}`}>{step.label}</p>
      </div>
      {step.status === "running" && elapsed !== null && (
        <span
          key={`elapsed-${String(tick)}`}
          className="text-xs tabular-nums text-neutral-500"
        >
          {formatElapsed(elapsed)}
        </span>
      )}
    </li>
  );
}

function StepGlyph({ status }: { status: SubStepStatus }) {
  switch (status) {
    case "pending":
      return <Circle size={18} className="text-neutral-700" strokeWidth={1.5} />;
    case "running":
      return <Loader2 size={18} className="animate-spin text-primary-400" />;
    case "succeeded":
      return <Check size={18} className="text-success-400" />;
    case "failed":
      return (
        <Circle
          size={18}
          strokeWidth={3}
          className="text-danger-400"
          fill="currentColor"
        />
      );
  }
}

function PulsingRings({ className }: { className?: string }) {
  return (
    <div
      className={`relative h-20 w-20 ${className ?? ""}`}
      role="img"
      aria-label="Installation in progress"
    >
      <span
        aria-hidden
        className="animate-pulse-ring absolute inset-0 rounded-full border-2 border-primary-500/40"
      />
      <span
        aria-hidden
        className="animate-pulse-ring absolute inset-3 rounded-full border-2 border-primary-500/60"
        style={{ animationDelay: "0.3s" }}
      />
      <span
        aria-hidden
        className="absolute inset-7 rounded-full bg-primary-500"
      />
    </div>
  );
}
