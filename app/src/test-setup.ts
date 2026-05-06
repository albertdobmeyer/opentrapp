import "@testing-library/jest-dom/vitest";

// Simulate Tauri environment so isTauri check passes in tauri.ts
(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-store", () => {
  const store = new Map<string, unknown>();
  const mockStore = {
    get: vi.fn((key: string) => Promise.resolve(store.get(key))),
    set: vi.fn((key: string, value: unknown) => {
      store.set(key, value);
      return Promise.resolve();
    }),
    save: vi.fn(() => Promise.resolve()),
    _store: store,
  };
  return {
    load: vi.fn(() => Promise.resolve(mockStore)),
    __mockStore: mockStore,
  };
});
