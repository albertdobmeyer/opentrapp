import { invoke } from "@tauri-apps/api/core";
import { renderHook, waitFor } from "@testing-library/react";
import { createElement } from "react";


import { useManifests } from "./useManifests";


import type { DiscoveredComponent } from "@/lib/types";

import { ToastProvider } from "@/lib/ToastContext";

const wrapper = ({ children }: { children: React.ReactNode }) =>
  createElement(ToastProvider, null, children);

const mockInvoke = vi.mocked(invoke);

const fakeComponent: DiscoveredComponent = {
  manifest: {
    identity: {
      id: "test-component",
      name: "Test Component",
      version: "1.0.0",
      description: "A test component",
      role: "runtime",
    },
    commands: [],
    configs: [],
    health: [],
    workflows: [],
  },
  component_dir: "/fake/path",
};

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("useManifests", () => {
  test("starts in loading state", () => {
    mockInvoke.mockReturnValue(new Promise(() => undefined)); // never resolves
    const { result } = renderHook(() => useManifests(), { wrapper });
    expect(result.current.loading).toBe(true);
    expect(result.current.components).toEqual([]);
    expect(result.current.error).toBeNull();
  });

  test("loading → success with components", async () => {
    mockInvoke.mockResolvedValue([fakeComponent]);
    const { result } = renderHook(() => useManifests(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.components).toEqual([fakeComponent]);
    expect(result.current.error).toBeNull();
  });

  test("loading → error on failure", async () => {
    mockInvoke.mockRejectedValue(new Error("Tauri not available"));
    const { result } = renderHook(() => useManifests(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe("Tauri not available");
    expect(result.current.components).toEqual([]);
  });

  test("refresh re-fetches components", async () => {
    mockInvoke.mockResolvedValue([]);
    const { result } = renderHook(() => useManifests(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    mockInvoke.mockResolvedValue([fakeComponent]);
    void result.current.refresh();

    await waitFor(() => {
      expect(result.current.components).toEqual([fakeComponent]);
    });
  });
});
