import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

import type { SubStep, SubStepId } from "@/components/wizard/install-step/utils";
import type { AppSettings } from "@/lib/settings";
import type { StreamEnd, StreamLine } from "@/lib/types";

import {
  runBuildStep,
  runCheckStep,
  runDownloadStep,
  runSafetyStep,
} from "@/components/wizard/install-step/pipeline-steps";
import { INITIAL_STEPS, prefetchTelegramUrl, sanitizeLine } from "@/components/wizard/install-step/utils";
import { classifyError, type ClassifiedError } from "@/lib/errors";
import { startStream, stopStream } from "@/lib/tauri";


export type Outcome =
  | { kind: "running" }
  | { kind: "missing-runtime" }
  | { kind: "failed"; classified: ClassifiedError; level: 2 | 3 }
  | { kind: "succeeded" };

interface UseInstallPipelineOptions {
  /** Setting updater used for telegram-url prefetch on success. */
  update: (patch: Partial<AppSettings>) => Promise<void>;
}

/**
 * Owns the four-phase install pipeline state machine: check → download →
 * build → safety. Returns the current sub-step list, the overall outcome,
 * and a `retry` function. A live `tick` value is returned so the host
 * component can re-render elapsed times without owning that timer itself.
 *
 * The pipeline runs once on mount; calling `retry()` resets state and
 * re-runs from the start. Stream listeners and any in-flight stream are
 * cleaned up on unmount. Phase implementations live in
 * `components/wizard/install-step/pipeline-steps.ts`.
 */
export function useInstallPipeline({ update }: UseInstallPipelineOptions) {
  const [steps, setSteps] = useState<SubStep[]>(INITIAL_STEPS);
  const [outcome, setOutcome] = useState<Outcome>({ kind: "running" });
  const [tick, setTick] = useState(0);
  const unlistenersRef = useRef<(() => void)[]>([]);
  const cancelledRef = useRef(false);
  const currentSubStepRef = useRef<SubStepId | null>(null);

  const updateStep = useCallback(
    (id: SubStepId, patch: Partial<SubStep>) => {
      setSteps((prev) =>
        prev.map((s) => (s.id === id ? { ...s, ...patch } : s)),
      );
    },
    [],
  );

  const appendLog = useCallback((id: SubStepId, line: string) => {
    setSteps((prev) =>
      prev.map((s) =>
        s.id === id ? { ...s, technicalLog: [...s.technicalLog, line] } : s,
      ),
    );
  }, []);

  // Live timer refresh for the running step.
  useEffect(() => {
    const interval = setInterval(() => { setTick((t) => t + 1); }, 1000);
    return () => { clearInterval(interval); };
  }, []);

  // Cleanup stream listeners on unmount.
  useEffect(() => {
    return () => {
      cancelledRef.current = true;
      for (const fn of unlistenersRef.current) fn();
    };
  }, []);

  const streamOneCommand = useCallback(
    (componentId: string, commandId: string, subStepId: SubStepId) =>
      streamCommand({ componentId, commandId, subStepId, appendLog, registerCleanup: (fn) => { unlistenersRef.current.push(fn); } }),
    [appendLog],
  );

  const runPipeline = useCallback(async () => {
    cancelledRef.current = false;
    setOutcome({ kind: "running" });
    setSteps(INITIAL_STEPS);

    const stepDeps = { appendLog, updateStep };
    try {
      currentSubStepRef.current = "check";
      const { canContinue } = await runCheckStep(stepDeps);
      if (!canContinue) {
        setOutcome({ kind: "missing-runtime" });
        return;
      }
      currentSubStepRef.current = "download";
      const postInitReport = await runDownloadStep(stepDeps);
      const componentIds = new Set(
        postInitReport.components
          .map((c) => c.component_id)
          .filter((id) => id !== "social"),
      );
      currentSubStepRef.current = "build";
      await runBuildStep(componentIds, { ...stepDeps, streamOneCommand });
      currentSubStepRef.current = "safety";
      await runSafetyStep(componentIds, stepDeps);
      void prefetchTelegramUrl(update);
      setOutcome({ kind: "succeeded" });
    } catch (error) {
      // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- closure-mutated by cleanup effect; ESLint's narrowing is unaware
      if (cancelledRef.current) return;
      const classified = classifyError(error, currentSubStepRef.current ?? undefined);
      setSteps((prev) =>
        prev.map((s) =>
          s.status === "running" ? { ...s, status: "failed" } : s,
        ),
      );
      setOutcome({
        kind: "failed",
        classified,
        level: classified.retryable ? 2 : 3,
      });
    }
  }, [appendLog, streamOneCommand, update, updateStep]);

  // Kick off pipeline on mount.
  useEffect(() => {
    void runPipeline();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Stop any in-flight stream on unmount.
  useEffect(() => {
    return () => {
      void stopStream("agent", "setup").catch(() => undefined);
      void stopStream("agent", "start").catch(() => undefined);
      void stopStream("skills", "setup").catch(() => undefined);
    };
  }, []);

  const retry = useCallback(() => {
    for (const fn of unlistenersRef.current) fn();
    unlistenersRef.current = [];
    void runPipeline();
  }, [runPipeline]);

  const escalateToLevel3 = useCallback(() => {
    setOutcome((prev) =>
      prev.kind === "failed" && prev.level === 2
        ? { ...prev, level: 3 }
        : prev,
    );
  }, []);

  return { steps, outcome, tick, retry, escalateToLevel3 };
}

interface StreamCommandArgs {
  componentId: string;
  commandId: string;
  subStepId: SubStepId;
  appendLog: (id: SubStepId, line: string) => void;
  registerCleanup: (fn: () => void) => void;
}

/**
 * Stream a single (componentId, commandId) pair to completion. Resolves on
 * `exit_code === 0`; rejects with an Error for any other code or for a
 * `startStream` failure. Listener registration happens before the Promise
 * is constructed (no async executor); a deferred-resolver pattern bridges
 * the stream-end event to the returned Promise.
 */
async function streamCommand(args: StreamCommandArgs): Promise<void> {
  const { componentId, commandId, subStepId, appendLog, registerCleanup } = args;

  let resolveFn!: () => void;
  let rejectFn!: (error: Error) => void;
  const promise = new Promise<void>((resolve, reject) => {
    resolveFn = resolve;
    rejectFn = reject;
  });

  let settled = false;
  const matches = (payload: { component_id: string; command_id: string }) =>
    payload.component_id === componentId && payload.command_id === commandId;

  const unlistenLine = await listen<StreamLine>("stream-line", (event) => {
    if (!matches(event.payload)) return;
    appendLog(subStepId, sanitizeLine(event.payload.line));
  });
  const unlistenEnd = await listen<StreamEnd>("stream-end", (event) => {
    if (!matches(event.payload)) return;
    if (settled) return;
    settled = true;
    unlistenLine();
    unlistenEnd();
    if (event.payload.exit_code === 0) {
      resolveFn();
    } else {
      rejectFn(
        new Error(
          `${componentId} ${commandId} exited with code ${String(event.payload.exit_code)}`,
        ),
      );
    }
  });
  registerCleanup(unlistenLine);
  registerCleanup(unlistenEnd);

  try {
    await startStream(componentId, commandId);
  } catch (error) {
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- closure-mutated by stream-end listener; ESLint's narrowing is unaware
    if (!settled) {
      settled = true;
      unlistenLine();
      unlistenEnd();
      rejectFn(error instanceof Error ? error : new Error(String(error)));
    }
  }

  return promise;
}
