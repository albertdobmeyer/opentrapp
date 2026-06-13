import { test, expect } from "./coverage-fixture";
import { skipRoutingGuard } from "./_demo-helpers";

test.describe("Smoke tests", () => {
  // Skip the Zone 1 routing guard so tests can hit user-mode routes
  // without being redirected to /setup. The wizard-specific test below
  // is independent — /setup is exempt from the guard either way.
  test.beforeEach(async ({ page }) => {
    await skipRoutingGuard(page);
  });

  test("app loads with home page or Setup wizard", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveTitle(/OpenTrApp/i);
    // On first run, the app may redirect to the setup wizard; otherwise shows home page with sidebar
    const homeLoaded = page.getByRole("link", { name: /preferences/i });
    const setup = page.getByText(/welcome|prerequisites/i);
    await expect(homeLoaded.or(setup).first()).toBeVisible();
  });

  test("content renders (not a blank white screen)", async ({ page }) => {
    await page.goto("/");
    const body = page.locator("body");
    const text = await body.textContent();
    // Should have meaningful text — either component names or discovery messages
    expect(text?.length).toBeGreaterThan(20);
  });

  test("navigation to /preferences works", async ({ page }) => {
    // /settings is a back-compat redirect to /preferences.
    await page.goto("/settings");
    await expect(page.getByRole("heading", { name: "Preferences" })).toBeVisible();
    // Footer shows version.
    await expect(page.getByText(/OpenTrApp v\d+\.\d+\.\d+/)).toBeVisible();
  });

  test("no unexpected console errors", async ({ page }) => {
    const errors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") {
        const text = msg.text();
        // Filter out expected Tauri-not-available errors
        if (
          !text.includes("__TAURI__") &&
          !text.includes("tauri") &&
          !text.includes("invoke")
        ) {
          errors.push(text);
        }
      }
    });

    await page.goto("/");
    // Wait for app to settle
    await page.waitForTimeout(1000);

    expect(errors).toEqual([]);
  });

  test("no console warnings about React Router future flags", async ({ page }) => {
    const warnings: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "warning") {
        warnings.push(msg.text());
      }
    });

    await page.goto("/");
    await page.waitForTimeout(1000);

    const routerWarnings = warnings.filter((w) => w.includes("v7_"));
    expect(routerWarnings).toEqual([]);
  });
});
