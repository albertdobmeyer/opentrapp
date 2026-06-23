/**
 * Clipboard shim for the de-Tauri dual-mode GUI (ADR-0022): copy text. Native Tauri clipboard in
 * the webview, the Web Clipboard API in a plain browser. Its own module so non-clipboard consumers
 * don't transitively import the clipboard plugin. Returns the plugin's promise directly (no extra
 * microtask — timing-transparent).
 */
import { writeText as tauriWriteText } from "@tauri-apps/plugin-clipboard-manager";

import { isTauriRuntime } from "./runtime";

/** Copy text to the clipboard: the Tauri clipboard in the webview, the Web Clipboard API otherwise. */
export function writeText(text: string): Promise<void> {
  if (isTauriRuntime()) {
    return tauriWriteText(text);
  }
  return navigator.clipboard.writeText(text);
}
