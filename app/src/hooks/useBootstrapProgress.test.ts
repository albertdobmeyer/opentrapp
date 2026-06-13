import { act, renderHook, waitFor } from "@testing-library/react";

import { useBootstrapProgress } from "./useBootstrapProgress";

const { listeners } = vi.hoisted(() => ({
  listeners: new Map<string, (e: unknown) => void>(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((name: string, cb: (e: unknown) => void) => {
    listeners.set(name, cb);
    return Promise.resolve(() => listeners.delete(name));
  }),
}));

function fire(name: string, payload: unknown) {
  const cb = listeners.get(name);
  if (!cb) throw new Error(`no listener registered for ${name}`);
  act(() => {
    cb({ payload, event: name, id: 1 });
  });
}

beforeEach(() => { listeners.clear(); });

describe("useBootstrapProgress", () => {
  test("starts empty", () => {
    const { result } = renderHook(() => useBootstrapProgress());
    expect(result.current).toEqual({
      step: null,
      current: 0,
      total: 7,
      label: null,
      detail: null,
      active: false,
      failed: null,
    });
  });

  test("a started event maps step → plain-language label and goes active", async () => {
    const { result } = renderHook(() => useBootstrapProgress());
    await waitFor(() => { expect(listeners.has("bootstrap-step-started")).toBe(true); });

    fire("bootstrap-step-started", {
      step: "pull-images",
      total_steps: 7,
      current: 5,
      detail: "layer 3/8",
    });

    expect(result.current.step).toBe("pull-images");
    expect(result.current.current).toBe(5);
    expect(result.current.total).toBe(7);
    expect(result.current.label).toBe("Verifying the security components");
    expect(result.current.detail).toBe("layer 3/8");
    expect(result.current.active).toBe(true);
    expect(result.current.failed).toBeNull();
  });

  test("a failed event clears active and records the failure", async () => {
    const { result } = renderHook(() => useBootstrapProgress());
    await waitFor(() => { expect(listeners.has("bootstrap-step-started")).toBe(true); });

    fire("bootstrap-step-started", { step: "up-agent", total_steps: 7, current: 7, detail: null });
    expect(result.current.active).toBe(true);

    fire("bootstrap-step-failed", { cause: "timeout", message: "agent did not come up" });
    expect(result.current.active).toBe(false);
    expect(result.current.failed).toEqual({ cause: "timeout", message: "agent did not come up" });
    // The prior step context is preserved.
    expect(result.current.step).toBe("up-agent");
  });
});
