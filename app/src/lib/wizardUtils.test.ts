import { describe, expect, it, vi } from "vitest";

import {
  identifyPastedKey,
  isAnthropicKeyLike,
  isTelegramTokenLike,
  maskKey,
  parseEnvKeys,
  upsertEnvVar,
  withRetry,
} from "./wizardUtils";

describe("identifyPastedKey", () => {
  it("classifies Anthropic keys", () => {
    expect(identifyPastedKey("sk-ant-api03-abc123def")).toBe("anthropic");
    expect(identifyPastedKey("  sk-ant-whatever  ")).toBe("anthropic");
  });

  it("classifies Telegram tokens", () => {
    expect(
      identifyPastedKey("1234567890:ABCdef_GHI-jklMNOpqrstuvwxyz1234567890"),
    ).toBe("telegram");
  });

  it("returns null for ambiguous input", () => {
    expect(identifyPastedKey("")).toBeNull();
    expect(identifyPastedKey("hello world")).toBeNull();
    // Too short to be a Telegram token
    expect(identifyPastedKey("12345:abc")).toBeNull();
    // Missing sk-ant- prefix
    expect(identifyPastedKey("sk-proj-1234567890abcdef")).toBeNull();
  });
});

describe("isAnthropicKeyLike / isTelegramTokenLike", () => {
  it("accepts well-formed Anthropic keys", () => {
    expect(isAnthropicKeyLike("sk-ant-api03-ABCdef1234567890xyz")).toBe(true);
  });

  it("rejects short or mismatched input", () => {
    expect(isAnthropicKeyLike("sk-ant-")).toBe(false);
    expect(isAnthropicKeyLike("not-a-key")).toBe(false);
  });

  it("accepts well-formed Telegram tokens", () => {
    expect(isTelegramTokenLike("123456:ABCdef1234567890_ghi-jkl")).toBe(true);
  });

  it("rejects invalid Telegram shapes", () => {
    expect(isTelegramTokenLike("no-colon-here")).toBe(false);
    expect(isTelegramTokenLike("abc:123")).toBe(false);
  });
});

describe("maskKey", () => {
  it("returns an empty string for empty input", () => {
    expect(maskKey("")).toBe("");
  });

  it("masks very short values entirely", () => {
    expect(maskKey("abcd")).toBe("••••");
  });

  it("shows the last 4 characters of longer secrets", () => {
    expect(maskKey("sk-ant-api03-ABCDEFGHIJKL1234")).toBe("••••••••1234");
  });
});

describe("parseEnvKeys", () => {
  it("extracts both keys from a typical .env", () => {
    const env = `
# Comment
ANTHROPIC_API_KEY=sk-ant-real-key
TELEGRAM_BOT_TOKEN=1234567890:ABCdef
OTHER_VAR=ignored
`;
    expect(parseEnvKeys(env)).toEqual({
      anthropicKey: "sk-ant-real-key",
      telegramToken: "1234567890:ABCdef",
    });
  });

  it("ignores REPLACE placeholders", () => {
    const env = `ANTHROPIC_API_KEY=REPLACE_ME
TELEGRAM_BOT_TOKEN=REPLACE_WITH_YOUR_TOKEN`;
    expect(parseEnvKeys(env)).toEqual({
      anthropicKey: null,
      telegramToken: null,
    });
  });

  it("strips surrounding quotes from values", () => {
    const env = `ANTHROPIC_API_KEY="sk-ant-quoted"
TELEGRAM_BOT_TOKEN='1234:ABCdef'`;
    expect(parseEnvKeys(env)).toEqual({
      anthropicKey: "sk-ant-quoted",
      telegramToken: "1234:ABCdef",
    });
  });

  it("returns nulls for empty input", () => {
    expect(parseEnvKeys("")).toEqual({
      anthropicKey: null,
      telegramToken: null,
    });
  });
});

describe("upsertEnvVar", () => {
  it("adds a new variable when absent", () => {
    expect(upsertEnvVar("", "FOO", "bar")).toBe("\nFOO=bar\n");
  });

  it("updates an existing variable in place", () => {
    const result = upsertEnvVar("FOO=old\nBAR=keep", "FOO", "new");
    expect(result).toContain("FOO=new");
    expect(result).toContain("BAR=keep");
    expect(result).not.toContain("FOO=old");
  });

  it("preserves surrounding comments and other vars", () => {
    const env = `# Header
ANTHROPIC_API_KEY=placeholder
# Middle
TELEGRAM_BOT_TOKEN=oldtoken`;
    const result = upsertEnvVar(env, "TELEGRAM_BOT_TOKEN", "newtoken");
    expect(result).toContain("# Header");
    expect(result).toContain("# Middle");
    expect(result).toContain("TELEGRAM_BOT_TOKEN=newtoken");
  });
});

describe("withRetry", () => {
  it("returns immediately on first success without retry", async () => {
    const fn = vi.fn().mockResolvedValue("ok");
    const result = await withRetry(fn, 2);
    expect(result).toBe("ok");
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("retries up to the max and succeeds on the last attempt", async () => {
    vi.useFakeTimers();
    const fn = vi
      .fn()
      .mockRejectedValueOnce(new Error("transient 1"))
      .mockRejectedValueOnce(new Error("transient 2"))
      .mockResolvedValueOnce("finally");

    const promise = withRetry(fn, 2);
    // First attempt runs synchronously; let the microtask resolve.
    await vi.advanceTimersByTimeAsync(0);
    // Backoff between attempt 1 and 2: 2000ms.
    await vi.advanceTimersByTimeAsync(2000);
    // Backoff between attempt 2 and 3: 4000ms.
    await vi.advanceTimersByTimeAsync(4000);

    const result = await promise;
    expect(result).toBe("finally");
    expect(fn).toHaveBeenCalledTimes(3);
    vi.useRealTimers();
  });

  it("throws the last error after exhausting retries", async () => {
    vi.useFakeTimers();
    const err = new Error("always fails");
    const fn = vi.fn().mockRejectedValue(err);

    const promise = withRetry(fn, 2).catch((error: unknown) => error);
    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(2000);
    await vi.advanceTimersByTimeAsync(4000);

    expect(await promise).toBe(err);
    expect(fn).toHaveBeenCalledTimes(3);
    vi.useRealTimers();
  });

  it("invokes onRetry with the attempt index for each retry", async () => {
    vi.useFakeTimers();
    const onRetry = vi.fn();
    const fn = vi
      .fn()
      .mockRejectedValueOnce(new Error("x"))
      .mockRejectedValueOnce(new Error("y"))
      .mockResolvedValueOnce("ok");

    const promise = withRetry(fn, 2, onRetry);
    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(2000);
    await vi.advanceTimersByTimeAsync(4000);
    await promise;

    expect(onRetry).toHaveBeenCalledTimes(2);
    expect(onRetry).toHaveBeenNthCalledWith(1, 1);
    expect(onRetry).toHaveBeenNthCalledWith(2, 2);
    vi.useRealTimers();
  });
});
