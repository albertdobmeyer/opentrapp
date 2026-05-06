import { useCallback } from "react";

import { useSettings } from "./useSettings";

import type { SetupProgress, SetupStep } from "@/lib/settings";

/**
 * Thin wrapper around `useSettings` that isolates wizard-progress reads and
 * writes. Step components should talk to this hook, not `useSettings.update`
 * directly, so the `setupProgress` shape is maintained in one place.
 */
export function useWizardProgress() {
  const { settings, loaded, update } = useSettings();

  const progress: SetupProgress | null = settings.setupProgress;

  /**
   * Record that the wizard has advanced to `step`. Adds `step` to
   * completedSteps if not already present and sets `step` as current.
   */
  const recordStep = useCallback(
    async (step: SetupStep, opts?: { skippedKeys?: boolean }) => {
      const completedSteps = new Set<SetupStep>(progress?.completedSteps ?? []);
      completedSteps.add(step);
      const next: SetupProgress = {
        step,
        completedSteps: [...completedSteps],
        ...(opts?.skippedKeys ? { skippedKeys: true } : {}),
      };
      await update({ setupProgress: next });
    },
    [progress, update],
  );

  /**
   * Mark the wizard as complete: clear in-progress state and flip the
   * `wizardCompleted` flag so router redirects stop sending user back.
   */
  const complete = useCallback(async () => {
    await update({ wizardCompleted: true, setupProgress: null });
  }, [update]);

  /** Reset progress without completing — used by "Re-run setup" in E.2.4. */
  const resetProgress = useCallback(async () => {
    await update({ setupProgress: null });
  }, [update]);

  return { progress, loaded, recordStep, complete, resetProgress };
}
