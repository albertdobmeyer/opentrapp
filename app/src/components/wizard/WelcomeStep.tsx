import { useEffect, useRef } from "react";

interface Props {
  onNext: () => void;
  /** Shown only when the wizard detects an already-complete install (spec 07 §Step 1). */
  canSkipToDashboard: boolean;
  onSkipToDashboard: () => void;
}

export default function WelcomeStep({
  onNext,
  canSkipToDashboard,
  onSkipToDashboard,
}: Props) {
  const primaryBtnRef = useRef<HTMLButtonElement | null>(null);

  // Spec §Step 1 Accessibility: "Button has keyboard focus on load"
  useEffect(() => {
    primaryBtnRef.current?.focus();
  }, []);

  return (
    <div className="relative mx-auto flex min-h-[70vh] max-w-xl flex-col items-center justify-center px-4 text-center">
      <div className="animate-slide-up w-full">
        <img
          src="/logo-banner.png"
          alt="OpenTrApp"
          className="mx-auto mb-10 w-full max-w-md"
        />

        <h1 className="mb-4 text-3xl font-semibold tracking-tight text-neutral-100 sm:text-4xl">
          Welcome
        </h1>
        <p className="mx-auto mb-10 max-w-md text-base leading-relaxed text-neutral-400">
          Your personal AI assistant, safe on your computer. Let’s get you set
          up — it takes about 3 minutes.
        </p>

        <button
          ref={primaryBtnRef}
          type="button"
          onClick={onNext}
          className="btn btn-xl btn-primary"
        >
          Get Started
        </button>

        {canSkipToDashboard && (
          <p className="mt-8 text-xs text-neutral-500">
            Already set up?{" "}
            <button
              type="button"
              onClick={onSkipToDashboard}
              className="text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
            >
              Skip to dashboard
            </button>
          </p>
        )}
      </div>
    </div>
  );
}
