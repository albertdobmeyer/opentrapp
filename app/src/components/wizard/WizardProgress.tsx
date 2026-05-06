import type { SetupStep } from "@/lib/settings";

const STEP_ORDER: SetupStep[] = ["welcome", "connect", "install", "ready"];
const STEP_LABELS: Record<SetupStep, string> = {
  welcome: "Welcome",
  connect: "Connect",
  install: "Install",
  ready: "Ready",
};

interface Props {
  currentStep: SetupStep;
  completedSteps: SetupStep[];
}

/**
 * Four-dot progress bar rendered above Connect / Install / Ready. Welcome
 * hides it (spec 07 §"Progress Indicator" — persistent at top of steps 2–4).
 * Filled dots are completed + current; empty dots are future.
 */
export default function WizardProgress({ currentStep, completedSteps }: Props) {
  const currentIdx = STEP_ORDER.indexOf(currentStep);
  const completedSet = new Set(completedSteps);

  return (
    <nav
      aria-label="Setup progress"
      className="mx-auto mb-8 flex max-w-md items-center justify-center gap-3"
    >
      {STEP_ORDER.map((step, idx) => {
        const isCurrent = step === currentStep;
        const isCompleted = completedSet.has(step) || idx < currentIdx;
        const isFilled = isCurrent || isCompleted;

        return (
          <div key={step} className="flex flex-1 items-center gap-3">
            <div
              className="flex flex-col items-center gap-1.5"
              aria-current={isCurrent ? "step" : undefined}
            >
              <div
                aria-label={`Step ${String(idx + 1)} of ${String(STEP_ORDER.length)}: ${STEP_LABELS[step]}${isCurrent ? " (current)" : (isCompleted ? " (done)" : "")}`}
                className={`h-2.5 w-2.5 rounded-full transition-colors duration-200 ${
                  isFilled
                    ? (isCurrent
                      ? "bg-primary-500 ring-4 ring-primary-500/20"
                      : "bg-primary-500")
                    : "bg-neutral-700"
                }`}
              />
              <span
                className={`hidden text-xs sm:block ${
                  isCurrent ? "text-neutral-100" : "text-neutral-500"
                }`}
              >
                {STEP_LABELS[step]}
              </span>
            </div>
            {idx < STEP_ORDER.length - 1 && (
              <div
                aria-hidden="true"
                className={`h-px flex-1 transition-colors duration-200 ${
                  isCompleted ? "bg-primary-500" : "bg-neutral-800"
                }`}
              />
            )}
          </div>
        );
      })}
    </nav>
  );
}
