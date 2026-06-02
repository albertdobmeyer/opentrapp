import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { renderHook, waitFor, act } from "@testing-library/react";

import { useSentinelActivity } from "./useSentinelActivity";

import type { SentinelActivity } from "@/lib/types";


// The global test-setup mocks @tauri-apps/api/core (invoke) but not /event.
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

const activity = (rung: SentinelActivity["rung"]): SentinelActivity => ({
  rung,
  label: rung.replace("_", " "),
  since_unix_ms: 1,
});

beforeEach(() => {
  mockInvoke.mockReset();
  mockListen.mockReset();
  mockListen.mockResolvedValue(vi.fn());
});

describe("useSentinelActivity", () => {
  test("rests at 'watching' initially", () => {
    mockInvoke.mockReturnValue(new Promise(() => undefined)); // never resolves
    const { result } = renderHook(() => useSentinelActivity());
    expect(result.current.rung).toBe("watching");
  });

  test("reflects the backend's current activity on mount", async () => {
    mockInvoke.mockResolvedValue(activity("thinking"));
    const { result } = renderHook(() => useSentinelActivity());
    await waitFor(() => { expect(result.current.rung).toBe("thinking"); });
  });

  test("updates when a sentinel-activity-changed event fires", async () => {
    mockInvoke.mockResolvedValue(activity("watching"));
    let handler: ((e: { payload: SentinelActivity }) => void) | null = null;
    mockListen.mockImplementation((_event, cb) => {
      handler = cb as typeof handler;
      return Promise.resolve(vi.fn());
    });

    const { result } = renderHook(() => useSentinelActivity());
    await waitFor(() => { expect(handler).not.toBeNull(); });

    act(() => { handler?.({ payload: activity("deep_analysis") }); });
    expect(result.current.rung).toBe("deep_analysis");
  });

  test("stays 'watching' in browser mode (IPC rejects)", async () => {
    mockInvoke.mockRejectedValue(new Error("Tauri IPC not available"));
    const { result } = renderHook(() => useSentinelActivity());
    // Give the rejected promise a tick; the listener still registers.
    await waitFor(() => { expect(mockListen).toHaveBeenCalled(); });
    expect(result.current.rung).toBe("watching");
  });
});
