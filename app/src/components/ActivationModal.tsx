import { open as openUrl } from "@tauri-apps/plugin-shell";
import { Check, Eye, EyeOff, Key, MessageCircle, X } from "lucide-react";
import { useEffect, useRef, useState, type ClipboardEvent } from "react";

import HowToModal, { type HowToStep } from "@/components/wizard/HowToModal";
import { useSettings } from "@/hooks/useSettings";
import { classifyError } from "@/lib/errors";
import {
  commitActivation,
  deriveTelegramBotUrl,
  readConfig,
  telegramAdvanceOffset,
  telegramDeleteWebhook,
  telegramPollForStart,
  telegramSendMessage,
  validateAnthropicKey,
  writeConfig,
  type TelegramUpdate,
} from "@/lib/tauri";
import { isAnthropicKeyLike, isTelegramTokenLike, upsertEnvVar } from "@/lib/wizardUtils";

// ─── How-to step content ──────────────────────────────────────────────────

const ANTHROPIC_STEPS: HowToStep[] = [
  {
    heading: "Open the Anthropic console",
    body: "Go to console.anthropic.com and sign in.",
    href: "https://console.anthropic.com/",
  },
  { heading: "Go to API Keys", body: "In the left menu, click Settings → API Keys." },
  {
    heading: "Create a new key",
    body: 'Click Create Key. Give it a name like "Lobster-TrApp".',
  },
  {
    heading: "Copy the key immediately",
    body: "Anthropic shows it once. It starts with sk-ant-. Copy it now.",
  },
];

const TELEGRAM_STEPS: HowToStep[] = [
  {
    heading: "Open Telegram and find BotFather",
    body: "Search for @BotFather. It has a blue checkmark.",
    href: "https://t.me/BotFather",
  },
  { heading: "Send /newbot", body: "BotFather will ask for a display name and a username." },
  {
    heading: "Pick a name and username",
    body: 'Username must end in "bot" (e.g. my_assistant_bot).',
  },
  { heading: "Copy the token", body: "BotFather sends a token like 1234567890:ABCdef…" },
];

// ─── Anthropic error copy ─────────────────────────────────────────────────

const ANTHROPIC_ERRORS: Record<string, string> = {
  auth_failure:
    "That key isn't being accepted. Double-check it's the latest one from console.anthropic.com.",
  billing:
    "Looks like there's an issue with your account. Check console.anthropic.com/billing.",
  permission:
    "Looks like there's an issue with your account. Check console.anthropic.com/billing.",
  rate: "Anthropic is rate-limiting right now. Wait a moment and try again.",
  server_error: "Anthropic's having a moment. Try again in a few seconds.",
  unknown: "Unexpected response. Try again.",
  network: "Can't reach Anthropic. Check your internet connection.",
};

// ─── Types ────────────────────────────────────────────────────────────────

type FlowStep = "anthropic" | "telegram" | "committing";

type AnthropicPhase = "idle" | "validating" | "valid" | "error";

type TelegramPhase =
  | "idle"
  | "validating"
  | "deep_link"
  | "polling"
  | "timed_out"
  | "sending"
  | "test_sent"
  | "error";

interface Props {
  onClose: () => void;
  /**
   * When true, the user has a valid Telegram token from a prior install but
   * needs to re-enter their Anthropic key (e.g. key was revoked). Step 2
   * is skipped — the existing token is read from .env and reused.
   */
  reCredential?: boolean;
}

// ─── Component ────────────────────────────────────────────────────────────

