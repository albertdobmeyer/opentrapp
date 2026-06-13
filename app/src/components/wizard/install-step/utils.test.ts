import { deriveTelegramBotUrl, readRuntimeEnv } from "@/lib/tauri";

import {
  detectPlatform,
  estimateRemaining,
  formatElapsed,
  INITIAL_STEPS,
  prefetchTelegramUrl,
  sanitizeLine,
  type SubStep,
} from "./utils";

vi.mock("@/lib/tauri", () => ({
  readRuntimeEnv: vi.fn(),
  deriveTelegramBotUrl: vi.fn(),
}));

const mockReadEnv = vi.mocked(readRuntimeEnv);
const mockDeriveBot = vi.mocked(deriveTelegramBotUrl);

const setUA = (ua: string) =>
  Object.defineProperty(navigator, "userAgent", { value: ua, configurable: true });

const step = (over: Partial<SubStep>): SubStep => ({
  id: "check",
  label: "Check",
  status: "pending",
  startedAt: null,
  durationMs: null,
  retryAttempt: 0,
  technicalLog: [],
  ...over,
});

describe("install-step utils", () => {
  describe("sanitizeLine", () => {
    test("redacts an Anthropic key (assignment + raw sk-ant token)", () => {
      expect(sanitizeLine("ANTHROPIC_API_KEY=sk-ant-secret123")).toBe(
        "ANTHROPIC_API_KEY=[REDACTED]",
      );
      // The raw-token path also gets caught by the OpenAI `sk-` rule, so assert
      // the security property (the secret chars are gone, a marker remains)
      // rather than the exact marker count.
      const out = sanitizeLine("using sk-ant-api03-abcDEF012-_x now");
      expect(out).not.toContain("abcDEF012");
      expect(out).toContain("[REDACTED]");
      expect(out.startsWith("using ")).toBe(true);
      expect(out.endsWith(" now")).toBe(true);
    });

    test("redacts an OpenAI key and a Telegram token", () => {
      expect(sanitizeLine("OPENAI_API_KEY=sk-abcdef0123456789")).toBe(
        "OPENAI_API_KEY=[REDACTED]",
      );
      expect(sanitizeLine("TELEGRAM_BOT_TOKEN=123:AAH_secret")).toBe(
        "TELEGRAM_BOT_TOKEN=[REDACTED]",
      );
    });

    test("redacts -e docker-style env injections", () => {
      expect(sanitizeLine("podman run -e ANTHROPIC_API_KEY=sk-ant-xyz image")).toContain(
        "-e ANTHROPIC_API_KEY=[REDACTED]",
      );
      expect(sanitizeLine("-e TELEGRAM_BOT_TOKEN=99:zzz")).toBe(
        "-e TELEGRAM_BOT_TOKEN=[REDACTED]",
      );
    });

    test("leaves a line with no secrets untouched", () => {
      expect(sanitizeLine("Building image layer 3/8...")).toBe("Building image layer 3/8...");
    });
  });

  describe("formatElapsed", () => {
    test("seconds under a minute", () => {
      expect(formatElapsed(0)).toBe("0s");
      expect(formatElapsed(45_000)).toBe("45s");
      expect(formatElapsed(59_999)).toBe("59s");
    });
    test("minutes and seconds at/over a minute", () => {
      expect(formatElapsed(60_000)).toBe("1m 0s");
      expect(formatElapsed(125_000)).toBe("2m 5s");
    });
  });

  describe("estimateRemaining", () => {
    test("sums pending-step estimates", () => {
      // All four steps pending → 2000+30000+120000+20000 = 172000.
      expect(estimateRemaining(INITIAL_STEPS)).toBe(172_000);
    });
    test("succeeded steps contribute nothing", () => {
      const steps: SubStep[] = [];
      for (const s of INITIAL_STEPS) steps.push(step({ ...s, status: "succeeded" }));
      expect(estimateRemaining(steps)).toBeNull();
    });
    test("a running step counts only its remaining estimate, never negative", () => {
      const longAgo = Date.now() - 999_999_999;
      const steps = [step({ id: "check", status: "running", startedAt: longAgo })];
      // Elapsed far exceeds the 2s estimate → clamped to 0 → overall null.
      expect(estimateRemaining(steps)).toBeNull();
    });
  });

  describe("detectPlatform", () => {
    test("maps userAgent substrings to platforms", () => {
      setUA("Mozilla/5.0 (Macintosh; Intel Mac OS X)");
      expect(detectPlatform()).toBe("mac");
      setUA("Mozilla/5.0 (X11; Linux x86_64)");
      expect(detectPlatform()).toBe("linux");
      setUA("Mozilla/5.0 (Windows NT 10.0; Win64)");
      expect(detectPlatform()).toBe("windows");
      setUA("Some Unknown Console");
      expect(detectPlatform()).toBe("other");
    });
  });

  describe("prefetchTelegramUrl", () => {
    beforeEach(() => {
      mockReadEnv.mockReset();
      mockDeriveBot.mockReset();
    });

    test("no token → clears the bot url/username", async () => {
      mockReadEnv.mockResolvedValue("ANTHROPIC_API_KEY=sk-ant-x\n");
      const update = vi.fn().mockResolvedValue(undefined);
      await prefetchTelegramUrl(update);
      expect(update).toHaveBeenCalledWith({ telegramBotUrl: null, telegramBotUsername: null });
      expect(mockDeriveBot).not.toHaveBeenCalled();
    });

    test("token present → derives + sets the bot url/username", async () => {
      mockReadEnv.mockResolvedValue("TELEGRAM_BOT_TOKEN=42:secret\n");
      mockDeriveBot.mockResolvedValue({ url: "https://t.me/x_bot", username: "x_bot" });
      const update = vi.fn().mockResolvedValue(undefined);
      await prefetchTelegramUrl(update);
      expect(mockDeriveBot).toHaveBeenCalledWith("42:secret");
      expect(update).toHaveBeenCalledWith({
        telegramBotUrl: "https://t.me/x_bot",
        telegramBotUsername: "x_bot",
      });
    });

    test("a downstream error is swallowed and clears the url/username", async () => {
      // Token present so we get past the inner `.catch`, then derive throws →
      // exercises the outer try/catch.
      mockReadEnv.mockResolvedValue("TELEGRAM_BOT_TOKEN=42:secret\n");
      mockDeriveBot.mockRejectedValue(new Error("boom"));
      const update = vi.fn().mockResolvedValue(undefined);
      await expect(prefetchTelegramUrl(update)).resolves.toBeUndefined();
      expect(update).toHaveBeenLastCalledWith({ telegramBotUrl: null, telegramBotUsername: null });
    });
  });
});
