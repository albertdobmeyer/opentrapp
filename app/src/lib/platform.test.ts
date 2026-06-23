// Mock the native plugins so importing the shim modules is clean (we exercise the BROWSER fallback,
// not the plugin). The plugins' Tauri path is covered implicitly by the component tests, which run
// in Tauri mode via test-setup and assert the native plugin was called.
vi.mock("@tauri-apps/plugin-shell", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: vi.fn() }));
vi.mock("@tauri-apps/plugin-store", () => ({ load: vi.fn() }));

import { writeText } from "./clipboard";
import { openUrl } from "./shell";
import { load } from "./store";

// The de-Tauri loopback viewer (ADR-0022) runs these shims in plain-browser mode (no Tauri runtime),
// where they must use the Web Platform equivalents.
describe("platform shims — browser mode (no Tauri runtime)", () => {
  const savedTauri = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

  beforeEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });
  afterEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = savedTauri;
    vi.unstubAllGlobals();
    localStorage.clear();
  });

  test("openUrl opens a new noopener tab", async () => {
    const openMock = vi.fn();
    vi.stubGlobal("open", openMock);
    await openUrl("https://example.com");
    expect(openMock).toHaveBeenCalledWith("https://example.com", "_blank", "noopener,noreferrer");
  });

  test("writeText uses the Web Clipboard API", async () => {
    const writeTextMock = vi.fn<(t: string) => Promise<void>>().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText: writeTextMock } });
    await writeText("copied");
    expect(writeTextMock).toHaveBeenCalledWith("copied");
  });

  test("load returns a localStorage-backed store (get/set/save roundtrip, namespaced)", async () => {
    const store = await load("settings.json");
    expect(await store.get("app_settings")).toBeUndefined();

    await store.set("app_settings", { theme: "dark" });
    await store.save();

    expect(await store.get("app_settings")).toEqual({ theme: "dark" });
    expect(localStorage.getItem("otv-store:settings.json:app_settings")).toBe(
      JSON.stringify({ theme: "dark" }),
    );
  });
});
