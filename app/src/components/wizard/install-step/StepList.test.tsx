import { render, screen, fireEvent } from "@testing-library/react";

import { StepList } from "./StepList";

import type { SubStep } from "./utils";

function step(over: Record<string, unknown>): SubStep {
  return {
    id: "check",
    label: "A step",
    status: "pending",
    startedAt: null,
    durationMs: null,
    retryAttempt: 0,
    technicalLog: [],
    ...over,
  } as unknown as SubStep;
}

describe("StepList", () => {
  test("shows the running step's label in the status line", () => {
    const steps = [
      step({ id: "a", label: "Check your computer", status: "succeeded" }),
      step({ id: "b", label: "Download the AI parts", status: "running", startedAt: 1 }),
    ];
    render(<StepList steps={steps} remainingMs={120000} tick={0} succeeded={false} />);
    expect(screen.getByText(/setting up your assistant/i)).toBeInTheDocument();
    expect(screen.getByText(/download the ai parts…/i)).toBeInTheDocument();
    expect(screen.getByText(/minutes? remaining/i)).toBeInTheDocument();
  });

  test("renders the all-done line when succeeded", () => {
    render(<StepList steps={[step({ status: "succeeded" })]} remainingMs={null} tick={0} succeeded />);
    expect(screen.getByText(/all done\. taking you to the finish line/i)).toBeInTheDocument();
  });

  test("the technical-details toggle reveals the step logs", () => {
    const steps = [step({ label: "Build", status: "succeeded", technicalLog: ["compiled ok"] })];
    render(<StepList steps={steps} remainingMs={null} tick={0} succeeded={false} />);
    expect(screen.queryByText(/compiled ok/)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /show technical details/i }));
    expect(screen.getByText(/compiled ok/)).toBeInTheDocument();
  });
});
