import { open as openUrl } from "@tauri-apps/plugin-shell";
import { LayoutDashboard, MessageCircle } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { useSettings } from "@/hooks/useSettings";

const AUTO_ADVANCE_SECONDS = 5;

interface Props {
  onGoToDashboard: () => void;
}

export default function ReadyStep({ onGoToDashboard }: Props) {
  const { settings } = useSettings();
  const primaryBtnRef = useRef<HTMLButtonElement | null>(null);
  const [secondsLeft, setSecondsLeft] = useState<number | null>(null);

  // Focus the primary CTA on mount.
  useEffect(() => {
    primaryBtnRef.current?.focus();
  }, []);

  // After a short grace period with no user action, show a countdown that
  // auto-advances to the dashboard. "Gives user agency" per spec 07 §Step 4.
  useEffect(() => {
    const startCountdownAfterMs = 3000;
    const t = setTimeout(() => { setSecondsLeft(AUTO_ADVANCE_SECONDS); }, startCountdownAfterMs);
    return () => { clearTimeout(t); };
  }, []);

  useEffect(() => {
    if (secondsLeft === null) return;
    if (secondsLeft <= 0) {
      onGoToDashboard();
      return;
    }
    const t = setTimeout(() => { setSecondsLeft((s) => (s === null ? null : s - 1)); }, 1000);
    return () => { clearTimeout(t); };
  }, [secondsLeft, onGoToDashboard]);

  const cancelAutoAdvance = () => {
    setSecondsLeft(null);
  };

  // Resolve the best Telegram link we have. If the prefetched URL from
  // Install isn't available but we know the bot's @username, build the
  // deep-link from it. Generic telegram.org is the last resort.
  const telegramLink =
    settings.telegramBotUrl ??
    (settings.telegramBotUsername
      ? `https://t.me/${settings.telegramBotUsername}?text=Hi`
      : "https://telegram.org");

  async function handleOpenTelegram() {
    cancelAutoAdvance();
    try {
      await openUrl(telegramLink);
    } catch {
      // Fall back to window.open if the shell plugin isn't available in a
      // given build (e.g. running `npm run dev` outside Tauri).
      window.open(telegramLink, "_blank", "noopener,noreferrer");
    }
  }

  return (
    <div className="mx-auto flex min-h-[70vh] max-w-xl flex-col items-center justify-center px-4 text-center">
      <div className="animate-celebrate">
        <CelebrationIllustration className="mx-auto mb-8" />
      </div>

      <h1 className="animate-slide-up mb-3 text-3xl font-semibold text-neutral-100 sm:text-4xl">
        Your assistant is ready! 🎉
      </h1>
      <p className="animate-slide-up mb-10 max-w-md text-base text-neutral-400">
        {settings.telegramBotUsername
          ? <>Say hi on Telegram — search for <span className="text-neutral-200">@{settings.telegramBotUsername}</span> if it doesn’t open the right chat.</>
          : "Say hi on Telegram to get started."}
      </p>

      <div className="flex flex-col items-center gap-4">
        <button
          ref={primaryBtnRef}
          type="button"
          onClick={handleOpenTelegram}
          className="btn btn-xl btn-primary"
        >
          <MessageCircle size={20} />
          Open Telegram
        </button>

        <button
          type="button"
          onClick={() => {
            cancelAutoAdvance();
            onGoToDashboard();
          }}
          className="btn btn-md btn-ghost"
        >
          <LayoutDashboard size={16} />
          Go to dashboard
        </button>
      </div>

      <div className="mt-12 max-w-md rounded-lg border border-neutral-800 bg-neutral-900 p-4 text-left text-xs text-neutral-400">
        <p>
          💡 <span className="text-neutral-300">Tip:</span> You can ask your
          assistant things like “What’s the weather?” or “Plan my Tuesday.”
        </p>
      </div>

      {secondsLeft !== null && secondsLeft > 0 && (
        <p
          aria-live="polite"
          className="mt-6 text-xs text-neutral-500"
        >
          Taking you to dashboard in {secondsLeft}…{" "}
          <button
            type="button"
            onClick={cancelAutoAdvance}
            className="text-primary-400 hover:text-primary-300 underline-offset-4 hover:underline"
          >
            Stay here
          </button>
        </p>
      )}
    </div>
  );
}

/**
 * Celebration SVG — lobster waving surrounded by confetti. Hand-rolled
 * placeholder until E.4 swaps in real art.
 */
function CelebrationIllustration({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      width="240"
      height="200"
      viewBox="0 0 240 200"
      role="img"
      aria-label="A celebrating lobster surrounded by confetti"
    >
      <defs>
        <linearGradient id="lobster-ready" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#ff8a5c" />
          <stop offset="100%" stopColor="#e55527" />
        </linearGradient>
      </defs>

      {/* Confetti */}
      {[
        { x: 30, y: 30, c: "#60a5fa", r: 4 },
        { x: 210, y: 40, c: "#f59e0b", r: 5 },
        { x: 50, y: 160, c: "#34d399", r: 3 },
        { x: 200, y: 150, c: "#ff6b35", r: 4 },
        { x: 120, y: 20, c: "#fbbf24", r: 3 },
        { x: 20, y: 100, c: "#f87171", r: 3 },
        { x: 220, y: 90, c: "#a78bfa", r: 4 },
        { x: 95, y: 180, c: "#60a5fa", r: 3 },
      ].map((c, i) => (
        <circle key={i} cx={c.x} cy={c.y} r={c.r} fill={c.c} opacity="0.85" />
      ))}

      {/* Lobster body */}
      <g transform="translate(70 50)">
        <ellipse cx="50" cy="60" rx="50" ry="34" fill="url(#lobster-ready)" />
        {/* Tail */}
        <path d="M2 60 Q -12 48 -10 32 Q 10 40 14 54 Z" fill="#ff6b35" />
        {/* Waving claw — raised up */}
        <g transform="translate(94 10) rotate(-20)">
          <ellipse cx="10" cy="8" rx="16" ry="10" fill="#ff6b35" />
          <path
            d="M 22 2 L 32 -6 M 22 14 L 32 22"
            stroke="#ff6b35"
            strokeWidth="6"
            strokeLinecap="round"
          />
        </g>
        {/* Resting claw */}
        <g transform="translate(88 68)">
          <ellipse cx="14" cy="8" rx="18" ry="10" fill="#ff6b35" />
        </g>
        {/* Antennae */}
        <path
          d="M 20 38 Q 10 12 24 6"
          fill="none"
          stroke="#b84520"
          strokeWidth="2"
          strokeLinecap="round"
        />
        <path
          d="M 30 38 Q 30 10 44 4"
          fill="none"
          stroke="#b84520"
          strokeWidth="2"
          strokeLinecap="round"
        />
        {/* Eyes — happy closed */}
        <path
          d="M 26 48 Q 30 44 34 48"
          fill="none"
          stroke="#0b1120"
          strokeWidth="2"
          strokeLinecap="round"
        />
        <path
          d="M 42 48 Q 46 44 50 48"
          fill="none"
          stroke="#0b1120"
          strokeWidth="2"
          strokeLinecap="round"
        />
        {/* Big smile */}
        <path
          d="M 26 60 Q 38 74 50 60"
          fill="none"
          stroke="#0b1120"
          strokeWidth="2"
          strokeLinecap="round"
        />
      </g>
    </svg>
  );
}
