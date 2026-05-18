import {
  checkPrerequisites,
  executeWorkflow,
  initSubmodules,
} from "@/lib/tauri";
import { withRetry } from "@/lib/wizardUtils";

import type { SubStep, SubStepId } from "./utils";

type PrerequisitesReport = Awaited<ReturnType<typeof checkPrerequisites>>;

interface StepDeps {
  appendLog: (id: SubStepId, line: string) => void;
  updateStep: (id: SubStepId, patch: Partial<SubStep>) => void;
}

interface BuildStepDeps extends StepDeps {
  streamOneCommand: (componentId: string, commandId: string, subStepId: SubStepId) => Promise<void>;
}

/** Phase A: probe the host for required prerequisites. */
export async function runCheckStep(
  { appendLog, updateStep }: StepDeps,
): Promise<{ canContinue: boolean; report: PrerequisitesReport }> {
  updateStep("check", { status: "running", startedAt: Date.now() });
  const report = await checkPrerequisites();
  appendLog(
    "check",
    `Sandbox runner: ${report.container_runtime.found ? "ready" : "not found"}`,
  );
  if (!report.container_runtime.found) {
    updateStep("check", { status: "failed", durationMs: 0 });
    return { canContinue: false, report };
  }
  updateStep("check", { status: "succeeded" });
  return { canContinue: true, report };
}

/** Phase B: clone or update component submodules; verify each has a manifest. */
export async function runDownloadStep(
  { appendLog, updateStep }: StepDeps,
): Promise<PrerequisitesReport> {
  updateStep("download", { status: "running", startedAt: Date.now() });
  await withRetry(
    async () => {
      appendLog("download", "Downloading your assistant…");
      const out = await initSubmodules();
      if (out) appendLog("download", out);
    },
    2,
    (attempt) => { updateStep("download", { retryAttempt: attempt }); },
  );
  const report = await checkPrerequisites();
  if (!report.submodules.every((s) => s.cloned && s.has_manifest)) {
    throw new Error("Some assistant modules failed to download");
  }
  updateStep("download", { status: "succeeded" });
  return report;
}

/**
 * Phase C: build vault then forge (serialised — forge depends on vault
 * networking, matching the first-run-setup workflow's depends_on wiring).
 */
export async function runBuildStep(
  componentIds: Set<string>,
  { appendLog, updateStep, streamOneCommand }: BuildStepDeps,
): Promise<void> {
  updateStep("build", { status: "running", startedAt: Date.now() });

  if (componentIds.has("opencli-container")) {
    await withRetry(
      async () => {
        appendLog("build", "→ Your assistant: install");
        await streamOneCommand("opencli-container", "setup", "build");
        appendLog("build", "→ Your assistant: start");
        await streamOneCommand("opencli-container", "start", "build");
      },
      2,
      (attempt) => { updateStep("build", { retryAttempt: attempt }); },
    );
  }
  if (componentIds.has("openskill-forge")) {
    await withRetry(
      async () => {
        appendLog("build", "→ Skill scanner: install");
        await streamOneCommand("openskill-forge", "setup", "build");
      },
      2,
      (attempt) => { updateStep("build", { retryAttempt: attempt }); },
    );
  }
  updateStep("build", { status: "succeeded" });
}

/** Phase D: parallel verify+full-check workflows; both must report status "completed". */
export async function runSafetyStep(
  componentIds: Set<string>,
  { appendLog, updateStep }: StepDeps,
): Promise<void> {
  updateStep("safety", { status: "running", startedAt: Date.now() });
  await withRetry(
    async () => {
      const tasks: Promise<unknown>[] = [];
      if (componentIds.has("opencli-container")) {
        appendLog("safety", "Running assistant security audit (24 checks)…");
        tasks.push(executeWorkflow("opencli-container", "full-verify"));
      }
      if (componentIds.has("openskill-forge")) {
        appendLog("safety", "Running skill scanner pipeline check…");
        tasks.push(executeWorkflow("openskill-forge", "full-check"));
      }
      const results = await Promise.all(tasks);
      for (const r of results as { status: string }[]) {
        if (r.status !== "completed") {
          throw new Error(`Workflow ended with status: ${r.status}`);
        }
      }
    },
    2,
    (attempt) => { updateStep("safety", { retryAttempt: attempt }); },
  );
  updateStep("safety", { status: "succeeded" });
}
