import { Check, ExternalLink, Eye, EyeOff, Key, MessageCircle } from "lucide-react";
import { useEffect, useRef, useState, type ClipboardEvent, type ReactNode, type Ref } from "react";

import { useToast } from "@/hooks/useToast";
import { classifyError } from "@/lib/errors";
import { readConfig, writeConfig } from "@/lib/tauri";
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

type FieldKind = "anthropic" | "telegram";

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
    body: "Click Create Key. Give it a name like \"OpenTrApp\". Choose a workspace if prompted.",
  },
  {
    heading: "Copy the key immediately",
    body: "Anthropic shows the full key once — the string starts with sk-ant-. Copy it now; you can't retrieve it later. If you lose it, you can always create another.",
  },
  {
    heading: "Paste it back in OpenTrApp",
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
    body: "Once the bot is created, BotFather sends a token that looks like 1234567890:ABCdef.... That's the token OpenTrApp needs.",
  },
  {
    heading: "Paste it back in OpenTrApp",
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
  const [openModal, setOpenModal] = useState<FieldKind | null>(null);

  const anthropicInputRef = useRef<HTMLInputElement | null>(null);
  const telegramInputRef = useRef<HTMLInputElement | null>(null);

  // Load existing keys from .env if available — pre-populate as masked.
  useEffect(() => {
    let cancelled = false;
    readConfig("openclaw-vault", ".env")
      .then((content) => {
         
        if (cancelled) return;
        const { anthropicKey: savedAnthropic, telegramToken: savedTelegram } = parseEnvKeys(content);
        if (savedAnthropic) setExistingAnthropicMask(maskKey(savedAnthropic));
        if (savedTelegram) setExistingTelegramMask(maskKey(savedTelegram));
      })
      .catch(() => undefined);
    return () => { cancelled = true; };
  }, []);

  // Clear the aria-live announcement after a beat so it doesn't stay stuck.
  useEffect(() => {
    if (!announcement) return;
    const t = setTimeout(() => { setAnnouncement(""); }, 2000);
    return () => { clearTimeout(t); };
  }, [announcement]);

  function handlePaste(e: ClipboardEvent<HTMLInputElement>, target: FieldKind) {
    const pasted = e.clipboardData.getData("text");
    const identified = identifyPastedKey(pasted);
    if (!identified || identified === target) return;
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

  const handleContinueClick = (skip: boolean) => {
    void runContinue({ skip, anthropicKey, telegramToken, setSaving, addToast, onContinue });
  };

  const hasAnyKey =
    anthropicKey.length > 0 ||
    telegramToken.length > 0 ||
    (existingAnthropicMask !== null && !anthropicKey) ||
    (existingTelegramMask !== null && !telegramToken);

  return (
    <div className="animate-fade-in mx-auto max-w-xl px-4 py-6">
      <h1 className="mb-2 text-2xl font-semibold text-neutral-100">Connect your accounts</h1>
      <p className="mb-8 text-sm text-neutral-400">
        Your assistant needs two things to work. Enter them once and you’re done. Nothing leaves your computer.
      </p>

      {/* Live region — paste-swap announcements */}
      <div aria-live="polite" className="sr-only">{announcement}</div>

      <KeyCard
        kind="anthropic"
        icon={<Key size={18} className="text-primary-400" />}
        label="Anthropic API key"
        hint="The AI’s brain. Also how you’ll pay for its thoughts (about $5–20/month for typical use)."
        placeholder="sk-ant-api03-..."
        inputId="anthropic-key"
        inputRef={anthropicInputRef}
        value={anthropicKey}
        onChange={setAnthropicKey}
        onPaste={(e) => { handlePaste(e, "anthropic"); }}
        show={showAnthropic}
        toggleShow={() => { setShowAnthropic((v) => !v); }}
        existingMask={existingAnthropicMask}
        clearMask={() => { setExistingAnthropicMask(null); }}
        isValid={isAnthropicKeyLike(anthropicKey)}
        howToLabel="Don’t have one yet?"
        howToCta="Show me how to get one (2 min)"
        onOpenHowTo={() => { setOpenModal("anthropic"); }}
        wrapperExtra="mb-5"
      />

      <KeyCard
        kind="telegram"
        icon={<MessageCircle size={18} className="text-info-400" />}
        label="Telegram bot"
        hint="How you’ll talk to your assistant."
        placeholder="1234567890:ABCdefGHIjkl..."
        inputId="telegram-token"
        inputRef={telegramInputRef}
        value={telegramToken}
        onChange={setTelegramToken}
        onPaste={(e) => { handlePaste(e, "telegram"); }}
        show={showTelegram}
        toggleShow={() => { setShowTelegram((v) => !v); }}
        existingMask={existingTelegramMask}
        clearMask={() => { setExistingTelegramMask(null); }}
        isValid={isTelegramTokenLike(telegramToken)}
        howToLabel="Need to create one?"
        howToCta="Walk me through it (3 min)"
        onOpenHowTo={() => { setOpenModal("telegram"); }}
        wrapperExtra="mb-8"
      />

      <div className="flex items-center justify-between">
        <button type="button" onClick={onBack} className="btn btn-md btn-ghost" disabled={saving}>
          Back
        </button>
        <div className="flex items-center gap-3">
          <button type="button" onClick={() => { handleContinueClick(true); }} className="btn btn-md btn-ghost" disabled={saving}>
            Skip
          </button>
          <button
            type="button"
            onClick={() => { handleContinueClick(false); }}
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

interface KeyCardProps {
  kind: FieldKind;
  icon: ReactNode;
  label: string;
  hint: string;
  placeholder: string;
  inputId: string;
  inputRef: Ref<HTMLInputElement> & { current: HTMLInputElement | null };
  value: string;
  onChange: (v: string) => void;
  onPaste: (e: ClipboardEvent<HTMLInputElement>) => void;
  show: boolean;
  toggleShow: () => void;
  existingMask: string | null;
  clearMask: () => void;
  isValid: boolean;
  howToLabel: string;
  howToCta: string;
  onOpenHowTo: () => void;
  wrapperExtra: string;
}

function KeyCard({
  icon, label, hint, placeholder, inputId, inputRef, value, onChange, onPaste,
  show, toggleShow, existingMask, clearMask, isValid,
  howToLabel, howToCta, onOpenHowTo, wrapperExtra,
}: KeyCardProps) {
  const showingMask = existingMask !== null && !value;
  return (
    <div className={`card-raised ${wrapperExtra}`}>
      <div className="mb-3 flex items-center gap-2">
        {icon}
        <label htmlFor={inputId} className="text-sm font-medium text-neutral-100">{label}</label>
        {isValid && <Check size={16} className="text-success-400" aria-label="Valid format" />}
      </div>
      <p className="mb-3 text-xs text-neutral-500">{hint}</p>

      {showingMask ? (
        <div className="flex items-center justify-between gap-3 rounded-md bg-neutral-900 px-3 py-2">
          <code className="text-sm text-neutral-300">{existingMask}</code>
          <button
            type="button"
            onClick={() => {
              clearMask();
              setTimeout(() => inputRef.current?.focus(), 0);
            }}
            className="text-xs text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
          >
            Change
          </button>
        </div>
      ) : (
        <div className="relative">
          <input
            ref={inputRef}
            id={inputId}
            type={show ? "text" : "password"}
            value={value}
            onChange={(e) => { onChange(e.target.value); }}
            onPaste={onPaste}
            placeholder={placeholder}
            autoComplete="off"
            aria-describedby={`${inputId}-hint`}
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
      )}

      <p id={`${inputId}-hint`} className="mt-3 text-xs text-neutral-500">
        {howToLabel}{" "}
        <button
          type="button"
          onClick={onOpenHowTo}
          className="inline-flex items-center gap-1 text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
        >
          {howToCta}
          <ExternalLink size={11} />
        </button>
      </p>
    </div>
  );
}

interface RunContinueArgs {
  skip: boolean;
  anthropicKey: string;
  telegramToken: string;
  setSaving: (v: boolean) => void;
  addToast: ReturnType<typeof useToast>["addToast"];
  onContinue: (opts: { skippedKeys: boolean }) => void;
}

async function runContinue(args: RunContinueArgs): Promise<void> {
  const { skip, anthropicKey, telegramToken, setSaving, addToast, onContinue } = args;
  setSaving(true);
  try {
    if (!skip) {
      await persistKeys({ anthropicKey, telegramToken });
    }
    onContinue({ skippedKeys: skip });
  } catch (error) {
    const classified = classifyError(error);
    addToast({
      type: "error",
      title: classified.title === "Something went wrong" ? "Couldn't save your keys" : classified.title,
      message: classified.userMessage,
      duration: 0,
    });
  } finally {
    setSaving(false);
  }
}

async function persistKeys({ anthropicKey, telegramToken }: { anthropicKey: string; telegramToken: string }): Promise<void> {
  // If user didn't touch the inputs but has existing masked values, nothing
  // to save — keep the .env as is.
  if (!anthropicKey && !telegramToken) return;

  let content = "";
  try {
    content = await readConfig("openclaw-vault", ".env");
  } catch {
    content = "# OpenTrApp agent configuration\n";
  }

  if (anthropicKey) content = upsertEnvVar(content, "ANTHROPIC_API_KEY", anthropicKey);
  if (telegramToken) content = upsertEnvVar(content, "TELEGRAM_BOT_TOKEN", telegramToken);

  await writeConfig("openclaw-vault", ".env", content);
}
