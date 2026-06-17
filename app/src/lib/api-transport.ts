// SCAFFOLD — frontend transport shim for the de-Tauri web GUI (spec §4 / ADR-0022 §migration-3).
//
// NOT yet wired. The migration swaps ONLY the internals of `tauri.ts` to use this; every exported
// function in `tauri.ts` keeps its current name + signature, so the ~73 components that import it
// do not change. (i.e. `tauri.ts`'s private `invoke()` body calls `apiInvoke` below; its `listen()`
// hooks subscribe via `events` below.)
//
// Session bootstrap (run once at app entry, before React mounts — see spec §2.3 / §4):
//   const n = new URLSearchParams(location.hash.slice(1)).get("n");        // launch nonce in #fragment
//   await fetch("/api/session", { method: "POST", headers: {"content-type":"application/json"},
//                                 body: JSON.stringify({ nonce: n }) });    // server sets HttpOnly cookie
//   history.replaceState(null, "", location.pathname);                      // scrub the nonce from history
// The bearer lives in the HttpOnly cookie; we send `credentials: "same-origin"`. The long-lived
// token is NEVER read by JS / placed in a URL (spec §2.3).

interface ApiError {
  error?: string;
}

/** Replaces Tauri `invoke(cmd, args)`. POST /api/<cmd> with named args as the JSON body. */
export async function apiInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const res = await fetch(`/api/${cmd}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    credentials: "same-origin", // HttpOnly bearer cookie (§2.3)
    body: JSON.stringify(args ?? {}),
  });
  if (!res.ok) {
    let detail = "";
    try {
      const body = (await res.json()) as ApiError;
      detail = body.error ?? "";
    } catch {
      /* non-JSON error body */
    }
    throw new Error(`api ${cmd} failed: ${String(res.status)} ${detail}`);
  }
  return (await res.json()) as T;
}

/** Replaces the per-event Tauri `listen()` calls with ONE shared WS to /api/events. */
type Handler = (payload: unknown) => void;
interface WsMessage {
  event: string;
  payload: unknown;
}

const handlers = new Map<string, Set<Handler>>();
let ws: WebSocket | null = null;

export const events = {
  /** Mirror of Tauri `listen(name, cb)`. Returns an unsubscribe fn. */
  listen(name: string, cb: Handler): () => void {
    let set = handlers.get(name);
    if (!set) {
      set = new Set<Handler>();
      handlers.set(name, set);
    }
    set.add(cb);
    ensureSocket();
    return () => {
      handlers.get(name)?.delete(cb);
    };
  },
};

function ensureSocket(): void {
  if (ws && ws.readyState <= WebSocket.OPEN) return;
  ws = new WebSocket(`ws://${location.host}/api/events`);
  // TODO: first-frame bearer auth (§2.3); on reconnect, re-seed state from get_perimeter_state /
  // get_status (markers remain truth, ADR-0019). Add backoff + close handling.
  ws.addEventListener("message", (e: MessageEvent<string>) => {
    const msg = JSON.parse(e.data) as WsMessage;
    const subs = handlers.get(msg.event);
    if (!subs) return;
    for (const h of subs) h(msg.payload);
  });
}

// Plugin replacements (spec §4 — all already the code's error-tolerant fallbacks):
//   shell.open(url)            → window.open(url, "_blank", "noopener")
//   clipboard.writeText(t)     → navigator.clipboard.writeText(t)
//   notification              → Web Notifications API
//   autostart toggle          → daemon-side service unit (new endpoint) or dropped from the web panel
//   store                     → localStorage (UI prefs only)
