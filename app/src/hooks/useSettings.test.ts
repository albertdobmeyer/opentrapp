import { load } from "@tauri-apps/plugin-store";
import { renderHook, act, waitFor } from "@testing-library/react";

import { useSettings } from "./useSettings";

import { DEFAULT_SETTINGS } from "@/lib/settings";



describe("useSettings", () => {
  beforeEach(async () => {
    // Clear the shared mock store between tests
    const storeInstance = await load("");
    const mockStore = storeInstance as unknown as { _store: Map<string, unknown> };
    mockStore._store.clear();
  });

  test("starts with defaults and sets loaded=true", async () => {
    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loaded).toBe(true);
    });
    expect(result.current.settings).toEqual(DEFAULT_SETTINGS);
  });

  test("merges saved settings with defaults", async () => {
    // Pre-populate the store before the hook renders
    const storeInstance = await load("");
    await storeInstance.set("app_settings", { autoRefreshInterval: 30000 });

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loaded).toBe(true);
    });
    expect(result.current.settings.autoRefreshInterval).toBe(30000);
    // Defaults should fill in missing fields
    expect(result.current.settings.wizardCompleted).toBe(false);
    expect(result.current.settings.lastViewedComponentId).toBeNull();
  });

  test("update() patches settings", async () => {
    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loaded).toBe(true);
    });

    await act(async () => {
      await result.current.update({ wizardCompleted: true });
    });

    expect(result.current.settings.wizardCompleted).toBe(true);
    // Other settings unchanged
    expect(result.current.settings.autoRefreshInterval).toBe(DEFAULT_SETTINGS.autoRefreshInterval);
  });
});
