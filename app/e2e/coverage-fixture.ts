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
export const test = base.extend<{ collectCoverage: void }>({
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
