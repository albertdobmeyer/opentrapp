import { act, renderHook, waitFor } from "@testing-library/react";

import { getAssistantStatus } from "@/lib/tauri";

import { useAlerts } from "./useAlerts";

import type { AssistantStatusSnapshot, BackendAlert } from "@/lib/tauri";

const { listeners } = vi.hoisted(() => ({
  listeners: new Map<string, (e: unknown) => void>(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((name: string, cb: (e: unknown) => void) => {
    listeners.set(name, cb);
    return Promise.resolve(() => listeners.delete(name));
  }),
}));

vi.mock("@/lib/tauri", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/tauri")>()),
  getAssistantStatus: vi.fn(),
}));

const mockGetStatus = vi.mocked(getAssistantStatus);

const mkAlert = (over: Partial<BackendAlert>): BackendAlert => ({
  id: "a1",
  severity: "warning",
  title: "Title",
  body: null,
  cta_label: null,
  cta_to: null,
  dismissable: true,
  suppress_during_wizard: false,
  ...over,
});

const snap = (alerts: BackendAlert[]): AssistantStatusSnapshot => ({
  status: "ok",
  alerts,
  last_checked_unix_ms: 1,
  bootstrap_failure: null,
});

function fire(name: string, payload: unknown) {
  const cb = listeners.get(name);
  if (!cb) throw new Error(`no listener registered for ${name}`);
  act(() => {
    cb({ payload, event: name, id: 1 });
  });
}

beforeEach(() => {
  listeners.clear();
  mockGetStatus.mockReset();
});

describe("useAlerts", () => {
  test("initial load: filters suppress-during-wizard and maps to frontend alerts", async () => {
    mockGetStatus.mockResolvedValue(
      snap([
        mkAlert({ id: "show", title: "Visible", body: "details", cta_label: "Fix it", cta_to: "/x" }),
        mkAlert({ id: "hide", title: "Hidden", suppress_during_wizard: true }),
      ]),
    );
    const { result } = renderHook(() => useAlerts());
    await waitFor(() => { expect(result.current.alerts.length).toBe(1); });

    expect(result.current.alerts[0]).toMatchObject({
      id: "show",
      title: "Visible",
      body: "details",
      severity: "warning",
      cta: { label: "Fix it", to: "/x" },
      dismissable: true,
    });
  });

  test("a null body / missing cta map to undefined (not null)", async () => {
    mockGetStatus.mockResolvedValue(snap([mkAlert({ id: "bare" })]));
    const { result } = renderHook(() => useAlerts());
    await waitFor(() => { expect(result.current.alerts.length).toBe(1); });
    const a = result.current.alerts[0];
    expect(a.body).toBeUndefined();
    expect(a.cta).toBeUndefined();
  });

  test("an 'assistant-status-changed' event replaces the alert set", async () => {
    mockGetStatus.mockResolvedValue(snap([mkAlert({ id: "old" })]));
    const { result } = renderHook(() => useAlerts());
    await waitFor(() => { expect(result.current.alerts[0]?.id).toBe("old"); });

    fire("assistant-status-changed", snap([mkAlert({ id: "fresh", title: "Fresh" })]));
    await waitFor(() => { expect(result.current.alerts[0]?.id).toBe("fresh"); });
  });

  test("dismiss() removes the alert", async () => {
    mockGetStatus.mockResolvedValue(snap([mkAlert({ id: "go-away" })]));
    const { result } = renderHook(() => useAlerts());
    await waitFor(() => { expect(result.current.alerts.length).toBe(1); });

    act(() => {
      result.current.dismiss("go-away");
    });
    await waitFor(() => { expect(result.current.alerts.length).toBe(0); });
  });

  test("getAssistantStatus failure (browser mode) → no alerts, no throw", async () => {
    mockGetStatus.mockRejectedValue(new Error("IPC unavailable"));
    const { result } = renderHook(() => useAlerts());
    await waitFor(() => { expect(listeners.has("assistant-status-changed")).toBe(true); });
    expect(result.current.alerts).toEqual([]);
  });
});
