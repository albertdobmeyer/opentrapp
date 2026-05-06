import { Check, Eye, EyeOff, Key, MessageCircle, ExternalLink } from "lucide-react";
import { useEffect, useRef, useState, type ClipboardEvent } from "react";

import { classifyError } from "@/lib/errors";
import { readConfig, writeConfig } from "@/lib/tauri";
import { useToast } from "@/lib/ToastContext";
import {
  identifyPastedKey,
  isAnthropicKeyLike,
  isTelegramTokenLike,
  maskKey,
  parseEnvKeys,
  upsertEnvVar,
} from "@/lib/wizardUtils";

import HowToModal, { type HowToStep } from "./HowToModal";

interface Props {
  onContinue: (opts: { skippedKeys: boolean }) => void;
  onBack: () => void;
}

const ANTHROPIC_STEPS: HowToStep[] = [
  {
    heading: "Open the Anthropic console",
    body: "Go to console.anthropic.com and sign in (or sign up if you don't have an account yet).",
    href: "https://console.anthropic.com/",
  },
  {
    heading: "Go to the API Keys page",
    body: "In the left-hand menu, click Settings, then API Keys.",
  },
  {
    heading: "Create a new key",
    body: "Click Create Key. Give it a name like \"Lobster-TrApp\". Choose a workspace if prompted.",
  },
  {
    heading: "Copy the key immediately",
    body: "Anthropic shows the full key once — the string starts with sk-ant-. Copy it now; you can't retrieve it later. If you lose it, you can always create another.",
  },
  {
    heading: "Paste it back in Lobster-TrApp",
    body: "Close this window and paste the key into the Anthropic card. The green checkmark appears when the format looks right.",
  },
];

const TELEGRAM_STEPS: HowToStep[] = [
  {
    heading: "Open Telegram and find BotFather",
    body: "Search for @BotFather in Telegram. It has an official blue checkmark. Start a chat with it.",
    href: "https://t.me/BotFather",
  },
  {
    heading: "Send /newbot",
    body: "Type /newbot and hit send. BotFather will ask for a display name and a username.",
  },
  {
    heading: "Pick a name and username",
    body: "Name: anything you like (e.g. \"My Assistant\"). Username: must end in \"bot\" and be unique (e.g. karen_assistant_bot).",
  },
  {
    heading: "Copy the token BotFather gives you",
    body: "Once the bot is created, BotFather sends a token that looks like 1234567890:ABCdef.... That's the token Lobster-TrApp needs.",
  },
  {
    heading: "Paste it back in Lobster-TrApp",
    body: "Close this window and paste the token into the Telegram card. The green checkmark appears when the format looks right.",
  },
];

