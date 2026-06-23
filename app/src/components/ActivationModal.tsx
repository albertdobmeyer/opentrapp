/* eslint-disable max-lines */
import { Check, Eye, EyeOff, Key, MessageCircle, X } from "lucide-react";
import { useEffect, useRef, useState, type ClipboardEvent } from "react";

import HowToModal, { type HowToStep } from "@/components/wizard/HowToModal";
import { useSettings } from "@/hooks/useSettings";
import { classifyError } from "@/lib/errors";
import { openUrl } from "@/lib/shell";
import {
  commitActivation,
  deriveTelegramBotUrl,
  readRuntimeEnv,
  saveCredentials,
  telegramAdvanceOffset,
  telegramDeleteWebhook,
  telegramPollForStart,
  telegramSendMessage,
  validateAnthropicKey,
  type TelegramUpdate,
} from "@/lib/tauri";
import { isAnthropicKeyLike, isTelegramTokenLike } from "@/lib/wizardUtils";

// ─── How-to step content ──────────────────────────────────────────────────

const ANTHROPIC_STEPS: HowToStep[] = [
  {
    heading: "Open the Anthropic console",
    body: "Go to console.anthropic.com and sign in.",
    href: "https://console.anthropic.com/",
  },
  { heading: "Go to API Keys", body: "In the left menu, click Settings → API Keys." },
  { heading: "Create a new key", body: 'Click Create Key. Give it a name like "OpenTrApp".' },
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
  billing: "Looks like there's an issue with your account. Check console.anthropic.com/billing.",
  permission: "Looks like there's an issue with your account. Check console.anthropic.com/billing.",
  rate: "Anthropic is rate-limiting right now. Wait a moment and try again.",
  server_error: "Anthropic's having a moment. Try again in a few seconds.",
  unknown: "Unexpected response. Try again.",
  network: "Can't reach Anthropic. Check your internet connection.",
};

// ─── Types ────────────────────────────────────────────────────────────────

type FlowStep = "anthropic" | "telegram" | "committing";
type AnthropicPhase = "idle" | "validating" | "valid" | "error";
type TelegramPhase =
  | "idle" | "validating" | "deep_link" | "polling"
  | "timed_out" | "sending" | "test_sent" | "error";

interface Props {
  onClose: () => void;
  reCredential?: boolean;
}

// ─── Telegram flow hook ───────────────────────────────────────────────────

function useTelegramFlow() {
  const [telegramToken, setTelegramToken] = useState("");
  const [showTelegramToken, setShowTelegramToken] = useState(false);
  const [telegramPhase, setTelegramPhase] = useState<TelegramPhase>("idle");
  const [telegramError, setTelegramError] = useState<string | null>(null);
  const [botUsername, setBotUsername] = useState<string | null>(null);
  const [botUrl, setBotUrl] = useState<string | null>(null);
  const [pollElapsed, setPollElapsed] = useState(0);
  const [pendingUpdate, setPendingUpdate] = useState<TelegramUpdate | null>(null);
  const pollOffsetRef = useRef(0);
  const pollElapsedRef = useRef(0);
  const telegramTokenRef = useRef("");
  const cancelRef = useRef<boolean>(false);
  telegramTokenRef.current = telegramToken;

  useEffect(() => {
    if (telegramPhase !== "deep_link") return;
    cancelRef.current = false;
    pollOffsetRef.current = 0;
    pollElapsedRef.current = 0;
    const token = telegramTokenRef.current;
    void (async () => {
      for (;;) {
        if (cancelRef.current) break;
        if (pollElapsedRef.current >= 90) {
          setTelegramPhase("timed_out");
          setPollElapsed(pollElapsedRef.current);
          break;
        }
        try {
          const update = await telegramPollForStart(token, pollOffsetRef.current, 30);
          // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
          if (cancelRef.current) break;
          if (update !== null) {
            setPendingUpdate(update);
            setTelegramPhase("sending");
            try {
              await telegramSendMessage(token, update.chat_id, "Hi! I'm your new assistant. I'm working.");
              // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
              if (!cancelRef.current) setTelegramPhase("test_sent");
            } catch (error) {
              // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
              if (!cancelRef.current) {
                const msg = String(error);
                setTelegramError(
                  msg === "conflict"
                    ? "Another instance of this bot is active. Make sure you're using a fresh bot token from BotFather."
                    : "Couldn't send the test message. Try again.",
                );
                setTelegramPhase("error");
              }
            }
            break;
          }
          pollElapsedRef.current += 30;
          // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
          if (!cancelRef.current) setPollElapsed(pollElapsedRef.current);
        } catch (error) {
          // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
          if (!cancelRef.current) {
            const msg = String(error);
            setTelegramError(
              msg === "conflict"
                ? "Another instance of this bot is active. Make sure you're using a fresh bot token from BotFather."
                : classifyError(error).userMessage,
            );
            setTelegramPhase("error");
          }
          break;
        }
      }
    })();
    return () => { cancelRef.current = true; };
  }, [telegramPhase]);

  async function handleValidateTelegram() {
    if (!isTelegramTokenLike(telegramToken)) return;
    setTelegramPhase("validating");
    setTelegramError(null);
    try {
      const bot = await deriveTelegramBotUrl(telegramToken);
      setBotUsername(bot.username);
      setBotUrl(bot.url);
      await telegramDeleteWebhook(telegramToken);
      setTelegramPhase("deep_link");
    } catch (error) {
      setTelegramPhase("error");
      setTelegramError(classifyError(error).userMessage);
    }
  }

  function handleTelegramPaste(e: ClipboardEvent<HTMLInputElement>) {
    const pasted = e.clipboardData.getData("text").trim();
    if (isTelegramTokenLike(pasted)) {
      setTelegramToken(pasted);
      setTelegramPhase("idle");
    }
  }

  return {
    telegramToken, setTelegramToken, showTelegramToken, setShowTelegramToken,
    telegramPhase, setTelegramPhase, telegramError, setTelegramError,
    botUsername, botUrl, pollElapsed, setPollElapsed,
    pendingUpdate, pollOffsetRef,
    handleValidateTelegram, handleTelegramPaste,
  };
}

