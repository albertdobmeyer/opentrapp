import { act, renderHook, waitFor } from "@testing-library/react";


import { useHero } from "./useHero";

import type { AssistantStatus, AssistantStatusSnapshot } from "@/lib/tauri";

import { getAssistantStatus } from "@/lib/tauri";

const { listeners, seeded } = vi.hoisted(() => ({
  listeners: new Map<string, (e: unknown) => void>(),
  seeded: { value: {} },
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((name: string, cb: (e: unknown) => void) => {
    listeners.set(name, cb);
    return Promise.resolve(() => listeners.delete(name));
  }),
}));
vi.mock("@/lib/tauri", () => ({ getAssistantStatus: vi.fn() }));
// Control settings.wizardCompleted (used by the derive() branches).
vi.mock("@tauri-apps/plugin-store", () => ({
  load: vi.fn(() =>
    Promise.resolve({
      get: vi.fn(() => Promise.resolve(seeded.value)),
      set: vi.fn(() => Promise.resolve()),
      save: vi.fn(() => Promise.resolve()),
    }),
  ),
}));

const mStatus = vi.mocked(getAssistantStatus);

const snap = (status: AssistantStatus): AssistantStatusSnapshot => ({
  status,
  alerts: [],
  last_checked_unix_ms: 1,
  bootstrap_failure: null,
});

function fire(status: AssistantStatus) {
  const cb = listeners.get("assistant-status-changed");
  if (!cb) throw new Error("no status listener registered");
  act(() => {
    cb({ payload: snap(status), event: "assistant-status-changed", id: 1 });
  });
}

beforeEach(() => {
  listeners.clear();
  seeded.value = {}; // wizardCompleted defaults to false
  mStatus.mockReset();
});

describe("useHero state derivation (status truth)", () => {
  test("'ok' → running_safely; loading clears after the first fetch", async () => {
    mStatus.mockResolvedValue(snap("ok"));
    const { result } = renderHook(() => useHero());
    expect(result.current.loading).toBe(true);
    await waitFor(() => { expect(result.current.state).toBe("running_safely"); });
    expect(result.current.loading).toBe(false);
  });

  test("error_perimeter BEFORE setup reads as 'not_setup', not 'broken'", async () => {
    seeded.value = { wizardCompleted: false };
    mStatus.mockResolvedValue(snap("error_perimeter"));
    const { result } = renderHook(() => useHero());
    await waitFor(() => { expect(result.current.state).toBe("not_setup"); });
  });

  test("error_perimeter AFTER setup reads as the real 'error_perimeter'", async () => {
    seeded.value = { wizardCompleted: true };
    mStatus.mockResolvedValue(snap("error_perimeter"));
    const { result } = renderHook(() => useHero());
    await waitFor(() => { expect(result.current.state).toBe("error_perimeter"); });
  });

  test("'recovering' before ever-healthy reads as 'starting' (avoids misleading copy)", async () => {
    mStatus.mockResolvedValue(snap("recovering"));
    const { result } = renderHook(() => useHero());
    await waitFor(() => { expect(result.current.state).toBe("starting"); });
  });

  test("'recovering' AFTER having been healthy reads as 'recovering'", async () => {
    mStatus.mockResolvedValue(snap("ok"));
    const { result } = renderHook(() => useHero());
    await waitFor(() => { expect(result.current.state).toBe("running_safely"); });
    fire("recovering");
    await waitFor(() => { expect(result.current.state).toBe("recovering"); });
  });

  test("passthrough states map 1:1", async () => {
    for (const s of ["dormant", "paused_by_user", "error_key"] as const) {
      mStatus.mockResolvedValue(snap(s));
      const { result, unmount } = renderHook(() => useHero());
      await waitFor(() => { expect(result.current.state).toBe(s); });
      unmount();
    }
  });

  test("IPC failure (browser mode) keeps the empty 'installing' snapshot, stops loading", async () => {
    mStatus.mockRejectedValue(new Error("no ipc"));
    const { result } = renderHook(() => useHero());
    await waitFor(() => { expect(result.current.loading).toBe(false); });
    expect(result.current.state).toBe("installing");
  });
});
