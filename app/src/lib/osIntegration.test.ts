import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";

import {
  ensureNotificationPermission,
  getAutostartEnabled,
  setAutostartEnabled,
} from "./osIntegration";

// test-setup.ts sets __TAURI_INTERNALS__, so isTauri === true here and the
// real (non-browser) branches run against these mocked plugins.
vi.mock("@tauri-apps/plugin-autostart", () => ({
  enable: vi.fn(),
  disable: vi.fn(),
  isEnabled: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: vi.fn(),
  requestPermission: vi.fn(),
}));

const mEnable = vi.mocked(enable);
const mDisable = vi.mocked(disable);
const mIsEnabled = vi.mocked(isEnabled);
const mGranted = vi.mocked(isPermissionGranted);
const mRequest = vi.mocked(requestPermission);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("setAutostartEnabled", () => {
  test("enables when currently disabled, returns true", async () => {
    mIsEnabled.mockResolvedValue(false);
    await expect(setAutostartEnabled(true)).resolves.toBe(true);
    expect(mEnable).toHaveBeenCalledTimes(1);
    expect(mDisable).not.toHaveBeenCalled();
  });

  test("disables when currently enabled, returns true", async () => {
    mIsEnabled.mockResolvedValue(true);
    await expect(setAutostartEnabled(false)).resolves.toBe(true);
    expect(mDisable).toHaveBeenCalledTimes(1);
  });

  test("no-op + false when already in the desired state", async () => {
    mIsEnabled.mockResolvedValue(true);
    await expect(setAutostartEnabled(true)).resolves.toBe(false);
    expect(mEnable).not.toHaveBeenCalled();
    expect(mDisable).not.toHaveBeenCalled();
  });

  test("re-throws a friendly error when the plugin fails", async () => {
    mIsEnabled.mockRejectedValue(new Error("dbus down"));
    await expect(setAutostartEnabled(true)).rejects.toThrow(/Couldn't update startup setting.*dbus down/);
  });
});

describe("getAutostartEnabled", () => {
  test("returns the plugin's value", async () => {
    mIsEnabled.mockResolvedValue(true);
    await expect(getAutostartEnabled()).resolves.toBe(true);
  });

  test("returns false (not throw) when the plugin fails", async () => {
    mIsEnabled.mockRejectedValue(new Error("nope"));
    await expect(getAutostartEnabled()).resolves.toBe(false);
  });
});

describe("ensureNotificationPermission", () => {
  test("already granted → 'granted' without prompting", async () => {
    mGranted.mockResolvedValue(true);
    await expect(ensureNotificationPermission()).resolves.toBe("granted");
    expect(mRequest).not.toHaveBeenCalled();
  });

  test("not granted, user accepts → 'granted'", async () => {
    mGranted.mockResolvedValue(false);
    mRequest.mockResolvedValue("granted");
    await expect(ensureNotificationPermission()).resolves.toBe("granted");
  });

  test("not granted, user declines → 'denied'", async () => {
    mGranted.mockResolvedValue(false);
    mRequest.mockResolvedValue("denied");
    await expect(ensureNotificationPermission()).resolves.toBe("denied");
  });

  test("plugin failure → 'unavailable'", async () => {
    mGranted.mockRejectedValue(new Error("no plugin"));
    await expect(ensureNotificationPermission()).resolves.toBe("unavailable");
  });
});
