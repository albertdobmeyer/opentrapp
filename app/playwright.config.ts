import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  retries: 0,
  use: {
    baseURL: "http://localhost:1420",
    headless: true,
  },
  webServer: {
    command: "npm run dev",
    port: 1420,
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
  projects: [
    {
      // Default test project — CI runs this. Excludes the demo specs so
      // CI doesn't waste cycles producing videos.
      name: "default",
      testIgnore: /demo-.*\.spec\.ts/,
    },
    {
      // Demo recorder — only runs when explicitly invoked
      // (`npx playwright test --project=demo` or via `npm run demo:record`).
      // Each spec writes a screenshot slideshow + delays.txt under
      // test-results/; scripts/demo-gif.sh assembles them into gifs.
      //
      // Why slideshow and not Playwright's `video` mode? The video pipeline
      // produces blank frames on this Linux setup even though
      // page.screenshot() captures the DOM correctly (confirmed by
      // toBeVisible() assertions passing alongside blank frames). The
      // slideshow path is also higher-quality for docs gifs — no VP8
      // compression artefacts in the gif palette.
      name: "demo",
      testMatch: /demo-.*\.spec\.ts/,
      timeout: 120_000,
      use: {
        viewport: { width: 1280, height: 800 },
      },
    },
  ],
});
