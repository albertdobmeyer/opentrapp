/**
 * Shell shim for the de-Tauri dual-mode GUI (ADR-0022): open a URL externally. Native in the Tauri
 * webview, the Web Platform equivalent in a plain browser served by `viewer-server`. Split into its
 * own module (not a shared `platform`) so store/clipboard consumers don't transitively import the
 * shell plugin — each consumer pulls in only the plugin it uses.
 *
 * The wrapper returns the plugin's promise DIRECTLY (not `async`/`await`) so it adds no extra
 * microtask and is timing-transparent vs. the raw plugin call it replaces.
 */
import { open as tauriOpen } from "@tauri-apps/plugin-shell";

import { isTauriRuntime } from "./runtime";

/** Open a URL externally: the OS browser in Tauri, a new tab (noopener) in the browser viewer. */
export function openUrl(url: string): Promise<void> {
  if (isTauriRuntime()) {
    return tauriOpen(url);
  }
  window.open(url, "_blank", "noopener,noreferrer");
  return Promise.resolve();
}