// ─── Activation flow hook ─────────────────────────────────────────────────

function useActivationFlow({ onClose, reCredential }: { onClose: () => void; reCredential: boolean }) {
  const { update: updateSettings } = useSettings();
  const [step, setStep] = useState<FlowStep>("anthropic");
  const [anthropicKey, setAnthropicKey] = useState("");
  const [showAnthropicKey, setShowAnthropicKey] = useState(false);
  const [anthropicPhase, setAnthropicPhase] = useState<AnthropicPhase>("idle");
  const [anthropicErrorKey, setAnthropicErrorKey] = useState<string | null>(null);
  const [howToOpen, setHowToOpen] = useState<"anthropic" | "telegram" | null>(null);
  const [commitError, setCommitError] = useState<string | null>(null);
  const telegram = useTelegramFlow();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    window.addEventListener("keydown", handler);
    return () => { window.removeEventListener("keydown", handler); };
  }, [onClose]);

  useEffect(() => {
    if (!reCredential) return;
    let cancelled = false;
    void readRuntimeEnv()
      .then((content) => {
        if (cancelled) return;
        for (const line of content.split("\n")) {
          const trimmed = line.trim();
          if (trimmed.startsWith("TELEGRAM_BOT_TOKEN=")) {
            const val = trimmed.slice("TELEGRAM_BOT_TOKEN=".length)
              .trim().replace(/^["']|["']$/g, "");
            if (val && !val.includes("REPLACE") && val.length >= 8) {
              telegram.setTelegramToken(val);
              break;
            }
          }
        }
      })
      .catch(() => { /* .env not readable */ });
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- setTelegramToken is a stable state setter
  }, [reCredential, telegram.setTelegramToken]);

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

  async function handleCommit(skipTelegramTest: boolean) {
    setStep("committing");
    setCommitError(null);
    if (!skipTelegramTest && telegram.pendingUpdate) {
      try {
        await telegramAdvanceOffset(telegram.telegramToken, telegram.pendingUpdate.update_id);
      } catch { /* non-fatal */ }
    }
    try {
      await saveCredentials(anthropicKey, telegram.telegramToken);
    } catch (error) {
      setCommitError("Couldn't save your keys: " + classifyError(error).userMessage);
      setStep("telegram");
      return;
    }
    try {
      await commitActivation();
    } catch (error) {
      setCommitError(classifyError(error).userMessage);
      setStep("telegram");
      return;
    }
    await updateSettings({ wizardCompleted: true });
    onClose();
  }

  return {
    step, setStep,
    anthropicKey, setAnthropicKey,
    showAnthropicKey, setShowAnthropicKey,
    anthropicPhase, setAnthropicPhase,
    anthropicErrorKey,
    howToOpen, setHowToOpen,
    commitError,
    telegram,
    handleValidateAnthropic,
    handleAnthropicPaste,
    handleCommit,
  };
}

// ─── Component ────────────────────────────────────────────────────────────

export default function ActivationModal({ onClose, reCredential = false }: Props) {
  const flow = useActivationFlow({ onClose, reCredential });
  const { step, anthropicKey, showAnthropicKey, anthropicPhase, anthropicErrorKey,
    howToOpen, setHowToOpen, commitError, telegram,
    setAnthropicKey, setAnthropicPhase, setShowAnthropicKey,
    handleValidateAnthropic, handleAnthropicPaste, handleCommit } = flow;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      role="dialog" aria-modal="true" aria-label="Launch your assistant">
      <div className="relative w-full max-w-lg rounded-xl bg-neutral-900 p-6 shadow-2xl ring-1 ring-neutral-700">
        <button type="button" onClick={onClose}
          className="absolute right-4 top-4 rounded p-1 text-neutral-500 hover:text-neutral-300"
          aria-label="Cancel">
          <X size={18} />
        </button>
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
            toggleShow={() => { setShowAnthropicKey((v) => !v); }}
            phase={anthropicPhase}
            errorKey={anthropicErrorKey}
            onValidate={() => { void handleValidateAnthropic(); }}
            onContinue={() => {
              if (reCredential && telegram.telegramToken) {
                void handleCommit(true);
              } else {
                flow.setStep("telegram");
              }
            }}
            continueLabel={reCredential && telegram.telegramToken ? "Launch my assistant" : "Continue"}
            onHowTo={() => { setHowToOpen("anthropic"); }}
          />
        )}
        {step === "telegram" && (
          <TelegramStep
            value={telegram.telegramToken}
            onChange={(v) => { telegram.setTelegramToken(v); telegram.setTelegramPhase("idle"); }}
            onPaste={telegram.handleTelegramPaste}
            show={telegram.showTelegramToken}
            toggleShow={() => { telegram.setShowTelegramToken((v) => !v); }}
            phase={telegram.telegramPhase}
            error={telegram.telegramError}
            botUsername={telegram.botUsername}
            botUrl={telegram.botUrl}
            pollElapsed={telegram.pollElapsed}
            onValidate={() => { void telegram.handleValidateTelegram(); }}
            onOpenBot={async () => {
              try { await openUrl(telegram.botUrl ?? "https://telegram.org"); }
              catch { window.open(telegram.botUrl ?? "https://telegram.org", "_blank", "noopener,noreferrer"); }
            }}
            onWaitMore={() => {
              telegram.pollOffsetRef.current = 0;
              telegram.setPollElapsed(0);
              telegram.setTelegramPhase("polling");
              setTimeout(() => { telegram.setTelegramPhase("deep_link"); }, 0);
            }}
            onSkip={() => { void handleCommit(true); }}
            onConfirm={() => { void handleCommit(false); }}
            onRetry={() => { telegram.setTelegramPhase("idle"); telegram.setTelegramError(null); }}
            onHowTo={() => { setHowToOpen("telegram"); }}
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
      <HowToModal open={howToOpen === "anthropic"} onClose={() => { setHowToOpen(null); }}
        title="How to get an Anthropic API key" steps={ANTHROPIC_STEPS} />
      <HowToModal open={howToOpen === "telegram"} onClose={() => { setHowToOpen(null); }}
        title="How to create a Telegram bot" steps={TELEGRAM_STEPS} />
    </div>
  );
}

