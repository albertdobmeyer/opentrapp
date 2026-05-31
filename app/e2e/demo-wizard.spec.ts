import { test } from "@playwright/test";
import { DemoRecorder } from "./_demo-helpers";

/**
 * Demo recorder — Karen's first-run experience.
 *
 * Records the routing guard sending a fresh user to /setup, plus the wizard's
 * welcome screen. Bootstrap steps require Tauri IPC so they don't render in
 * browser mode; the recording stops at the welcome screen, which is the
 * honest portrayal of what a new user sees before the backend takes over.
 *
 * Captures via the screenshot slideshow path in _demo-helpers.ts (the
 * Playwright video pipeline produces blank frames on this Linux setup).
 *
 * Only runs under the `demo` project (see playwright.config.ts). Excluded
 * from the default CI test run.
 */
test("Karen's first run", async ({ page }, testInfo) => {
  const rec = new DemoRecorder(page, testInfo);
  await rec.init();
  page.setDefaultTimeout(15_000);

  await page.goto("/");
  // Beat 1: arriving at the wizard — hold so viewers register where they are.
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(500);
  await rec.frame(2000);

  // Beat 2: small scroll down to show this is real content + scroll back up.
  // Each scroll step captures a frame, so viewers see the page move.
  await rec.scrollAndCapture(200, 400, 3);
  await page.evaluate(() => window.scrollTo({ top: 0, behavior: "smooth" }));
  await page.waitForTimeout(600);
  await rec.frame(1200);

  // Beat 3: hover the primary CTA so the gif ends on an actionable beat.
  // We don't click — clicking advances into Tauri-IPC-dependent steps.
  const cta = page.getByRole("button").filter({ hasText: /next|continue|get started|begin|let'?s go|start/i }).first();
  if (await cta.count() > 0) {
    await cta.hover();
    await page.waitForTimeout(500);
    await rec.frame(2200);
  } else {
    await rec.frame(1500);
  }
});
