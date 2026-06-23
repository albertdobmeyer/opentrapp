import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

import { test as base, expect } from "@playwright/test";

const COVERAGE_DIR = path.resolve(process.cwd(), ".nyc_output");

/**
 * Extends Playwright's `test` so that — ONLY when VITE_COVERAGE=true — each test
 * dumps the in-page istanbul coverage map (window.__coverage__, injected by
 * vite-plugin-istanbul) to .nyc_output/. `nyc` later turns those into an
 * istanbul report that scripts/merge-coverage.mjs merges with the vitest unit
 * coverage. Completely inert on the normal CI smoke run (flag unset).
 *
 * The fixture is auto-applied, so specs only need to import `test`/`expect`
 * from this file instead of "@playwright/test" — no per-test wiring.
 */
export const test = base.extend<{ collectCoverage: void; tauriRuntimeSim: void }>({
  /**
   * Simulate the SHIPPED runtime in every e2e test. OpenTrApp still ships as the Tauri 2 desktop
   * app (v0.8.0 is the de-Tauri *foundation* — the browser viewer is built but not yet the default,
   * ADR-0022), so the end-user-faithful runtime to exercise here is the Tauri webview. We inject a
   * `__TAURI_INTERNALS__` whose `invoke` rejects: that flips `isTauriRuntime()` (app/src/lib/runtime.ts)
   * true, so the `tauri.ts` chokepoint takes the native-IPC path — NOT the `/api/*` fetch path, which
   * would 404 against a `viewer-server` the static e2e harness does not run. The rejection is
   * `tauri`/`invoke`-tagged, so the "no JS errors" specs' existing backend-unavailable filter catches
   * it — exactly the graceful degradation the app showed before the de-Tauri shim. The browser/fetch
   * transport is covered separately by `tauri.test.ts` (unit) + the viewer-server Rust integration
   * tests; full browser e2e needs the running server (step 4, hardware-gated).
   */
  tauriRuntimeSim: [
    async ({ page }, use) => {
      await page.addInitScript(() => {
        // A complete-enough `__TAURI_INTERNALS__` so BOTH `invoke()` and `listen()` (which calls
        // `transformCallback` then `invoke('plugin:event|listen', …)`) flow through one rejecting
        // `invoke` — a clean "Tauri present, backend unavailable" simulation. `transformCallback`
        // returns a callback id like real Tauri so `listen()` reaches `invoke` instead of throwing a
        // different TypeError; the rejection is `tauri`/`invoke`-tagged for the console-error filter.
        let cbId = 0;
        (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {
          invoke: (cmd: string) =>
            Promise.reject(new Error(`tauri invoke unavailable in e2e (no backend): ${cmd}`)),
          transformCallback: () => (cbId += 1),
        };
      });
      await use();
    },
    { auto: true },
  ],

  collectCoverage: [
    async ({ page }, use, testInfo) => {
      await use();
      if (process.env.VITE_COVERAGE !== "true") return;
      const cov = await page
        .evaluate(() => (window as unknown as { __coverage__?: unknown }).__coverage__)
        .catch(() => null);
      if (cov === null || cov === undefined) return;
      mkdirSync(COVERAGE_DIR, { recursive: true });
      const safe = testInfo.testId.replace(/[^a-z0-9]/gi, "_");
      writeFileSync(path.join(COVERAGE_DIR, `e2e-${safe}.json`), JSON.stringify(cov));
    },
    { auto: true },
  ],
});

export { expect };
