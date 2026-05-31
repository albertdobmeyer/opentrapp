import { test, expect } from "@playwright/test";
import { DemoRecorder } from "./_demo-helpers";

/**
 * Demo recorder — security story tour.
 *
 * Walks through the polished v0.6 user-mode surfaces: Home (hero status card)
 * → Security (the five protective layers from Zone 1) → Help → Preferences.
 *
 * Uses window.__OPENTRAPP_DEMO__ to bypass the routing guard so the recording
 * can show the post-wizard state without a real Tauri settings store. The
 * override is read by app/src/App.tsx and is inert unless explicitly set —
 * production users never see this code path.
 *
 * Captures via the screenshot slideshow path in _demo-helpers.ts.
 *
 * Only runs under the `demo` project (see playwright.config.ts). Excluded
 * from the default CI test run.
 */
test("Security story tour", async ({ page }, testInfo) => {
  await page.addInitScript(() => {
    (window as unknown as { __OPENTRAPP_DEMO__?: boolean }).__OPENTRAPP_DEMO__ = true;
  });

  const rec = new DemoRecorder(page, testInfo);
  await rec.init();
  page.setDefaultTimeout(15_000);

  // ─── Beat 1: Home ─────────────────────────────────────────────────
  await page.goto("/");
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(800);
  await rec.frame(2000);

  // ─── Beat 2: Security (the headliner) ─────────────────────────────
  await page.goto("/security");
  await expect(page.getByRole("heading", { name: "Security", level: 1 })).toBeVisible();
  await page.waitForTimeout(500);
  await rec.frame(2200);

  // Scroll through the five layer cards, capturing a frame for each.
  await rec.scrollAndCapture(900, 900, 5);

  // Hold on the footer ("For the technically curious") — the bridge to
  // the opencode pitch.
  await page.waitForTimeout(400);
  await rec.frame(1800);

  // ─── Beat 3: Help ─────────────────────────────────────────────────
  await page.goto("/help");
  await expect(page.getByRole("heading", { name: "Help & support", level: 1 })).toBeVisible();
  await page.waitForTimeout(500);
  await rec.frame(1800);
  await rec.scrollAndCapture(300, 600, 2);

  // ─── Beat 4: Preferences (key fields) ─────────────────────────────
  await page.goto("/preferences");
  await expect(page.getByRole("heading", { name: "Preferences", exact: true })).toBeVisible();
  await page.waitForTimeout(500);
  await rec.frame(1500);
  await page.getByRole("heading", { name: "Your keys" }).scrollIntoViewIfNeeded();
  await page.waitForTimeout(500);
  await rec.frame(2500);

  // ─── Closing: back to Security ────────────────────────────────────
  await page.goto("/security");
  await expect(page.getByRole("heading", { name: "Security", level: 1 })).toBeVisible();
  await page.waitForTimeout(500);
  await rec.frame(2000);
});
