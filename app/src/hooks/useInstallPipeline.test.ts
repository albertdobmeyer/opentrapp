import { act, renderHook, waitFor } from "@testing-library/react";

import {
  runBuildStep,
  runCheckStep,
  runDownloadStep,
  runSafetyStep,
} from "@/components/wizard/install-step/pipeline-steps";
import { classifyError } from "@/lib/errors";

import { useInstallPipeline } from "./useInstallPipeline";

import type { ClassifiedError } from "@/lib/errors";
import type { PrerequisiteReport } from "@/lib/tauri";

vi.mock("@/components/wizard/install-step/pipeline-steps", () => ({
  runCheckStep: vi.fn(),
  runDownloadStep: vi.fn(),
  runBuildStep: vi.fn(),
  runSafetyStep: vi.fn(),
}));
vi.mock("@/components/wizard/install-step/utils", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/components/wizard/install-step/utils")>()),
  prefetchTelegramUrl: vi.fn(),
}));
vi.mock("@/lib/errors", () => ({ classifyError: vi.fn() }));
vi.mock("@/lib/tauri", () => ({ startStream: vi.fn(), stopStream: vi.fn(() => Promise.resolve()) }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => undefined)),
}));

const mCheck = vi.mocked(runCheckStep);
const mDownload = vi.mocked(runDownloadStep);
const mBuild = vi.mocked(runBuildStep);
const mSafety = vi.mocked(runSafetyStep);
const mClassify = vi.mocked(classifyError);

const comp = (id: string) => ({
  component_id: id,
  component_name: id,
  needs_container_runtime: true,
  missing_config_files: [],
  check_passed: null,
});

const report = (ids: string[]): PrerequisiteReport => ({
  container_runtime: { found: true, name: "podman", version: "5" },
  submodules: [],
  components: ids.map((id) => comp(id)),
});

const ce = (retryable: boolean): ClassifiedError => ({
  category: "unknown",
  severity: "transient",
  title: "T",
  userMessage: "u",
  suggestedAction: "a",
  message: "m",
  technicalDetails: "m",
  retryable,
});

function setupHappyPath() {
  mCheck.mockResolvedValue({ canContinue: true, report: report([]) });
  mDownload.mockResolvedValue(report(["agent", "skills", "social"]));
  mBuild.mockResolvedValue(undefined);
  mSafety.mockResolvedValue(undefined);
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("useInstallPipeline (the onboarding conductor)", () => {
  test("runtime missing → outcome 'missing-runtime', later phases not run", async () => {
    mCheck.mockResolvedValue({ canContinue: false, report: report([]) });
    const { result } = renderHook(() => useInstallPipeline({ update: vi.fn() }));
    await waitFor(() => { expect(result.current.outcome.kind).toBe("missing-runtime"); });
    expect(mDownload).not.toHaveBeenCalled();
    expect(mBuild).not.toHaveBeenCalled();
  });

  test("all phases succeed → outcome 'succeeded'; 'social' is filtered out of the build set", async () => {
    setupHappyPath();
    const { result } = renderHook(() => useInstallPipeline({ update: vi.fn() }));
    await waitFor(() => { expect(result.current.outcome.kind).toBe("succeeded"); });

    const buildSet = mBuild.mock.calls[0][0];
    expect([...buildSet].sort()).toEqual(["agent", "skills"]);
    expect(buildSet.has("social")).toBe(false);
    expect(mSafety).toHaveBeenCalledTimes(1);
  });

  test("a retryable failure → outcome 'failed' at level 2", async () => {
    setupHappyPath();
    mBuild.mockRejectedValue(new Error("build blew up"));
    mClassify.mockReturnValue(ce(true));
    const { result } = renderHook(() => useInstallPipeline({ update: vi.fn() }));
    await waitFor(() => { expect(result.current.outcome.kind).toBe("failed"); });
    expect(result.current.outcome).toMatchObject({ kind: "failed", level: 2 });
  });

  test("a non-retryable failure → outcome 'failed' at level 3", async () => {
    setupHappyPath();
    mSafety.mockRejectedValue(new Error("safety failed hard"));
    mClassify.mockReturnValue(ce(false));
    const { result } = renderHook(() => useInstallPipeline({ update: vi.fn() }));
    await waitFor(() => { expect(result.current.outcome.kind).toBe("failed"); });
    expect(result.current.outcome).toMatchObject({ kind: "failed", level: 3 });
  });

  test("retry() re-runs the pipeline to success after a failure", async () => {
    setupHappyPath();
    mBuild.mockRejectedValueOnce(new Error("transient"));
    mClassify.mockReturnValue(ce(true));
    const { result } = renderHook(() => useInstallPipeline({ update: vi.fn() }));
    await waitFor(() => { expect(result.current.outcome.kind).toBe("failed"); });

    act(() => {
      result.current.retry();
    });
    await waitFor(() => { expect(result.current.outcome.kind).toBe("succeeded"); });
  });

  test("escalateToLevel3 promotes a level-2 failure to level 3", async () => {
    setupHappyPath();
    mBuild.mockRejectedValue(new Error("x"));
    mClassify.mockReturnValue(ce(true));
    const { result } = renderHook(() => useInstallPipeline({ update: vi.fn() }));
    await waitFor(() => { expect(result.current.outcome).toMatchObject({ level: 2 }); });

    act(() => {
      result.current.escalateToLevel3();
    });
    expect(result.current.outcome).toMatchObject({ kind: "failed", level: 3 });
  });
});
