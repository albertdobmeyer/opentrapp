import "@testing-library/jest-dom/vitest";

// Mock @tauri-apps/api/core
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Mock @tauri-apps/api/event
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => undefined)),
  emit: vi.fn(() => Promise.resolve()),
}));

// De-Tauri dual transport (ADR-0022): the unit suite simulates the SHIPPED runtime — the Tauri
// webview — so the `invoke`/`listen` chokepoints (lib/tauri.ts, lib/events.ts) take the native path
// and the @tauri-apps/api mocks above intercept them. The browser fetch+WS path has its own
// dedicated tests (tauri.test.ts, platform.test.ts, session-bootstrap.test.ts) that toggle this.
(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
  invoke: vi.fn(),
  transformCallback: vi.fn(),
};
