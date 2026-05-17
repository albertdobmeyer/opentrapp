// Thin wrappers around the Tauri OS-integration plugins. Centralises
// browser-mode safety (these throw `__TAURI_INTERNALS__` errors when
// run via `npm run dev`) and gives the rest of the app a friendlier API.

const isTauri = !!(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

// ─── Autostart ───────────────────────────────────────────────────────

/**
 * Register or unregister OpenTrApp with the OS so it launches at
 * login. Idempotent — safe to call when the desired state already matches.
 * Returns whether the change was actually applied (vs already in the
 * right state, or browser mode where it's a no-op).
 */
export async function setAutostartEnabled(enabled: boolean): Promise<boolean> {
  if (!isTauri) return false;
  try {
    const { enable, disable, isEnabled } = await import(
      "@tauri-apps/plugin-autostart"
    );
    const current = await isEnabled();
    if (current === enabled) return false;
    await (enabled ? enable() : disable());
    return true;
  } catch (error) {
    // Re-throw so the caller can surface a friendly toast.
    throw new Error(
      `Couldn't update startup setting: ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
  }
}

export async function getAutostartEnabled(): Promise<boolean> {
  if (!isTauri) return false;
  try {
    const { isEnabled } = await import("@tauri-apps/plugin-autostart");
    return await isEnabled();
  } catch {
    return false;
  }
}

// ─── Notifications ───────────────────────────────────────────────────

/**
 * Result of a permission request — `granted` (proceed), `denied` (user
 * said no, fall back to in-app toasts), or `unavailable` (browser mode
 * or the plugin failed to load).
 */
export type NotificationPermissionResult = "granted" | "denied" | "unavailable";

/**
 * Ensure the OS has granted notification permission. Cheap when already
 * granted (no prompt). Returns `denied` when the user has previously
 * declined — caller should keep the in-app toast fallback active.
 */
export async function ensureNotificationPermission(): Promise<NotificationPermissionResult> {
  if (!isTauri) return "unavailable";
  try {
    const { isPermissionGranted, requestPermission } = await import(
      "@tauri-apps/plugin-notification"
    );
    const already = await isPermissionGranted();
    if (already) return "granted";
    const result = await requestPermission();
    return result === "granted" ? "granted" : "denied";
  } catch {
    return "unavailable";
  }
}