export default function ConnectStep({ onContinue, onBack }: Props) {
  const { addToast } = useToast();

  const [anthropicKey, setAnthropicKey] = useState("");
  const [telegramToken, setTelegramToken] = useState("");
  const [existingAnthropicMask, setExistingAnthropicMask] = useState<string | null>(null);
  const [existingTelegramMask, setExistingTelegramMask] = useState<string | null>(null);
  const [showAnthropic, setShowAnthropic] = useState(false);
  const [showTelegram, setShowTelegram] = useState(false);
  const [saving, setSaving] = useState(false);
  const [announcement, setAnnouncement] = useState("");
  const [openModal, setOpenModal] = useState<"anthropic" | "telegram" | null>(null);

  const anthropicInputRef = useRef<HTMLInputElement | null>(null);
  const telegramInputRef = useRef<HTMLInputElement | null>(null);

  // Load existing keys from .env if available — pre-populate as masked.
  useEffect(() => {
    let cancelled = false;
    readConfig("openclaw-vault", ".env")
      .then((content) => {
        if (cancelled) return;
        const { anthropicKey: savedAnthropic, telegramToken: savedTelegram } =
          parseEnvKeys(content);
        if (savedAnthropic) setExistingAnthropicMask(maskKey(savedAnthropic));
        if (savedTelegram) setExistingTelegramMask(maskKey(savedTelegram));
      })
      .catch(() => {
        // .env doesn't exist yet — totally fine
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Clear the aria-live announcement after a beat so it doesn't stay stuck.
  useEffect(() => {
    if (!announcement) return;
    const t = setTimeout(() => { setAnnouncement(""); }, 2000);
    return () => { clearTimeout(t); };
  }, [announcement]);

  function handlePaste(
    e: ClipboardEvent<HTMLInputElement>,
    targetField: "anthropic" | "telegram",
  ) {
    const pasted = e.clipboardData.getData("text");
    const identified = identifyPastedKey(pasted);
    if (!identified || identified === targetField) return;

    // Swap to the correct field.
    e.preventDefault();
    if (identified === "anthropic") {
      setAnthropicKey(pasted.trim());
      setExistingAnthropicMask(null);
      anthropicInputRef.current?.focus();
      setAnnouncement("That looks like an Anthropic key; moved to the right field.");
    } else {
      setTelegramToken(pasted.trim());
      setExistingTelegramMask(null);
      telegramInputRef.current?.focus();
      setAnnouncement("That looks like a Telegram bot token; moved to the right field.");
    }
  }

  async function persistKeys(): Promise<void> {
    // If user didn't touch the inputs but has existing masked values, nothing
    // to save — keep the .env as is.
    if (!anthropicKey && !telegramToken) return;

    let content = "";
    try {
      content = await readConfig("openclaw-vault", ".env");
    } catch {
      content = "# OpenClaw-Vault configuration\n";
    }

    if (anthropicKey) {
      content = upsertEnvVar(content, "ANTHROPIC_API_KEY", anthropicKey);
    }
    if (telegramToken) {
      content = upsertEnvVar(content, "TELEGRAM_BOT_TOKEN", telegramToken);
    }

    await writeConfig("openclaw-vault", ".env", content);
  }

  async function handleContinue(opts: { skip: boolean }) {
    setSaving(true);
    try {
      if (!opts.skip) {
        await persistKeys();
      }
      onContinue({ skippedKeys: opts.skip });
    } catch (error) {
      const classified = classifyError(error);
      addToast({
        type: "error",
        title: classified.title === "Something went wrong"
          ? "Couldn't save your keys"
          : classified.title,
        message: classified.userMessage,
        duration: 0,
      });
    } finally {
      setSaving(false);
    }
  }

  const anthropicValid = isAnthropicKeyLike(anthropicKey);
  const telegramValid = isTelegramTokenLike(telegramToken);
  const hasAnyKey =
    anthropicKey.length > 0 ||
    telegramToken.length > 0 ||
    (existingAnthropicMask !== null && !anthropicKey) ||
    (existingTelegramMask !== null && !telegramToken);

  return (
    <div className="animate-fade-in mx-auto max-w-xl px-4 py-6">
      <h1 className="mb-2 text-2xl font-semibold text-neutral-100">
        Connect your accounts
      </h1>
      <p className="mb-8 text-sm text-neutral-400">
        Your assistant needs two things to work. Enter them once and you're
        done. Nothing leaves your computer.
      </p>

      {/* Live region — paste-swap announcements */}
      <div aria-live="polite" className="sr-only">
        {announcement}
      </div>

      {/* Anthropic card */}
      <div className="card-raised mb-5">
        <div className="mb-3 flex items-center gap-2">
          <Key size={18} className="text-primary-400" />
          <label
            htmlFor="anthropic-key"
            className="text-sm font-medium text-neutral-100"
          >
            Anthropic API key
          </label>
          {anthropicValid && (
            <Check size={16} className="text-success-400" aria-label="Valid format" />
          )}
        </div>
        <p className="mb-3 text-xs text-neutral-500">
          The AI's brain. Also how you'll pay for its thoughts (about $5–20/month
          for typical use).
        </p>

        {existingAnthropicMask && !anthropicKey ? (
          <div className="flex items-center justify-between gap-3 rounded-md bg-neutral-900 px-3 py-2">
            <code className="text-sm text-neutral-300">{existingAnthropicMask}</code>
            <button
              type="button"
              onClick={() => {
                setExistingAnthropicMask(null);
                setTimeout(() => anthropicInputRef.current?.focus(), 0);
              }}
              className="text-xs text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
            >
              Change
            </button>
          </div>
        ) : (
          <div className="relative">
            <input
              ref={anthropicInputRef}
              id="anthropic-key"
              type={showAnthropic ? "text" : "password"}
              value={anthropicKey}
              onChange={(e) => { setAnthropicKey(e.target.value); }}
              onPaste={(e) => { handlePaste(e, "anthropic"); }}
              placeholder="sk-ant-api03-..."
              autoComplete="off"
              aria-describedby="anthropic-hint"
              className="input pr-10"
            />
            <button
              type="button"
              aria-label={showAnthropic ? "Hide key" : "Show key"}
              onClick={() => { setShowAnthropic((v) => !v); }}
              className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-neutral-500 hover:text-neutral-300"
            >
              {showAnthropic ? <EyeOff size={16} /> : <Eye size={16} />}
            </button>
          </div>
        )}

        <p id="anthropic-hint" className="mt-3 text-xs text-neutral-500">
          Don't have one yet?{" "}
          <button
            type="button"
            onClick={() => { setOpenModal("anthropic"); }}
            className="inline-flex items-center gap-1 text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
          >
            Show me how to get one (2 min)
            <ExternalLink size={11} />
          </button>
        </p>
      </div>

      {/* Telegram card */}
      <div className="card-raised mb-8">
        <div className="mb-3 flex items-center gap-2">
          <MessageCircle size={18} className="text-info-400" />
          <label
            htmlFor="telegram-token"
            className="text-sm font-medium text-neutral-100"
          >
            Telegram bot
          </label>
          {telegramValid && (
            <Check size={16} className="text-success-400" aria-label="Valid format" />
          )}
        </div>
        <p className="mb-3 text-xs text-neutral-500">
          How you'll talk to your assistant.
        </p>

        {existingTelegramMask && !telegramToken ? (
          <div className="flex items-center justify-between gap-3 rounded-md bg-neutral-900 px-3 py-2">
            <code className="text-sm text-neutral-300">{existingTelegramMask}</code>
            <button
              type="button"
              onClick={() => {
                setExistingTelegramMask(null);
                setTimeout(() => telegramInputRef.current?.focus(), 0);
              }}
              className="text-xs text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
            >
              Change
            </button>
          </div>
        ) : (
          <div className="relative">
            <input
              ref={telegramInputRef}
              id="telegram-token"
              type={showTelegram ? "text" : "password"}
              value={telegramToken}
              onChange={(e) => { setTelegramToken(e.target.value); }}
              onPaste={(e) => { handlePaste(e, "telegram"); }}
              placeholder="1234567890:ABCdefGHIjkl..."
              autoComplete="off"
              aria-describedby="telegram-hint"
              className="input pr-10"
            />
            <button
              type="button"
              aria-label={showTelegram ? "Hide token" : "Show token"}
              onClick={() => { setShowTelegram((v) => !v); }}
              className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-neutral-500 hover:text-neutral-300"
            >
              {showTelegram ? <EyeOff size={16} /> : <Eye size={16} />}
            </button>
          </div>
        )}

        <p id="telegram-hint" className="mt-3 text-xs text-neutral-500">
          Need to create one?{" "}
          <button
            type="button"
            onClick={() => { setOpenModal("telegram"); }}
            className="inline-flex items-center gap-1 text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
          >
            Walk me through it (3 min)
            <ExternalLink size={11} />
          </button>
        </p>
      </div>

      <div className="flex items-center justify-between">
        <button
          type="button"
          onClick={onBack}
          className="btn btn-md btn-ghost"
          disabled={saving}
        >
          Back
        </button>
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => handleContinue({ skip: true })}
            className="btn btn-md btn-ghost"
            disabled={saving}
          >
            Skip
          </button>
          <button
            type="button"
            onClick={() => handleContinue({ skip: false })}
            className="btn btn-md btn-primary"
            disabled={saving || !hasAnyKey}
          >
            {saving ? "Saving…" : "Continue"}
          </button>
        </div>
      </div>

      <HowToModal
        open={openModal === "anthropic"}
        onClose={() => { setOpenModal(null); }}
        title="How to get an Anthropic API key"
        steps={ANTHROPIC_STEPS}
      />
      <HowToModal
        open={openModal === "telegram"}
        onClose={() => { setOpenModal(null); }}
        title="How to create a Telegram bot"
        steps={TELEGRAM_STEPS}
      />
    </div>
  );
}