export default function ActivationModal({ onClose, reCredential = false }: Props) {
  const { update: updateSettings } = useSettings();

  // Flow step
  const [step, setStep] = useState<FlowStep>("anthropic");

  // Anthropic state
  const [anthropicKey, setAnthropicKey] = useState("");
  const [showAnthropicKey, setShowAnthropicKey] = useState(false);
  const [anthropicPhase, setAnthropicPhase] = useState<AnthropicPhase>("idle");
  const [anthropicErrorKey, setAnthropicErrorKey] = useState<string | null>(null);

  // Telegram state
  const [telegramToken, setTelegramToken] = useState("");
  const [showTelegramToken, setShowTelegramToken] = useState(false);
  const [telegramPhase, setTelegramPhase] = useState<TelegramPhase>("idle");
  const [telegramError, setTelegramError] = useState<string | null>(null);
  const [botUsername, setBotUsername] = useState<string | null>(null);
  const [botUrl, setBotUrl] = useState<string | null>(null);
  const [pollOffset] = useState(0);
  const [pollElapsed, setPollElapsed] = useState(0);
  const [pendingUpdate, setPendingUpdate] = useState<TelegramUpdate | null>(null);

  // Refs to avoid stale closures inside polling effect
  const pollOffsetRef = useRef(0);
  const pollElapsedRef = useRef(0);
  const telegramTokenRef = useRef("");
  telegramTokenRef.current = telegramToken;

  // How-to modals
  const [howToOpen, setHowToOpen] = useState<"anthropic" | "telegram" | null>(null);

  // Commit error
  const [commitError, setCommitError] = useState<string | null>(null);

  // ESC to close
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  // Re-credential mode: load the existing Telegram token from .env so we can
  // reuse it in the commit without forcing the user through Step 2 again.
  useEffect(() => {
    if (!reCredential) return;
    let cancelled = false;
    void (async () => {
      try {
        const content = await readConfig("openclaw-vault", ".env");
        if (cancelled) return;
        // Extract TELEGRAM_BOT_TOKEN line
        for (const line of content.split("\n")) {
          const trimmed = line.trim();
          if (trimmed.startsWith("TELEGRAM_BOT_TOKEN=")) {
            const val = trimmed.slice("TELEGRAM_BOT_TOKEN=".length)
              .trim().replace(/^['"]|['"]$/g, "");
            if (val && !val.includes("REPLACE") && val.length >= 8) {
              setTelegramToken(val);
              break;
            }
          }
        }
      } catch {
        // .env not readable; user will need to re-enter both keys
      }
    })();
    return () => { cancelled = true; };
  }, [reCredential]);

  // ─── Anthropic validation ────────────────────────────────────────────

  async function handleValidateAnthropic() {
    if (!isAnthropicKeyLike(anthropicKey)) return;
    setAnthropicPhase("validating");
    setAnthropicErrorKey(null);
    try {
      const outcome = await validateAnthropicKey(anthropicKey);
      if (outcome === "ok") {
        setAnthropicPhase("valid");
      } else {
        setAnthropicPhase("error");
        setAnthropicErrorKey(outcome);
      }
    } catch {
      setAnthropicPhase("error");
      setAnthropicErrorKey("network");
    }
  }

  function handleAnthropicPaste(e: ClipboardEvent<HTMLInputElement>) {
    const pasted = e.clipboardData.getData("text").trim();
    if (isAnthropicKeyLike(pasted)) {
      setAnthropicKey(pasted);
      setAnthropicPhase("idle");
    }
  }

  // ─── Telegram validation (getMe) ─────────────────────────────────────

  async function handleValidateTelegram() {
    if (!isTelegramTokenLike(telegramToken)) return;
    setTelegramPhase("validating");
    setTelegramError(null);
    try {
      const bot = await deriveTelegramBotUrl(telegramToken);
      setBotUsername(bot.username);
      setBotUrl(bot.url);
      // Delete any leftover webhook before we start polling.
      await telegramDeleteWebhook(telegramToken);
      setTelegramPhase("deep_link");
    } catch (e) {
      setTelegramPhase("error");
      setTelegramError(classifyError(e).userMessage);
    }
  }

  function handleTelegramPaste(e: ClipboardEvent<HTMLInputElement>) {
    const pasted = e.clipboardData.getData("text").trim();
    if (isTelegramTokenLike(pasted)) {
      setTelegramToken(pasted);
      setTelegramPhase("idle");
    }
  }

  // ─── Polling effect (runs while telegramPhase === "deep_link") ────────

  useEffect(() => {
    if (telegramPhase !== "deep_link") return;

    let cancelled = false;
    pollOffsetRef.current = pollOffset;
    pollElapsedRef.current = pollElapsed;

    const token = telegramTokenRef.current;

    void (async () => {
      while (!cancelled) {
        if (pollElapsedRef.current >= 90) {
          if (!cancelled) {
            setTelegramPhase("timed_out");
            setPollElapsed(pollElapsedRef.current);
          }
          return;
        }

        try {
          const update = await telegramPollForStart(token, pollOffsetRef.current, 30);
          if (cancelled) return;

          if (update !== null) {
            // Found a /start — send the test message.
            setPendingUpdate(update);
            setTelegramPhase("sending");
            try {
              await telegramSendMessage(
                token,
                update.chat_id,
                "Hi! I'm your new assistant. I'm working.",
              );
              if (!cancelled) setTelegramPhase("test_sent");
            } catch (e) {
              if (!cancelled) {
                const msg = String(e);
                setTelegramError(
                  msg === "conflict"
                    ? "Another instance of this bot is active. Make sure you're using a fresh bot token from BotFather."
                    : "Couldn't send the test message. Try again.",
                );
                setTelegramPhase("error");
              }
            }
            return;
          } else {
            // Timeout expired without /start.
            pollElapsedRef.current += 30;
            if (!cancelled) setPollElapsed(pollElapsedRef.current);
          }
        } catch (e) {
          if (!cancelled) {
            const msg = String(e);
            setTelegramError(
              msg === "conflict"
                ? "Another instance of this bot is active. Make sure you're using a fresh bot token from BotFather."
                : classifyError(e).userMessage,
            );
            setTelegramPhase("error");
          }
          return;
        }
      }
    })();

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: only re-run when phase transitions to deep_link
  }, [telegramPhase]);

  // ─── Commit ───────────────────────────────────────────────────────────

  async function handleCommit(skipTelegramTest: boolean) {
    setStep("committing");
    setCommitError(null);

    // Advance the Telegram offset so vault-agent doesn't replay /start.
    if (!skipTelegramTest && pendingUpdate) {
      try {
        await telegramAdvanceOffset(telegramToken, pendingUpdate.update_id);
      } catch {
        // Non-fatal — vault-agent handles duplicate /start gracefully.
      }
    }

    // Write both keys to .env transactionally.
    try {
      let envContent = "";
      try {
        envContent = await readConfig("openclaw-vault", ".env");
      } catch {
        envContent = "# OpenClaw-Vault configuration\n";
      }
      envContent = upsertEnvVar(envContent, "ANTHROPIC_API_KEY", anthropicKey);
      envContent = upsertEnvVar(envContent, "TELEGRAM_BOT_TOKEN", telegramToken);
      await writeConfig("openclaw-vault", ".env", envContent);
    } catch (e) {
      setCommitError("Couldn't save your keys: " + classifyError(e).userMessage);
      setStep("telegram");
      return;
    }

    // Restart proxy + start agent + write markers.
    try {
      await commitActivation();
    } catch (e) {
      setCommitError(classifyError(e).userMessage);
      setStep("telegram");
      return;
    }

    // Mark setup complete for legacy compatibility.
    await updateSettings({ wizardCompleted: true });

    onClose();
  }

  // ─── Render ───────────────────────────────────────────────────────────

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      role="dialog"
      aria-modal="true"
      aria-label="Launch your assistant"
    >
      <div className="relative w-full max-w-lg rounded-xl bg-neutral-900 p-6 shadow-2xl ring-1 ring-neutral-700">
        {/* Close button */}
        <button
          type="button"
          onClick={onClose}
          className="absolute right-4 top-4 rounded p-1 text-neutral-500 hover:text-neutral-300"
          aria-label="Cancel"
        >
          <X size={18} />
        </button>

        {/* Step indicator */}
        <div className="mb-6 flex items-center gap-2">
          <StepDot active={step === "anthropic"} done={step === "telegram" || step === "committing"} label="1" />
          <div className="h-px flex-1 bg-neutral-700" />
          <StepDot active={step === "telegram"} done={step === "committing"} label="2" />
        </div>

        {step === "anthropic" && (
          <AnthropicStep
            value={anthropicKey}
            onChange={(v) => { setAnthropicKey(v); setAnthropicPhase("idle"); }}
            onPaste={handleAnthropicPaste}
            show={showAnthropicKey}
            toggleShow={() => setShowAnthropicKey((v) => !v)}
            phase={anthropicPhase}
            errorKey={anthropicErrorKey}
            onValidate={() => void handleValidateAnthropic()}
            onContinue={() => {
              if (reCredential && telegramToken) {
                // Skip Step 2 — existing Telegram token is already loaded.
                void handleCommit(true);
              } else {
                setStep("telegram");
              }
            }}
            continueLabel={reCredential && telegramToken ? "Launch my assistant" : "Continue"}
            onHowTo={() => setHowToOpen("anthropic")}
          />
        )}

        {step === "telegram" && (
          <TelegramStep
            value={telegramToken}
            onChange={(v) => { setTelegramToken(v); setTelegramPhase("idle"); }}
            onPaste={handleTelegramPaste}
            show={showTelegramToken}
            toggleShow={() => setShowTelegramToken((v) => !v)}
            phase={telegramPhase}
            error={telegramError}
            botUsername={botUsername}
            botUrl={botUrl}
            pollElapsed={pollElapsed}
            onValidate={() => void handleValidateTelegram()}
            onOpenBot={async () => {
              try {
                await openUrl(botUrl ?? "https://telegram.org");
              } catch {
                window.open(botUrl ?? "https://telegram.org", "_blank", "noopener,noreferrer");
              }
            }}
            onWaitMore={() => {
              // Reset elapsed and re-enter deep_link phase so the polling effect restarts.
              pollElapsedRef.current = 0;
              setPollElapsed(0);
              setTelegramPhase("polling");
              // Micro-delay so the effect dependency sees a phase change.
              setTimeout(() => setTelegramPhase("deep_link"), 0);
            }}
            onSkip={() => void handleCommit(true)}
            onConfirm={() => void handleCommit(false)}
            onRetry={() => {
              setTelegramPhase("idle");
              setTelegramError(null);
            }}
            onHowTo={() => setHowToOpen("telegram")}
            commitError={commitError}
          />
        )}

        {step === "committing" && (
          <div className="flex flex-col items-center gap-4 py-8">
            <div className="h-10 w-10 animate-spin rounded-full border-4 border-primary-500/30 border-t-primary-500" />
            <p className="text-sm text-neutral-400">Launching your assistant…</p>
          </div>
        )}
      </div>

      <HowToModal
        open={howToOpen === "anthropic"}
        onClose={() => setHowToOpen(null)}
        title="How to get an Anthropic API key"
        steps={ANTHROPIC_STEPS}
      />
      <HowToModal
        open={howToOpen === "telegram"}
        onClose={() => setHowToOpen(null)}
        title="How to create a Telegram bot"
        steps={TELEGRAM_STEPS}
      />
    </div>
  );
}

// ─── Sub-components ───────────────────────────────────────────────────────

function StepDot({
  active,
  done,
  label,
}: {
  active: boolean;
  done: boolean;
  label: string;
}) {
  return (
    <div
      className={`flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold transition-colors ${
        done
          ? "bg-success-500 text-white"
          : active
            ? "bg-primary-500 text-white"
            : "bg-neutral-700 text-neutral-400"
      }`}
    >
      {done ? <Check size={13} /> : label}
    </div>
  );
}

interface AnthropicStepProps {
  value: string;
  onChange: (v: string) => void;
  onPaste: (e: ClipboardEvent<HTMLInputElement>) => void;
  show: boolean;
  toggleShow: () => void;
  phase: AnthropicPhase;
  errorKey: string | null;
  onValidate: () => void;
  onContinue: () => void;
  continueLabel?: string;
  onHowTo: () => void;
}

function AnthropicStep({
  value, onChange, onPaste, show, toggleShow,
  phase, errorKey, onValidate, onContinue, continueLabel = "Continue", onHowTo,
}: AnthropicStepProps) {
  const formatOk = isAnthropicKeyLike(value);
  const canValidate = formatOk && phase !== "validating" && phase !== "valid";

  return (
    <div>
      <div className="mb-5 flex items-center gap-2">
        <Key size={18} className="text-primary-400" />
        <h2 className="text-lg font-semibold text-neutral-100">Anthropic API key</h2>
        {phase === "valid" && <Check size={16} className="text-success-400" />}
      </div>
      <p className="mb-4 text-sm text-neutral-400">
        Your assistant's brain — also how you'll pay for its thoughts (~$5–20/month for typical use).
      </p>

      <div className="relative mb-1">
        <input
          type={show ? "text" : "password"}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onPaste={onPaste}
          placeholder="sk-ant-api03-…"
          autoComplete="off"
          className="input pr-10"
        />
        <button
          type="button"
          aria-label={show ? "Hide key" : "Show key"}
          onClick={toggleShow}
          className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-neutral-500 hover:text-neutral-300"
        >
          {show ? <EyeOff size={16} /> : <Eye size={16} />}
        </button>
      </div>

      <p className="mb-4 text-xs text-neutral-600">
        Your key is stored in plain text on this computer. We're working on encrypted storage for a future release.
      </p>

      {phase === "error" && errorKey && (
        <p className="mb-4 rounded-md bg-danger-500/10 px-3 py-2 text-sm text-danger-400">
          {ANTHROPIC_ERRORS[errorKey] ?? ANTHROPIC_ERRORS["unknown"]}
        </p>
      )}

      <p className="mb-6 text-xs text-neutral-500">
        Don't have one yet?{" "}
        <button
          type="button"
          onClick={onHowTo}
          className="text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
        >
          Show me how to get one (2 min)
        </button>
      </p>

      <div className="flex items-center justify-end gap-3">
        {phase !== "valid" && (
          <button
            type="button"
            onClick={onValidate}
            className="btn btn-md btn-primary"
            disabled={!canValidate}
          >
            {phase === "validating" ? "Checking…" : "Validate key"}
          </button>
        )}
        {phase === "valid" && (
          <button type="button" onClick={onContinue} className="btn btn-md btn-primary">
            {continueLabel}
          </button>
        )}
      </div>
    </div>
  );
}

interface TelegramStepProps {
  value: string;
  onChange: (v: string) => void;
  onPaste: (e: ClipboardEvent<HTMLInputElement>) => void;
  show: boolean;
  toggleShow: () => void;
  phase: TelegramPhase;
  error: string | null;
  botUsername: string | null;
  botUrl: string | null;
  pollElapsed: number;
  onValidate: () => void;
  onOpenBot: () => void;
  onWaitMore: () => void;
  onSkip: () => void;
  onConfirm: () => void;
  onRetry: () => void;
  onHowTo: () => void;
  commitError: string | null;
}

function TelegramStep({
  value, onChange, onPaste, show, toggleShow,
  phase, error, botUsername, botUrl, pollElapsed,
  onValidate, onOpenBot, onWaitMore, onSkip, onConfirm, onRetry, onHowTo,
  commitError,
}: TelegramStepProps) {
  const formatOk = isTelegramTokenLike(value);
  const canValidate = formatOk && phase !== "validating" && phase !== "deep_link" &&
    phase !== "polling" && phase !== "sending" && phase !== "test_sent";

  return (
    <div>
      <div className="mb-5 flex items-center gap-2">
        <MessageCircle size={18} className="text-info-400" />
        <h2 className="text-lg font-semibold text-neutral-100">Telegram bot</h2>
      </div>
      <p className="mb-4 text-sm text-neutral-400">
        How you'll talk to your assistant on your phone.
      </p>

      {(phase === "idle" || phase === "validating" || phase === "error") && (
        <>
          <div className="relative mb-4">
            <input
              type={show ? "text" : "password"}
              value={value}
              onChange={(e) => onChange(e.target.value)}
              onPaste={onPaste}
              placeholder="1234567890:ABCdefGHIjkl…"
              autoComplete="off"
              className="input pr-10"
            />
            <button
              type="button"
              aria-label={show ? "Hide token" : "Show token"}
              onClick={toggleShow}
              className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-neutral-500 hover:text-neutral-300"
            >
              {show ? <EyeOff size={16} /> : <Eye size={16} />}
            </button>
          </div>

          {phase === "error" && error && (
            <p className="mb-4 rounded-md bg-danger-500/10 px-3 py-2 text-sm text-danger-400">
              {error}
            </p>
          )}

          <p className="mb-6 text-xs text-neutral-500">
            Need to create one?{" "}
            <button
              type="button"
              onClick={onHowTo}
              className="text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
            >
              Walk me through it (3 min)
            </button>
          </p>

          <div className="flex items-center justify-between">
            {phase === "error" && (
              <button type="button" onClick={onRetry} className="btn btn-md btn-ghost">
                Change token
              </button>
            )}
            <div className="ml-auto flex gap-3">
              <button
                type="button"
                onClick={onValidate}
                className="btn btn-md btn-primary"
                disabled={!canValidate}
              >
                {phase === "validating" ? "Checking…" : "Validate bot"}
              </button>
            </div>
          </div>
        </>
      )}

      {(phase === "deep_link" || phase === "polling" || phase === "timed_out") && (
        <div className="text-center">
          <p className="mb-4 text-sm text-neutral-300">
            Open your bot in Telegram and tap <strong>Start</strong>.
          </p>
          {botUrl && (
            <button
              type="button"
              onClick={onOpenBot}
              className="btn btn-md btn-primary mb-6"
            >
              <MessageCircle size={16} />
              Open @{botUsername} in Telegram
            </button>
          )}

          {phase === "timed_out" ? (
            <div>
              <p className="mb-4 text-sm text-neutral-500">
                Still waiting for your /start in Telegram.
              </p>
              <div className="flex justify-center gap-3">
                <button type="button" onClick={onSkip} className="btn btn-md btn-ghost">
                  Skip and test later
                </button>
                <button type="button" onClick={onWaitMore} className="btn btn-md btn-primary">
                  Wait another 90s
                </button>
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center gap-3">
              <div className="flex items-center gap-2 text-xs text-neutral-500">
                <span className="inline-block h-2 w-2 animate-pulse rounded-full bg-primary-500" />
                Waiting for your /start{pollElapsed > 0 ? ` (${pollElapsed}s)` : ""}…
              </div>
              <button type="button" onClick={onSkip} className="btn btn-sm btn-ghost">
                Skip and test later
              </button>
            </div>
          )}
        </div>
      )}

      {phase === "sending" && (
        <div className="flex flex-col items-center gap-3 py-4">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary-500/30 border-t-primary-500" />
          <p className="text-sm text-neutral-400">Sending test message…</p>
        </div>
      )}

      {phase === "test_sent" && (
        <div className="text-center">
          <p className="mb-6 text-sm text-neutral-300">
            Did you see the message from your bot in Telegram?
          </p>
          {commitError && (
            <p className="mb-4 rounded-md bg-danger-500/10 px-3 py-2 text-sm text-danger-400">
              {commitError}
            </p>
          )}
          <div className="flex justify-center gap-3">
            <button type="button" onClick={onSkip} className="btn btn-md btn-ghost">
              No, skip for now
            </button>
            <button type="button" onClick={onConfirm} className="btn btn-md btn-primary">
              Yes, launch my assistant
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
