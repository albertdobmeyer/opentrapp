import { useEffect, useState } from "react";

import { listen, type Event } from "@/lib/events";

/**
 * Step IDs emitted by the Rust bootstrap pipeline. Mirrors
 * `BootstrapStep::as_str()` in `app/src-tauri/src/lifecycle.rs`.
 */
export type BootstrapStepId =
  | "detect-runtime"
  | "install-runtime"
  | "write-env"
  | "build-images"
  | "pull-images"
  | "up-shell"
  | "verify-shell"
  | "up-agent";

interface StepStartedPayload {
  step: BootstrapStepId;
  total_steps: number;
  current: number;
  detail: string | null;
}

interface StepFailedPayload {
  cause: string;
  message: string;
}

/**
 * Plain-language labels for each step. Karen sees these — no jargon, no
 * container names. One line, sentence-case, present continuous.
 */
const STEP_LABEL: Record<BootstrapStepId, string> = {
  "detect-runtime": "Checking your computer for the secure runtime",
  "install-runtime": "Installing the secure runtime",
  "write-env": "Saving your configuration",
  "build-images": "Preparing the security components",
  "pull-images": "Verifying the security components",
  "up-shell": "Starting the safe room",
  "verify-shell": "Double-checking everything's locked down",
  "up-agent": "Bringing your assistant online",
};

export interface BootstrapProgress {
  /** Last emitted step. Null until the first event arrives. */
  step: BootstrapStepId | null;
  /** 1-indexed current step. */
  current: number;
  total: number;
  /** Plain-language label for the current step. */
  label: string | null;
  /** Optional backend-provided detail string. */
  detail: string | null;
  /** True between any started event and the next failed/cleared event. */
  active: boolean;
  /** Failure payload if the pipeline emitted a failure. */
  failed: StepFailedPayload | null;
}

const EMPTY: BootstrapProgress = {
  step: null,
  current: 0,
  total: 7,
  label: null,
  detail: null,
  active: false,
  failed: null,
};

/**
 * Subscribes to `bootstrap-step-started` and `bootstrap-step-failed` events
 * from the Rust backend and exposes the current pipeline state. Used by
 * HeroStatusCard to render a real progress indicator instead of a
 * fire-and-forget toast (Zone 1).
 */
export function useBootstrapProgress(): BootstrapProgress {
  const [progress, setProgress] = useState<BootstrapProgress>(EMPTY);

  useEffect(() => {
    let unlistenStarted: (() => void) | null = null;
    let unlistenFailed: (() => void) | null = null;

    // Named handlers (defined at the effect's top level) keep the listener
    // wiring out of a 4th nesting level under the async IIFE.
    const onStarted = (event: Event<StepStartedPayload>) => {
      setProgress({
        step: event.payload.step,
        current: event.payload.current,
        total: event.payload.total_steps,
        label: STEP_LABEL[event.payload.step],
        detail: event.payload.detail,
        active: true,
        failed: null,
      });
    };
    const onFailed = (event: Event<StepFailedPayload>) => {
      setProgress((prev) => ({ ...prev, active: false, failed: event.payload }));
    };

    void (async () => {
      unlistenStarted = await listen<StepStartedPayload>(
        "bootstrap-step-started",
        onStarted,
      );
      unlistenFailed = await listen<StepFailedPayload>(
        "bootstrap-step-failed",
        onFailed,
      );
    })();

    return () => {
      unlistenStarted?.();
      unlistenFailed?.();
    };
  }, []);

  return progress;
}
