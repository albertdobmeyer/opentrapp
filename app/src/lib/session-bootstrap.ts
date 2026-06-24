import { isTauriRuntime } from "./runtime";

/**
 * De-Tauri loopback-viewer session bootstrap (ADR-0022 §2.3). Run ONCE at app entry, before React
 * mounts and before any `/api/*` call.
 *
 * The daemon/launcher opens the browser to a bare loopback URL with a single-use launch nonce in the
 * URL *fragment* (`http://127.0.0.1:<port>/#n=<nonce>`). The fragment is never sent to the server in
 * a request line / Referer, so the nonce can't leak that way. Here we read it, exchange it at
 * `POST /api/session`, and the server validates + BURNS it and returns the bearer as an HttpOnly
 * `otv_bearer` cookie (`SameSite=Strict; Path=/`). Every later request just sends
 * `credentials: "same-origin"`; JS never reads the long-lived token. We then scrub the nonce from
 * the URL so it survives neither the back button nor `history`.
 *
 * In the Tauri webview there is no nonce and no cookie — commands go over native IPC — so this is a
 * no-op there, leaving the shipped desktop app's startup unchanged.
 */
export async function bootstrapSession(): Promise<void> {
  if (isTauriRuntime()) return; // native IPC — no loopback session to establish

  const nonce = new URLSearchParams(window.location.hash.slice(1)).get("n");
  if (!nonce) return; // nothing to exchange (e.g. a reload after the nonce was already scrubbed)

  try {
    await fetch("/api/session", {
      method: "POST",
      headers: { "content-type": "application/json" },
      credentials: "same-origin", // the Set-Cookie bearer must stick to the origin
      body: JSON.stringify({ nonce }),
    });
  } catch {
    // A failed exchange is not fatal here: the app still mounts and surfaces the resulting 401s
    // through its normal error handling. We must NOT rethrow (that would white-screen the app).
  } finally {
    // The nonce is single-use — the server burns it on ANY attempt — so scrub it regardless of
    // outcome, keeping it out of the back button / history / Referer.
    history.replaceState(null, "", window.location.pathname + window.location.search);
  }
}
