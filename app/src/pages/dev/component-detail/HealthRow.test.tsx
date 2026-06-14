import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { runHealthProbe } from "@/lib/tauri";

import { HealthRow } from "./HealthRow";

import type { HealthProbe } from "@/lib/types";


vi.mock("@/lib/tauri", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/tauri")>()),
  runHealthProbe: vi.fn(),
}));

const mProbe = vi.mocked(runHealthProbe);

const PROBE: HealthProbe = {
  id: "ping",
  name: "Proxy reachable",
  command: "curl -s proxy",
  interval_seconds: 30,
  timeout_seconds: 5,
  parse: { kind: "exit_code" },
} as unknown as HealthProbe;

beforeEach(() => { vi.clearAllMocks(); });

describe("HealthRow", () => {
  test("renders the probe name and a Check button", () => {
    render(<HealthRow componentId="c1" probe={PROBE} />);
    expect(screen.getByText("Proxy reachable")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /check/i })).toBeInTheDocument();
  });

  test("a passing probe shows an 'ok' pill and its stdout", async () => {
    mProbe.mockResolvedValue({ exit_code: 0, stdout: "pong" } as never);
    render(<HealthRow componentId="c1" probe={PROBE} />);
    fireEvent.click(screen.getByRole("button", { name: /check/i }));

    await waitFor(() => { expect(mProbe).toHaveBeenCalledWith("c1", "curl -s proxy", 5); });
    expect(await screen.findByText("ok")).toBeInTheDocument();
    expect(screen.getByText("pong")).toBeInTheDocument();
  });

  test("a non-zero exit shows the exit-code pill", async () => {
    mProbe.mockResolvedValue({ exit_code: 2, stdout: "" } as never);
    render(<HealthRow componentId="c1" probe={PROBE} />);
    fireEvent.click(screen.getByRole("button", { name: /check/i }));
    expect(await screen.findByText(/exit 2/i)).toBeInTheDocument();
  });

  test("a probe error surfaces the message", async () => {
    mProbe.mockRejectedValue(new Error("no such probe"));
    render(<HealthRow componentId="c1" probe={PROBE} />);
    fireEvent.click(screen.getByRole("button", { name: /check/i }));
    expect(await screen.findByText(/no such probe/i)).toBeInTheDocument();
  });
});
