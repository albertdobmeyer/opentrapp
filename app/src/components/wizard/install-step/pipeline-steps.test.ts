import { checkPrerequisites, executeWorkflow, initSubmodules } from "@/lib/tauri";

import {
  runBuildStep,
  runCheckStep,
  runDownloadStep,
  runSafetyStep,
} from "./pipeline-steps";

import type { PrerequisiteReport } from "@/lib/tauri";
import type { WorkflowResult, WorkflowStatus } from "@/lib/types";

vi.mock("@/lib/tauri", () => ({
  checkPrerequisites: vi.fn(),
  executeWorkflow: vi.fn(),
  initSubmodules: vi.fn(),
}));

// withRetry has its own backoff delays (and its own tests in wizardUtils.test.ts).
// Stub it to run the op once so these step tests are fast + deterministic.
vi.mock("@/lib/wizardUtils", () => ({
  withRetry: vi.fn((op: () => Promise<unknown>) => op()),
}));

const mCheck = vi.mocked(checkPrerequisites);
const mWorkflow = vi.mocked(executeWorkflow);
const mInit = vi.mocked(initSubmodules);

const report = (over: Partial<PrerequisiteReport>): PrerequisiteReport => ({
  container_runtime: { found: true, name: "podman", version: "5" },
  submodules: [{ id: "agent", name: "Agent", cloned: true, has_manifest: true }],
  components: [],
  ...over,
});

const wf = (status: WorkflowStatus): WorkflowResult => ({
  workflow_id: "w",
  status,
  steps: [],
  duration_ms: 1,
});

const deps = () => ({
  appendLog: vi.fn(),
  updateStep: vi.fn(),
  streamOneCommand: vi.fn().mockResolvedValue(undefined),
});

beforeEach(() => {
  vi.clearAllMocks();
});

describe("runCheckStep", () => {
  test("runtime found → succeeded + canContinue", async () => {
    mCheck.mockResolvedValue(report({ container_runtime: { found: true, name: "podman", version: "5" } }));
    const d = deps();
    const out = await runCheckStep(d);
    expect(out.canContinue).toBe(true);
    expect(d.updateStep).toHaveBeenLastCalledWith("check", { status: "succeeded" });
  });

  test("runtime missing → failed + cannot continue", async () => {
    mCheck.mockResolvedValue(report({ container_runtime: { found: false, name: null, version: null } }));
    const d = deps();
    const out = await runCheckStep(d);
    expect(out.canContinue).toBe(false);
    expect(d.updateStep).toHaveBeenLastCalledWith("check", { status: "failed", durationMs: 0 });
  });
});

describe("runDownloadStep", () => {
  test("all submodules cloned + manifested → succeeded", async () => {
    mInit.mockResolvedValue("cloned 2 modules");
    mCheck.mockResolvedValue(
      report({
        submodules: [
          { id: "agent", name: "Agent", cloned: true, has_manifest: true },
          { id: "skills", name: "Skills", cloned: true, has_manifest: true },
        ],
      }),
    );
    const d = deps();
    await expect(runDownloadStep(d)).resolves.toBeDefined();
    expect(d.updateStep).toHaveBeenLastCalledWith("download", { status: "succeeded" });
  });

  test("a module missing its manifest → throws", async () => {
    mInit.mockResolvedValue("");
    mCheck.mockResolvedValue(
      report({
        submodules: [{ id: "agent", name: "Agent", cloned: true, has_manifest: false }],
      }),
    );
    await expect(runDownloadStep(deps())).rejects.toThrow(/failed to download/i);
  });
});

describe("runBuildStep", () => {
  test("builds agent (setup+start) and skills (setup) when both present", async () => {
    const d = deps();
    await runBuildStep(new Set(["agent", "skills"]), d);
    expect(d.streamOneCommand).toHaveBeenCalledWith("agent", "setup", "build");
    expect(d.streamOneCommand).toHaveBeenCalledWith("agent", "start", "build");
    expect(d.streamOneCommand).toHaveBeenCalledWith("skills", "setup", "build");
    expect(d.updateStep).toHaveBeenLastCalledWith("build", { status: "succeeded" });
  });

  test("no known components → no stream calls, still succeeds", async () => {
    const d = deps();
    await runBuildStep(new Set(["unknown"]), d);
    expect(d.streamOneCommand).not.toHaveBeenCalled();
    expect(d.updateStep).toHaveBeenLastCalledWith("build", { status: "succeeded" });
  });
});

describe("runSafetyStep", () => {
  test("all workflows complete → succeeded", async () => {
    mWorkflow.mockResolvedValue(wf("completed"));
    const d = deps();
    await runSafetyStep(new Set(["agent", "skills"]), d);
    expect(mWorkflow).toHaveBeenCalledWith("agent", "full-verify");
    expect(mWorkflow).toHaveBeenCalledWith("skills", "full-check");
    expect(d.updateStep).toHaveBeenLastCalledWith("safety", { status: "succeeded" });
  });

  test("a workflow not 'completed' → throws", async () => {
    mWorkflow.mockResolvedValue(wf("failed"));
    await expect(runSafetyStep(new Set(["agent"]), deps())).rejects.toThrow(/status: failed/i);
  });
});
