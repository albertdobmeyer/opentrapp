import { mkdir, writeFile, appendFile } from "node:fs/promises";
import * as path from "node:path";
import type { Page, TestInfo } from "@playwright/test";

/**
 * Inject the post-wizard escape hatch BEFORE the React app loads, so
 * App.tsx's routing guard sees `wizardCompletedForRouting = true` on
 * first render. Use in any test that exercises user-mode routes
 * (/preferences, /security, /help, /component/...). Production users
 * never set this — the override is read by App.tsx and is inert unless
 * explicitly set. See app/src/App.tsx (Zone 1 routing guard).
 */
export async function skipRoutingGuard(page: Page): Promise<void> {
  await page.addInitScript(() => {
    (window as unknown as { __OPENTRAPP_DEMO__?: boolean }).__OPENTRAPP_DEMO__ = true;
  });
}

/**
 * Demo screenshot recorder.
 *
 * Drives Playwright as usual but writes a *slideshow* (one PNG per beat +
 * a per-frame delay manifest) instead of relying on Playwright's video
 * pipeline. The video pipeline produces blank frames on this Linux setup
 * even though page.screenshot() captures the DOM correctly — sidestepped
 * here because the slideshow approach is honestly higher-quality for docs
 * gifs anyway (no codec compression artefacts).
 *
 * Output: <test-results>/<test-dirname>/frames/00.png, 01.png, …, plus
 * delays.txt with one centisecond-delay-per-frame on each line (ImageMagick
 * convert reads centiseconds, not ms).
 *
 * scripts/demo-gif.sh assembles the gif via:
 *   convert -delay <each> <each>.png … -loop 0 output.gif
 */
export class DemoRecorder {
  private framesDir!: string;
  private delaysFile!: string;
  private seq = 0;

  constructor(
    private page: Page,
    private testInfo: TestInfo,
  ) {}

  /** Call once at the top of the test, before any frame(). */
  async init(): Promise<void> {
    // testInfo.outputDir is the per-test output dir (test-results/...).
    this.framesDir = path.join(this.testInfo.outputDir, "frames");
    this.delaysFile = path.join(this.testInfo.outputDir, "delays.txt");
    await mkdir(this.framesDir, { recursive: true });
    await writeFile(this.delaysFile, "");
  }

  /**
   * Capture a frame and hold it for `holdMs` milliseconds in the resulting
   * gif. Use shorter holds for transitions (300-500), longer holds for
   * beats the viewer should read (1500-2500).
   */
  async frame(holdMs: number): Promise<void> {
    const name = String(this.seq).padStart(3, "0") + ".png";
    const fullPath = path.join(this.framesDir, name);
    await this.page.screenshot({ path: fullPath, fullPage: false });
    // ImageMagick `-delay` is in centiseconds.
    const centi = Math.max(1, Math.round(holdMs / 10));
    await appendFile(this.delaysFile, `${name} ${centi}\n`);
    this.seq += 1;
  }

  /**
   * Convenience: smooth-scroll by `pixels` and capture frames every
   * `stepMs`. Use for showing long content (e.g. the Security page's
   * five layer cards).
   */
  async scrollAndCapture(pixels: number, stepMs: number, steps: number): Promise<void> {
    const each = Math.round(pixels / steps);
    for (let i = 0; i < steps; i++) {
      await this.page.evaluate(({ y }) => {
        window.scrollBy({ top: y, behavior: "smooth" });
      }, { y: each });
      await this.page.waitForTimeout(stepMs);
      await this.frame(stepMs);
    }
  }
}
