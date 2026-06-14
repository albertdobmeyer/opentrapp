import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { executeWorkflow } from "@/lib/tauri";

import { WorkflowRow } from "./WorkflowRow";

import type { Workflow } from "@/lib/types";


vi.mock("@/lib/tauri", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/tauri")>()),
  executeWorkflow: vi.fn(),
}));

const mExec = vi.mocked(executeWorkflow);

const WF: Workflow = {
  id: "backup",
  name: "Backup data",
  description: "Snapshots the volume",
  trigger: "manual",
  danger: "safe",
  shell_requirement: "any",
  steps: [
    { id: "s1", component: "c", command: "a" },
    { id: "s2", component: "c", command: "b" },
  ],
  inputs: [],
} as unknown as Workflow;

beforeEach(() => { vi.clearAllMocks(); });

describe("WorkflowRow", () => {
  test("renders the workflow name, trigger and a Run control", () => {
    render(<WorkflowRow componentId="c1" workflow={WF} />);
    expect(screen.getByText("Backup data")).toBeInTheDocument();
    expect(screen.getByText("manual")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /run/i })).toBeInTheDocument();
  });

  test("expanding reveals the description and step count", () => {
    render(<WorkflowRow componentId="c1" workflow={WF} />);
    fireEvent.click(screen.getByText("Backup data"));
    expect(screen.getByText(/snapshots the volume/i)).toBeInTheDocument();
    expect(screen.getByText(/2 steps/i)).toBeInTheDocument();
  });

  test("Run executes the workflow and summarises the result", async () => {
    mExec.mockResolvedValue({
      workflow_id: "backup",
      status: "completed",
      steps: [{ status: "passed" }, { status: "passed" }],
      duration_ms: 42,
    } as never);
    render(<WorkflowRow componentId="c1" workflow={WF} />);
    fireEvent.click(screen.getByRole("button", { name: /run/i }));

    await waitFor(() => { expect(mExec).toHaveBeenCalledWith("c1", "backup", {}); });
    expect(await screen.findByText(/completed · 2\/2 steps · 42 ms/i)).toBeInTheDocument();
  });

  test("a failed run surfaces the error message", async () => {
    mExec.mockRejectedValue(new Error("perimeter down"));
    render(<WorkflowRow componentId="c1" workflow={WF} />);
    fireEvent.click(screen.getByRole("button", { name: /run/i }));
    expect(await screen.findByText(/perimeter down/i)).toBeInTheDocument();
  });
});
