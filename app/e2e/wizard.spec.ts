import { test, expect, type Page } from "@playwright/test";

/**
 * Phase E.2.1 onboarding wizard tests.
 *
 * These run against the Vite dev server (no Tauri IPC available), so they
 * exercise the UI transitions and pure-frontend logic only. Full happy-path
 * and container-build flows are covered by manual smoke — see the
 * verification plan for E.2.1.
 *
 * Developer terms that must NEVER appear in user-visible wizard text.
 */
const BANNED_WIZARD_TERMS = [
  "container",
  "Container",
  "manifest",
  "Manifest",
  "submodule",
  "Submodule",
  "podman",
  "Podman",
  "proxy",
  "Proxy",
  "component",
  "Component",
  "shell level",
  "vault-",
  "compose",
  "seccomp",
  // Project codenames — must never reach user-facing copy.
  "openclaw",
  "OpenClaw",
  "clawhub",
  "ClawHub",
  "moltbook",
  "Moltbook",
];

// Exceptions — phrases that happen to contain a banned token for legitimate
// reasons (e.g. the MissingRuntimeCard tells users to install Podman/Docker
// which is a real user action, and the InstallStep's pulsing-rings role label
// shows when Tauri IPC fails and we render the level-2 error instead).
const EXCEPTION_PHRASES = [
  "Podman or Docker",
  "Download Podman Desktop",
  "install podman",
  "Installation in progress",
];

async function getVisibleText(page: Page): Promise<string> {
  return page.evaluate(() => {
    const walker = document.createTreeWalker(
      document.body,
      NodeFilter.SHOW_TEXT,
      {
        acceptNode(node) {
          const parent = node.parentElement;
          if (!parent) return NodeFilter.FILTER_REJECT;
          const tag = parent.tagName.toLowerCase();
          if (["script", "style", "noscript"].includes(tag)) {
            return NodeFilter.FILTER_REJECT;
          }
          const style = window.getComputedStyle(parent);
          if (style.display === "none" || style.visibility === "hidden") {
            return NodeFilter.FILTER_REJECT;
          }
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

function stripExceptions(text: string): string {
  let stripped = text;
  for (const phrase of EXCEPTION_PHRASES) {
    stripped = stripped.split(phrase).join("");
  }
  return stripped;
}

function assertNoBannedWizardTerms(text: string, context: string) {
  const searchable = stripExceptions(text);
  for (const term of BANNED_WIZARD_TERMS) {
    expect(
      searchable,
      `Banned wizard term "${term}" found in ${context}`,
    ).not.toContain(term);
  }
}

test.describe("Setup wizard — 4-step flow", () => {
  test("Welcome step renders spec copy and focused CTA", async ({ page }) => {
    await page.goto("/setup");
    // Brand banner image carries the OpenTrApp wordmark; the heading
    // beside it reads simply "Welcome".
    await expect(
      page.getByRole("img", { name: "OpenTrApp" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Welcome" }),
    ).toBeVisible();
    await expect(
      page.getByText("Your personal AI assistant, safe on your computer."),
    ).toBeVisible();
    await expect(page.getByText(/about 3 minutes/)).toBeVisible();

    const cta = page.getByRole("button", { name: "Get Started" });
    await expect(cta).toBeVisible();
    // Spec §Step 1 Accessibility — button is focused on load.
    await expect(cta).toBeFocused();
  });

  test("Welcome → Connect transition shows progress bar", async ({ page }) => {
    await page.goto("/setup");
    await page.getByRole("button", { name: "Get Started" }).click();

    await expect(
      page.getByRole("heading", { name: "Connect your accounts" }),
    ).toBeVisible();
    // Progress nav landmark is rendered on steps 2–4 only.
    await expect(page.getByRole("navigation", { name: "Setup progress" })).toBeVisible();
    // Both key cards present.
    await expect(page.getByText("Anthropic API key")).toBeVisible();
    await expect(page.getByText("Telegram bot")).toBeVisible();
  });

  test("paste swap: Anthropic key pasted into Telegram field auto-corrects", async ({
    page,
  }) => {
    await page.goto("/setup");
    await page.getByRole("button", { name: "Get Started" }).click();

    // Ensure Connect step is ready before interacting.
    await expect(
      page.getByRole("heading", { name: "Connect your accounts" }),
    ).toBeVisible();

    const telegramField = page.locator("#telegram-token");
    await telegramField.focus();

    // Simulate a clipboard paste targeting the telegram field with an
    // Anthropic-shaped key. The ConnectStep handler should intercept the
    // paste event and redirect the value into the anthropic field.
    await telegramField.evaluate((el) => {
      const data = new DataTransfer();
      data.setData("text", "sk-ant-api03-fake-key-for-paste-swap-test");
      const ev = new ClipboardEvent("paste", {
        clipboardData: data,
        bubbles: true,
        cancelable: true,
      });
      el.dispatchEvent(ev);
    });

    await expect(page.locator("#anthropic-key")).toHaveValue(
      /^sk-ant-api03-fake-key-for-paste-swap-test$/,
    );
    await expect(telegramField).toHaveValue("");
  });

  test("no developer jargon on Welcome or Connect screens", async ({ page }) => {
    await page.goto("/setup");
    assertNoBannedWizardTerms(await getVisibleText(page), "Welcome");

    await page.getByRole("button", { name: "Get Started" }).click();
    await expect(
      page.getByRole("heading", { name: "Connect your accounts" }),
    ).toBeVisible();
    assertNoBannedWizardTerms(await getVisibleText(page), "Connect");

    // Expand both how-to modals — they're surface area that Karen will see.
    await page.getByRole("button", { name: /Show me how to get one/ }).click();
    await expect(
      page.getByRole("heading", { name: "How to get an Anthropic API key" }),
    ).toBeVisible();
    assertNoBannedWizardTerms(
      await getVisibleText(page),
      "Anthropic how-to modal",
    );
    await page.getByRole("button", { name: "Done, let me enter it" }).click();

    await page.getByRole("button", { name: /Walk me through it/ }).click();
    await expect(
      page.getByRole("heading", { name: "How to create a Telegram bot" }),
    ).toBeVisible();
    assertNoBannedWizardTerms(
      await getVisibleText(page),
      "Telegram how-to modal",
    );
  });
});
