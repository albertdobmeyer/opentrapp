import { act, render, screen } from "@testing-library/react";

import InstallStep from "./InstallStep";

import { useInstallPipeline } from "@/hooks/useInstallPipeline";


vi.mock("@/hooks/useInstallPipeline", () => ({ useInstallPipeline: vi.fn() }));
vi.mock("@/hooks/useSettings", () => ({ useSettings: () => ({ update: vi.fn() }) }));
// Stub the four render targets so we assert InstallStep's branch selection only.
vi.mock("@/components/wizard/install-step/StepList", () => ({ StepList: () => <div>step-list</div> }));
vi.mock("@/components/wizard/install-step/MissingRuntimeCard", () => ({ MissingRuntimeCard: () => <div>missing-runtime</div> }));
vi.mock("@/components/failure/ContactSupport", () => ({ default: () => <div>contact-support</div> }));
vi.mock("@/components/failure/FriendlyRetry", () => ({ default: () => <div>friendly-retry</div> }));

const retry = vi.fn();
const escalateToLevel3 = vi.fn();

function setPipeline(outcome: unknown) {
  vi.mocked(useInstallPipeline).mockReturnValue({
    steps: [], outcome, tick: 0, retry, escalateToLevel3,
  } as never);
}

beforeEach(() => { vi.clearAllMocks(); });

describe("InstallStep branch selection", () => {
  test("running shows the step list", () => {
    setPipeline({ kind: "running" });
    render(<InstallStep onComplete={vi.fn()} onBack={vi.fn()} />);
    expect(screen.getByText("step-list")).toBeInTheDocument();
  });

  test("missing-runtime shows the runtime card", () => {
    setPipeline({ kind: "missing-runtime" });
    render(<InstallStep onComplete={vi.fn()} onBack={vi.fn()} />);
    expect(screen.getByText("missing-runtime")).toBeInTheDocument();
  });

  test("a level-2 failure shows FriendlyRetry", () => {
    setPipeline({ kind: "failed", level: 2, classified: { retryable: true } });
    render(<InstallStep onComplete={vi.fn()} onBack={vi.fn()} />);
    expect(screen.getByText("friendly-retry")).toBeInTheDocument();
  });

  test("a level-3 failure shows ContactSupport", () => {
    setPipeline({ kind: "failed", level: 3, classified: { retryable: false } });
    render(<InstallStep onComplete={vi.fn()} onBack={vi.fn()} />);
    expect(screen.getByText("contact-support")).toBeInTheDocument();
  });

  test("succeeded auto-advances to onComplete after the pause", () => {
    vi.useFakeTimers();
    try {
      setPipeline({ kind: "succeeded" });
      const onComplete = vi.fn();
      render(<InstallStep onComplete={onComplete} onBack={vi.fn()} />);
      expect(screen.getByText("step-list")).toBeInTheDocument();
      act(() => { vi.advanceTimersByTime(1000); });
      expect(onComplete).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
    }
  });
});