// ─── Sub-components ───────────────────────────────────────────────────────

function StepDot({ active, done, label }: { active: boolean; done: boolean; label: string }) {
  return (
    <div className={`flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold transition-colors ${
      done ? "bg-success-500 text-white" : (active ? "bg-primary-500 text-white" : "bg-neutral-700 text-neutral-400")
    }`}>
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
        Your assistant&rsquo;s brain — also how you&rsquo;ll pay for its thoughts (~$5–20/month for typical use).
      </p>
      <div className="relative mb-1">
        <input type={show ? "text" : "password"} value={value}
          onChange={(e) => { onChange(e.target.value); }} onPaste={onPaste}
          placeholder="sk-ant-api03-…" autoComplete="off" className="input pr-10" />
        <button type="button" aria-label={show ? "Hide key" : "Show key"} onClick={toggleShow}
          className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-neutral-500 hover:text-neutral-300">
          {show ? <EyeOff size={16} /> : <Eye size={16} />}
        </button>
      </div>
      <p className="mb-4 text-xs text-neutral-600">
        Your key is stored in plain text on this computer. We&rsquo;re working on encrypted storage for a future release.
      </p>
      {phase === "error" && errorKey && (
        <p className="mb-4 rounded-md bg-danger-500/10 px-3 py-2 text-sm text-danger-400">
          {ANTHROPIC_ERRORS[errorKey] ?? ANTHROPIC_ERRORS.unknown}
        </p>
      )}
      <p className="mb-6 text-xs text-neutral-500">
        Don&rsquo;t have one yet?{" "}
        <button type="button" onClick={onHowTo}
          className="text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline">
          Show me how to get one (2 min)
        </button>
      </p>
      <div className="flex items-center justify-end gap-3">
        {phase !== "valid" && (
          <button type="button" onClick={onValidate} className="btn btn-md btn-primary" disabled={!canValidate}>
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

// eslint-disable-next-line complexity
function TelegramStep({
  value, onChange, onPaste, show, toggleShow,
  phase, error, botUsername, botUrl, pollElapsed,
  onValidate, onOpenBot, onWaitMore, onSkip, onConfirm, onRetry, onHowTo, commitError,
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
        How you&rsquo;ll talk to your assistant on your phone.
      </p>
      {(phase === "idle" || phase === "validating" || phase === "error") && (
        <>
          <div className="relative mb-4">
            <input type={show ? "text" : "password"} value={value}
              onChange={(e) => { onChange(e.target.value); }} onPaste={onPaste}
              placeholder="1234567890:ABCdefGHIjkl…" autoComplete="off" className="input pr-10" />
            <button type="button" aria-label={show ? "Hide token" : "Show token"} onClick={toggleShow}
              className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-neutral-500 hover:text-neutral-300">
              {show ? <EyeOff size={16} /> : <Eye size={16} />}
            </button>
          </div>
          {phase === "error" && error && (
            <p className="mb-4 rounded-md bg-danger-500/10 px-3 py-2 text-sm text-danger-400">{error}</p>
          )}
          <p className="mb-6 text-xs text-neutral-500">
            Need to create one?{" "}
            <button type="button" onClick={onHowTo}
              className="text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline">
              Walk me through it (3 min)
            </button>
          </p>
          <div className="flex items-center justify-between">
            {phase === "error" && (
              <button type="button" onClick={onRetry} className="btn btn-md btn-ghost">Change token</button>
            )}
            <div className="ml-auto flex gap-3">
              <button type="button" onClick={onValidate} className="btn btn-md btn-primary" disabled={!canValidate}>
                {phase === "validating" ? "Checking…" : "Validate bot"}
              </button>
            </div>
          </div>
        </>
      )}
      {(phase === "deep_link" || phase === "polling" || phase === "timed_out") && (
        <div className="text-center">
          <p className="mb-4 text-sm text-neutral-300">Open your bot in Telegram and tap <strong>Start</strong>.</p>
          {botUrl && (
            <button type="button" onClick={onOpenBot} className="btn btn-md btn-primary mb-6">
              <MessageCircle size={16} />
              Open @{botUsername} in Telegram
            </button>
          )}
          {phase === "timed_out" ? (
            <div>
              <p className="mb-4 text-sm text-neutral-500">Still waiting for your /start in Telegram.</p>
              <div className="flex justify-center gap-3">
                <button type="button" onClick={onSkip} className="btn btn-md btn-ghost">Skip and test later</button>
                <button type="button" onClick={onWaitMore} className="btn btn-md btn-primary">Wait another 90s</button>
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center gap-3">
              <div className="flex items-center gap-2 text-xs text-neutral-500">
                <span className="inline-block h-2 w-2 animate-pulse rounded-full bg-primary-500" />
                Waiting for your /start{pollElapsed > 0 ? ` (${String(pollElapsed)}s)` : ""}…
              </div>
              <button type="button" onClick={onSkip} className="btn btn-sm btn-ghost">Skip and test later</button>
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
          <p className="mb-6 text-sm text-neutral-300">Did you see the message from your bot in Telegram?</p>
          {commitError && (
            <p className="mb-4 rounded-md bg-danger-500/10 px-3 py-2 text-sm text-danger-400">{commitError}</p>
          )}
          <div className="flex justify-center gap-3">
            <button type="button" onClick={onSkip} className="btn btn-md btn-ghost">No, skip for now</button>
            <button type="button" onClick={onConfirm} className="btn btn-md btn-primary">Yes, launch my assistant</button>
          </div>
        </div>
      )}
    </div>
  );
}
