/**
 * Runtime detection for the de-Tauri dual-mode GUI (ADR-0022): are we inside the Tauri webview, or
 * a plain browser served by `viewer-server`? Detected at CALL time (not module load) so the runtime
 * can differ per call / be set in tests.
 *
 * This lives in its own tiny module — NOT in `./tauri` — on purpose: many tests mock `@/lib/tauri`
 * wholesale to control the command wrappers, which would strip this detector and break the platform
 * shims that depend on it. Keeping it standalone means the shims' runtime check survives those mocks.
 */
export function isTauriRuntime(): boolean {
  return !!(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
}
