import { test, expect } from "./coverage-fixture";
import { skipRoutingGuard } from "./_demo-helpers";

/**
 * Non-technical user experience tests.
 *
 * These tests verify the frontend reframe: a user should never see
 * developer jargon. Every page is checked for banned terms and for
 * the presence of user-friendly language.
 */

// Developer terms that must NEVER appear in user-visible text.
// (They can still exist in HTML attributes, data-testid, URLs, etc.)
//
// Pass 7 Day 4 additions, all caught from the live Telethon Pass 1.5
// transcripts and the spending-feature unwind:
//   "container", "sandboxed" — leaked from bot replies, also a sweep
//     hedge against any new infrastructure-leak.
//   "web_search", "web_fetch" — Anthropic tool-call names appeared raw
//     in Telegram replies; user-mode UI must never reproduce that.
//   "admin key", "billing scope", "cost endpoint" — vestiges of the
//     short-lived admin-API spending integration (Day 1a, unwound 1b);
//     guarding so they can't quietly creep back in.
//   "Podman", "Docker" — runtime names are allowed only behind a
//     "Show terminal command" disclosure on the install screen
//     (Pass 5). Banning the bare strings catches accidental leaks
//     into any other surface.
const BANNED_TERMS = [
  "OpenClaw Orchestrator",
  "OpenClaw Vault",
  "ClawHub Forge",
  "Moltbook Pioneer",
  "MoltBook Pioneer",
  "container_runtime",
  "component.yml",
  "compose.yml",
  "seccomp",
  "MITRE ATT&CK",
  "proxy",
  "manifest",
  "Monorepo",
  "monorepo",
  "health probes",
  "configure components",
  "Checking prerequisites",
  "submodule",
  "Submodule",
  "containers",
  "sandboxed",
  "web_search",
  "web_fetch",
  "admin key",
  "Admin key",
  "Admin Key",
  "billing scope",
  "cost endpoint",
  // Product name replaced by "the sandbox engine" in MissingRuntimeCard;
  // banning prevents re-introduction into any user-visible surface.
  "Podman Desktop",
];

/** Get visible text from the page, ignoring script/style/meta content */
async function getVisibleText(page: import("@playwright/test").Page): Promise<string> {
  return page.evaluate(() => {
    const walker = document.createTreeWalker(
      document.body,
      NodeFilter.SHOW_TEXT,
      {
        acceptNode(node) {
          const parent = node.parentElement;
          if (!parent) return NodeFilter.FILTER_REJECT;
          const tag = parent.tagName.toLowerCase();
          if (["script", "style", "noscript"].includes(tag))
            return NodeFilter.FILTER_REJECT;
          // Skip invisible elements
          const style = window.getComputedStyle(parent);
          if (style.display === "none" || style.visibility === "hidden")
            return NodeFilter.FILTER_REJECT;
          return NodeFilter.FILTER_ACCEPT;
        },
      },
    );
    const parts: string[] = [];
    while (walker.nextNode()) {
      const val = walker.currentNode.textContent?.trim();
      if (val) parts.push(val);
    }
    return parts.join(" ");
  });
}

function assertNoBannedTerms(text: string, pageName: string) {
  for (const term of BANNED_TERMS) {
    expect(text, `Banned term "${term}" found on ${pageName}`).not.toContain(
      term,
    );
  }
}

test.describe("Non-technical user experience", () => {
  // Skip Zone 1 routing guard for post-wizard tests. The wizard-specific
  // test below explicitly navigates to /setup which is exempt from the
  // guard regardless.
  test.beforeEach(async ({ page }) => {
    await skipRoutingGuard(page);
  });

  test("setup wizard welcome uses friendly language", async ({ page }) => {
    await page.goto("/setup");
    await expect(
      page.getByRole("heading", { name: /welcome/i }),
    ).toBeVisible();

    // Must contain assistant-first language
    await expect(page.getByText(/personal AI assistant/i)).toBeVisible();
    await expect(page.getByText(/safely|safe/i).first()).toBeVisible();
    await expect(
      page.getByRole("button", { name: /get started/i }),
    ).toBeVisible();

    // No developer jargon
    const text = await getVisibleText(page);
    assertNoBannedTerms(text, "Setup Welcome");
    expect(text).not.toMatch(/ecosystem/i);
    expect(text).not.toMatch(/security-first desktop GUI/i);
  });

  test("home page uses assistant-first language", async ({ page }) => {
    await page.goto("/");

    // Either shows home page (sidebar visible) or redirects to setup wizard — both are valid
    const homeLoaded = page.getByRole("link", { name: /preferences/i });
    const setup = page.getByText(/welcome/i);
    await expect(homeLoaded.or(setup)).toBeVisible();

    const text = await getVisibleText(page);
    assertNoBannedTerms(text, "Home");

    // If on home page, assistant-first language should be present
    if (await homeLoaded.isVisible()) {
      await expect(page.getByText(/assistant/i)).toBeVisible();
    }
  });

  test("preferences page has no developer jargon in user-visible text", async ({
    page,
  }) => {
    // /settings is a back-compat redirect to /preferences.
    await page.goto("/settings");
    await expect(
      page.getByRole("heading", { name: "Preferences" }),
    ).toBeVisible();

    const text = await getVisibleText(page);
    assertNoBannedTerms(text, "Preferences");
  });

  test("component detail (unknown) says 'Page not found' not 'Component not found'", async ({
    page,
  }) => {
    await page.goto("/component/unknown-id");
    await expect(page.getByText("Page not found")).toBeVisible();

    const text = await getVisibleText(page);
    assertNoBannedTerms(text, "Component Detail 404");
  });

  test("sidebar shows role-based labels, not component names", async ({
    page,
  }) => {
    await page.goto("/preferences"); // Any user-mode page renders UserSidebar

    // UserSidebar uses 5 role-based icon-nav links: Home / Security / Discover
    // / Preferences / Help. No codenames. No "Components" section.
    const sidebar = page.locator("aside");
    await expect(sidebar).toBeVisible();

    const sidebarText = await sidebar.textContent();
    // Role-based labels present.
    expect(sidebarText).toContain("Home");
    expect(sidebarText).toContain("Preferences");
    // No codenames.
    expect(sidebarText).not.toContain("OpenClaw");
    expect(sidebarText).not.toContain("Components");
    expect(sidebarText).not.toContain("COMPONENTS");
  });

  test("no banned terms on any reachable page", async ({ page }) => {
    const pages = ["/", "/setup", "/settings"];

    for (const url of pages) {
      await page.goto(url);
      // Wait for content to render
      await page.waitForTimeout(500);
      const text = await getVisibleText(page);
      assertNoBannedTerms(text, url);
    }
  });
});
