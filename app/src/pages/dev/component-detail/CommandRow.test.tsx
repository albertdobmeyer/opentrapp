import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { runCommand, startStream, stopStream } from "@/lib/tauri";

import { CommandRow } from "./CommandRow";

import type { Command, CommandResult } from "@/lib/types";

vi.mock("@/lib/tauri", () => ({
  runCommand: vi.fn(),
  startStream: vi.fn(),
  stopStream: vi.fn(),
}));

const mRun = vi.mocked(runCommand);
const mStart = vi.mocked(startStream);
const mStop = vi.mocked(stopStream);

const cmd = (over: Partial<Command>): Command => ({
  id: "setup",
  name: "Set up",
  group: "lifecycle",
  type: "action",
  danger: "safe",
  command: "make setup",
  args: [],
  available_when: [],
  sort_order: 0,
  tier: "user",
  timeout_seconds: 60,
  ...over,
});

const okResult: CommandResult = { stdout: "done", stderr: "", exit_code: 0, duration_ms: 5 };

beforeEach(() => vi.clearAllMocks());

describe("CommandRow", () => {
  test("renders the command name and danger pill", () => {
    render(<CommandRow componentId="agent" command={cmd({ name: "Restart", danger: "caution" })} />);
    expect(screen.getByText("Restart")).toBeInTheDocument();
    expect(screen.getByText("caution")).toBeInTheDocument();
  });

  test("expand reveals the underlying shell command", () => {
    render(<CommandRow componentId="agent" command={cmd({ command: "podman restart x" })} />);
    expect(screen.queryByText("podman restart x")).not.toBeInTheDocument();
    fireEvent.click(screen.getByText("Set up"));
    expect(screen.getByText("podman restart x")).toBeInTheDocument();
  });

  test("Run invokes runCommand and shows the result panel", async () => {
    mRun.mockResolvedValue(okResult);
    render(<CommandRow componentId="agent" command={cmd({ id: "start" })} />);
    fireEvent.click(screen.getByRole("button", { name: /run/i }));
    await waitFor(() => expect(screen.getByText("exit 0")).toBeInTheDocument());
    expect(mRun).toHaveBeenCalledWith("agent", "start", {});
    expect(screen.getByText("done")).toBeInTheDocument();
  });

  test("Run failure surfaces the error message", async () => {
    mRun.mockRejectedValue(new Error("boom failed"));
    render(<CommandRow componentId="agent" command={cmd({})} />);
    fireEvent.click(screen.getByRole("button", { name: /run/i }));
    await waitFor(() => expect(screen.getByText("boom failed")).toBeInTheDocument());
  });

  test("stream command toggles start → stop", async () => {
    mStart.mockResolvedValue(undefined);
    mStop.mockResolvedValue(undefined);
    render(<CommandRow componentId="agent" command={cmd({ id: "logs", type: "stream" })} />);

    fireEvent.click(screen.getByRole("button", { name: /stream/i }));
    await waitFor(() => { expect(mStart).toHaveBeenCalledWith("agent", "logs", {}); });
    await waitFor(() => expect(screen.getByRole("button", { name: /stop/i })).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: /stop/i }));
    await waitFor(() => { expect(mStop).toHaveBeenCalledWith("agent", "logs"); });
  });
});
