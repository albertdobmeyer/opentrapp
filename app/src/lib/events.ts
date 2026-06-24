import { listen as tauriListen, type Event, type UnlistenFn } from "@tauri-apps/api/event";

import { isTauriRuntime } from "./runtime";

// The dual-transport EVENT chokepoint (de-Tauri, ADR-0022 §4), the counterpart to the `invoke`
// chokepoint in `tauri.ts`. Every `listen()` call site imports from here instead of
// `@tauri-apps/api/event`, so swapping the transport is invisible to the ~8 hooks that subscribe.
//
//  - Tauri webview  → the native `@tauri-apps/api/event` listen (unchanged shipped behaviour).
//  - Plain browser  → ONE shared WebSocket to `/api/events`, dispatching frames to subscribers by
//    event name (mirrors Tauri's emit/listen). Auth is on the handshake: the HttpOnly `otv_bearer`
//    cookie rides the upgrade request automatically; the §2 server middleware enforces it.
//
// `markers remain truth` (ADR-0019): the WS is a fast PUSH path; hooks still re-seed from the
// `get_*` reads on mount, so a missed/closed socket degrades to the polled value, never to a lie.

export type { Event };

type Listener<T> = (event: Event<T>) => void;

const subscribers = new Map<string, Set<(payload: unknown) => void>>();
let socket: WebSocket | null = null;

function ensureSocket(): void {
  if (socket && socket.readyState <= WebSocket.OPEN) return;
  // Same-origin ws:// — the bearer cookie is sent on the handshake. Loopback plaintext is correct
  // (§2.4: TLS on 127.0.0.1 adds a cert-trust problem for no threat reduction).
  socket = new WebSocket(`ws://${window.location.host}/api/events`);
  socket.addEventListener("message", (e: MessageEvent<string>) => {
    let msg: { event: string; payload: unknown };
    try {
      msg = JSON.parse(e.data) as { event: string; payload: unknown };
    } catch {
      return; // ignore a non-JSON frame rather than throw in the socket callback
    }
    const subs = subscribers.get(msg.event);
    if (subs) for (const fn of subs) fn(msg.payload);
  });
}

function browserListen<T>(event: string, handler: Listener<T>): UnlistenFn {
  let set = subscribers.get(event);
  if (!set) {
    set = new Set();
    subscribers.set(event, set);
  }
  // Wrap so the public surface matches Tauri's `Event<T>` shape (handlers read `.payload`).
  const wrapped = (payload: unknown): void => { handler({ event, id: 0, payload: payload as T }); };
  set.add(wrapped);
  ensureSocket();
  return () => {
    subscribers.get(event)?.delete(wrapped);
  };
}

/** Dual-transport mirror of Tauri's `listen<T>(event, handler)`. Returns an unsubscribe fn. */
export async function listen<T>(event: string, handler: Listener<T>): Promise<UnlistenFn> {
  if (isTauriRuntime()) return tauriListen<T>(event, handler);
  return browserListen(event, handler);
}
