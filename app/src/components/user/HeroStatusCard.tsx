import { open as openUrl } from "@tauri-apps/plugin-shell";
import { MessageCircle, Pause, Play, RotateCcw } from "lucide-react";
import { useState } from "react";
import { useNavigate } from "react-router-dom";

import { useSettings } from "@/hooks/useSettings";
import { useToast } from "@/hooks/useToast";
import { classifyError } from "@/lib/errors";
import { pausePerimeter, resumePerimeter } from "@/lib/tauri";

import type { HeroState } from "@/hooks/useHero";

interface Props {
  state: HeroState;
  loading: boolean;
}

interface Copy {
  title: string;
  subline: string;
  /** Tailwind classes for the illustration ring color. */
  ringTint: string;
  /** Inner-dot tailwind class. */
  dotTint: string;
}

const COPY: Record<HeroState, Copy> = {
  running_safely: {
    title: "Your assistant is running safely",
    subline: "Everything looks good.",
    ringTint: "border-success-500/40 border-success-500/60",
    dotTint: "bg-success-500",
  },
  starting: {
    title: "Your assistant is starting…",
    subline: "Hang tight — this usually takes a few seconds.",
    ringTint: "border-warning-500/40 border-warning-500/60",
    dotTint: "bg-warning-500",
  },
  recovering: {
    title: "Your assistant is taking a moment",
    subline: "Back in a few seconds.",
    ringTint: "border-warning-500/40 border-warning-500/60",
    dotTint: "bg-warning-500",
  },
  error_perimeter: {
    title: "Your assistant didn't fully recover",
    subline: "Let's try to fix it together.",
    ringTint: "border-danger-500/40 border-danger-500/60",
    dotTint: "bg-danger-500",
  },
  error_key: {
    title: "Your Anthropic key isn't working",
    subline: "Update it in Preferences and your assistant will be back.",
    ringTint: "border-danger-500/40 border-danger-500/60",
    dotTint: "bg-danger-500",
  },
  not_setup: {
    title: "Your assistant isn't set up yet",
    subline: "Let's get you started.",
    ringTint: "border-primary-500/40 border-primary-500/60",
    dotTint: "bg-primary-500",
  },
  paused_by_user: {
    title: "Your assistant is paused",
    subline: "It won't respond on Telegram until you resume it.",
    ringTint: "border-neutral-500/40 border-neutral-500/60",
    dotTint: "bg-neutral-500",
  },
};

