import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { bootstrapSession } from "./session-bootstrap";

// The de-Tauri loopback viewer (ADR-0022 §2.3): the daemon opens the browser to a BARE loopback URL
// with a single-use launch nonce in the URL *fragment* (`#n=<nonce>`). On load, before any `/api/*`
// call, the app must exchange that nonce at `POST /api/session` for the HttpOnly `otv_bearer` cookie
// the §2 security middleware requires, then scrub the nonce from history so it never leaks via
// back-button / Referer. In the Tauri webview there is no nonce/cookie — the bootstrap must no-op.
describe("session bootstrap (browser viewer nonce → bearer exchange)", () => {
  const savedTauri = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

  beforeEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    window.location.hash = "";
    history.replaceState(null, "", "/");
  });
  afterEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = savedTauri;
    vi.unstubAllGlobals();
    window.location.hash = "";
  });

  test("browser mode with #n= nonce: POSTs it to /api/session and scrubs the nonce from history", async () => {
    window.location.hash = "#n=deadbeefcafebabe0123456789abcdef";
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);
    const replaceSpy = vi.spyOn(history, "replaceState");

    await bootstrapSession();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/session");
    expect(init?.method).toBe("POST");
    expect(init?.credentials).toBe("same-origin");
    expect(JSON.parse(init?.body as string)).toEqual({ nonce: "deadbeefcafebabe0123456789abcdef" });
    // nonce scrubbed from the URL (no longer in the hash)
    expect(replaceSpy).toHaveBeenCalled();
    expect(window.location.hash).not.toContain("deadbeef");
  });

  test("Tauri runtime: does NOT call /api/session (native IPC, no session)", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = { invoke: vi.fn() };
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);

    await bootstrapSession();

    expect(fetchMock).not.toHaveBeenCalled();
  });

  test("browser mode without a nonce: does nothing (no exchange to make)", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);

    await bootstrapSession();

    expect(fetchMock).not.toHaveBeenCalled();
  });

  test("browser mode: a failed exchange does not throw and still scrubs the single-use nonce", async () => {
    window.location.hash = "#n=00000000000000000000000000000000";
    // single-use nonce: even a rejected exchange must be scrubbed (the server burns it on any attempt)
    const fetchMock = vi.fn<typeof fetch>().mockRejectedValue(new Error("network down"));
    vi.stubGlobal("fetch", fetchMock);
    const replaceSpy = vi.spyOn(history, "replaceState");

    await expect(bootstrapSession()).resolves.toBeUndefined();
    expect(replaceSpy).toHaveBeenCalled();
    expect(window.location.hash).not.toContain("0000");
  });
});
