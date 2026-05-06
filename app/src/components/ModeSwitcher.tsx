import { Terminal, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";

import { useAppContext } from "@/hooks/useAppContext";

/**
 * Listens for Cmd/Ctrl+Shift+D, toggles between user and developer mode, and
 * shows a one-time welcome dialog the first time a user opts into Advanced Mode.
 *
 * Mounted once at the app root (inside the Router) so the shortcut is global.
 */
export default function ModeSwitcher() {
  const { mode, toggleMode, settings, markAdvancedModeIntroSeen } =
    useAppContext();
  const navigate = useNavigate();
  const location = useLocation();
  const [introOpen, setIntroOpen] = useState(false);
  const [lastEnteredFrom, setLastEnteredFrom] = useState<string | null>(null);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const isToggle =
        (event.metaKey || event.ctrlKey) &&
        event.shiftKey &&
        (event.key === "d" || event.key === "D");
      if (!isToggle) return;

      event.preventDefault();
      const enteringDeveloper = mode !== "developer";

      void toggleMode().then((next) => {
        if (next === "developer") {
          setLastEnteredFrom(location.pathname);
          navigate("/dev", { replace: false });
          if (enteringDeveloper && !settings.hasSeenAdvancedModeIntro) {
            setIntroOpen(true);
          }
        } else {
          navigate("/", { replace: false });
        }
      });
    }

    window.addEventListener("keydown", onKeyDown);
    return () => { window.removeEventListener("keydown", onKeyDown); };
  }, [
    mode,
    toggleMode,
    navigate,
    location.pathname,
    settings.hasSeenAdvancedModeIntro,
  ]);

  async function handleContinue() {
    await markAdvancedModeIntroSeen();
    setIntroOpen(false);
  }

  async function handleGoBack() {
    await markAdvancedModeIntroSeen();
    setIntroOpen(false);
    await toggleMode();
    navigate(lastEnteredFrom ?? "/", { replace: true });
  }

  if (!introOpen) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="advanced-intro-title"
      className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-neutral-950/80 animate-fade-in"
    >
      <div className="card-hero max-w-lg w-full relative animate-slide-up">
        <button
          type="button"
          onClick={handleContinue}
          aria-label="Close"
          className="absolute top-4 right-4 text-neutral-500 hover:text-neutral-300"
        >
          <X size={18} />
        </button>
        <div className="flex items-center gap-3 mb-4">
          <div className="w-10 h-10 rounded-md bg-info-500/15 text-info-400 flex items-center justify-center">
            <Terminal size={20} aria-hidden />
          </div>
          <h2
            id="advanced-intro-title"
            className="text-xl font-semibold text-neutral-100"
          >
            Welcome to Advanced Mode
          </h2>
        </div>
        <p className="text-sm text-neutral-300 mb-3">
          You’re now seeing Lobster-TrApp’s full technical controls. This view
          shows you every component, log, configuration, and security check.
        </p>
        <ul className="text-sm text-neutral-400 mb-6 space-y-1.5 list-disc list-inside">
          <li>Changes here can break your setup. Be careful.</li>
          <li>
            Return to the friendly view anytime with{" "}
            <kbd className="px-1.5 py-0.5 rounded border border-neutral-700 text-neutral-300 font-mono text-xs">
              ⌘⇧D
            </kbd>
            .
          </li>
          <li>This mode is hidden by default — you probably don’t need it.</li>
        </ul>
        <div className="flex items-center justify-end gap-2">
          <button
            type="button"
            onClick={handleGoBack}
            className="btn btn-ghost btn-md"
          >
            Actually, go back
          </button>
          <button
            type="button"
            onClick={handleContinue}
            className="btn btn-primary btn-md"
          >
            Got it, let me explore
          </button>
        </div>
      </div>
    </div>
  );
}