export default function HeroStatusCard({ state, loading }: Props) {
  const navigate = useNavigate();
  const { settings } = useSettings();
  const { addToast, removeToast } = useToast();
  const copy = COPY[state];
  const [pauseLoading, setPauseLoading] = useState(false);

  const telegramLink =
    settings.telegramBotUrl ??
    (settings.telegramBotUsername
      ? `https://t.me/${settings.telegramBotUsername}?text=Hi`
      : "https://telegram.org");

  async function handleOpenTelegram() {
    try {
      await openUrl(telegramLink);
    } catch {
      window.open(telegramLink, "_blank", "noopener,noreferrer");
    }
  }

  const handlePause = () =>
    runPerimeterToggle({
      setLoading: setPauseLoading,
      addToast,
      removeToast,
      action: pausePerimeter,
      pendingTitle: "Pausing your assistant…",
      pendingMessage: undefined,
      successTitle: "Your assistant is paused",
      successMessage: "It won't respond on Telegram until you resume it.",
      errorFallbackTitle: "Couldn't pause",
    });

  const handleResume = () =>
    runPerimeterToggle({
      setLoading: setPauseLoading,
      addToast,
      removeToast,
      action: resumePerimeter,
      pendingTitle: "Bringing your assistant back…",
      pendingMessage: "This usually takes about 10 seconds.",
      successTitle: "Your assistant is back online",
      successMessage: undefined,
      errorFallbackTitle: "Couldn't resume",
    });

  if (loading) {
    return (
      <div className="card-hero animate-pulse">
        <div className="mx-auto mb-6 h-20 w-20 rounded-full bg-neutral-800" />
        <div className="mx-auto mb-3 h-7 w-2/3 rounded bg-neutral-800" />
        <div className="mx-auto h-4 w-1/2 rounded bg-neutral-800" />
      </div>
    );
  }

  return (
    <div className="card-hero text-center" aria-live="polite">
      <StatusIllustration ringTint={copy.ringTint} dotTint={copy.dotTint} />

      <h1 className="mb-2 text-3xl font-semibold text-neutral-100">
        {copy.title}
      </h1>
      <p className="mx-auto mb-8 max-w-xl text-sm text-neutral-400">
        {copy.subline}
      </p>

      <div className="flex flex-wrap items-center justify-center gap-3">
        {state === "running_safely" && (
          <>
            <button
              type="button"
              onClick={handleOpenTelegram}
              className="btn btn-lg btn-primary"
            >
              <MessageCircle size={18} />
              Open Telegram
            </button>
            <button
              type="button"
              onClick={handlePause}
              className="btn btn-lg btn-ghost"
              disabled={pauseLoading}
            >
              <Pause size={18} />
              {pauseLoading ? "Pausing…" : "Pause"}
            </button>
          </>
        )}

        {state === "starting" && (
          <span className="text-xs text-neutral-500">
            Working on it — no action needed.
          </span>
        )}

        {state === "recovering" && (
          <span className="text-xs text-neutral-500">
            Working on it — no action needed.
          </span>
        )}

        {state === "error_perimeter" && (
          <>
            <button
              type="button"
              onClick={() => { window.location.reload(); }}
              className="btn btn-lg btn-primary"
            >
              <RotateCcw size={18} />
              Try to fix
            </button>
            <button
              type="button"
              onClick={() => { navigate("/help"); }}
              className="btn btn-lg btn-ghost"
            >
              Get help
            </button>
          </>
        )}

        {state === "error_key" && (
          <button
            type="button"
            onClick={() => { navigate("/preferences"); }}
            className="btn btn-lg btn-primary"
          >
            <RotateCcw size={18} />
            Update your key
          </button>
        )}

        {state === "not_setup" && (
          <button
            type="button"
            onClick={() => { navigate("/setup"); }}
            className="btn btn-lg btn-primary"
          >
            Run setup
          </button>
        )}

        {state === "paused_by_user" && (
          <button
            type="button"
            onClick={handleResume}
            className="btn btn-lg btn-primary"
            disabled={pauseLoading}
          >
            <Play size={18} />
            {pauseLoading ? "Resuming…" : "Resume"}
          </button>
        )}
      </div>
    </div>
  );
}

/**
 * State-coloured pulsing rings — stand-in for the bespoke status-{state}.svg
 * illustrations the page spec calls for. Reuses the wizard's animate-pulse-ring
 * keyframes from globals.css.
 */
function StatusIllustration({ ringTint, dotTint }: { ringTint: string; dotTint: string }) {
  // Split combined tint string into the two border tones.
  const [outer, inner] = ringTint.split(" ");
  return (
    <div
      className="relative mx-auto mb-6 h-20 w-20"
      role="img"
      aria-hidden="true"
    >
      <span
        className={`animate-pulse-ring absolute inset-0 rounded-full border-2 ${outer}`}
      />
      <span
        className={`animate-pulse-ring absolute inset-3 rounded-full border-2 ${inner}`}
        style={{ animationDelay: "0.3s" }}
      />
      <span className={`absolute inset-7 rounded-full ${dotTint}`} />
    </div>
  );
}

interface PerimeterToggleArgs {
  setLoading: (v: boolean) => void;
  addToast: ReturnType<typeof useToast>["addToast"];
  removeToast: ReturnType<typeof useToast>["removeToast"];
  action: () => Promise<void>;
  pendingTitle: string;
  pendingMessage: string | undefined;
  successTitle: string;
  successMessage: string | undefined;
  errorFallbackTitle: string;
}

/**
 * Drives one perimeter pause/resume action through its full toast lifecycle:
 * sticky in-flight → success or classified error. Hoisted out of the
 * component so the host stays focused on render concerns.
 */
async function runPerimeterToggle(args: PerimeterToggleArgs): Promise<void> {
  const {
    setLoading, addToast, removeToast, action,
    pendingTitle, pendingMessage,
    successTitle, successMessage,
    errorFallbackTitle,
  } = args;
  setLoading(true);
  const stickyId = addToast({
    type: "info",
    title: pendingTitle,
    message: pendingMessage,
    duration: 0,
  });
  try {
    await action();
    removeToast(stickyId);
    addToast({ type: "success", title: successTitle, message: successMessage });
  } catch (error) {
    removeToast(stickyId);
    const c = classifyError(error);
    addToast({
      type: "error",
      title: c.title === "Something went wrong" ? errorFallbackTitle : c.title,
      message: c.userMessage,
    });
  } finally {
    setLoading(false);
  }
}
