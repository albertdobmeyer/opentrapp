import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { Navigate, Route, Routes } from "react-router-dom";

import { ErrorBoundary } from "@/components/ErrorBoundary";
import ModeSwitcher from "@/components/ModeSwitcher";
import UserLayout from "@/components/UserLayout";
import { useManifests } from "@/hooks/useManifests";
import { useSettings } from "@/hooks/useSettings";
import DevLayout from "@/layouts/DevLayout";
import { AppContextProvider } from "@/lib/AppContext";
import {
  getAutostartEnabled,
  setAutostartEnabled,
} from "@/lib/osIntegration";
import { ToastProvider } from "@/lib/ToastContext";
import DevAllowlist from "@/pages/dev/DevAllowlist";
import DevComponentDetail from "@/pages/dev/DevComponentDetail";
import DevComponents from "@/pages/dev/DevComponents";
import DevLogs from "@/pages/dev/DevLogs";
import DevManifests from "@/pages/dev/DevManifests";
import DevOverview from "@/pages/dev/DevOverview";
import DevPreferences from "@/pages/dev/DevPreferences";
import DevSecurity from "@/pages/dev/DevSecurity";
import DevShellLevels from "@/pages/dev/DevShellLevels";
import NotFound from "@/pages/NotFound";
import Setup from "@/pages/Setup";
import Discover from "@/pages/user/Discover";
import Help from "@/pages/user/Help";
import Home from "@/pages/user/Home";
import Preferences from "@/pages/user/Preferences";
import SecurityMonitor from "@/pages/user/SecurityMonitor";

export default function App() {
  const { settings, loaded: settingsLoaded, update: updateSettings } = useSettings();
  // Manifests are still discovered (used by dev mode + setup wizard); user mode no longer needs them at the top level.
  useManifests();

  // Reconcile OS-level autostart with the persisted preference once on
  // boot. Handles two cases: a fresh install where the default ("on")
  // hasn't been registered with the OS yet, and a user who toggled
  // autostart off via System Settings outside the app.
  useEffect(() => {
    if (!settingsLoaded) return;
    let cancelled = false;
    void (async () => {
      try {
        const osState = await getAutostartEnabled();
        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- closure-mutated by cleanup function below; ESLint's narrowing is unaware
        if (cancelled) return;
        if (osState !== settings.autostart) {
          await setAutostartEnabled(settings.autostart);
        }
      } catch {
        // Browser dev mode or plugin not initialised — silent. The user
        // can still toggle in Preferences once running in Tauri.
      }
    })();
    return () => {
      cancelled = true;
    };
    // Intentionally only depending on settingsLoaded — we don't want to
    // re-run every time autostart toggles (the toggle handler already
    // calls setAutostartEnabled directly).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settingsLoaded]);

  // Populate the bot URL/username from the backend's auto-activate or
  // migration path — emitted once after vault-agent comes up successfully.
  useEffect(() => {
    let unlistenFn: (() => void) | undefined;
    void listen<{ url: string; username: string }>(
      "telegram-bot-resolved",
      (event) => {
        void updateSettings({
          telegramBotUrl: event.payload.url,
          telegramBotUsername: event.payload.username,
        });
      }
    ).then((fn) => {
      unlistenFn = fn;
    });
    return () => {
      unlistenFn?.();
    };
  }, [updateSettings]);

  if (!settingsLoaded) {
    return (
      <div className="flex h-screen items-center justify-center bg-neutral-900">
        <div className="text-sm text-neutral-500">Loading...</div>
      </div>
    );
  }

  const mode = settings.mode;

  // Demo escape hatch: only honoured when window.__OPENTRAPP_DEMO__ is set,
  // which only happens from the e2e/demo-tour.spec.ts Playwright recorder via
  // page.addInitScript. Production users never set this; the routing guard
  // behaves identically for them.
  const demoOverride =
    typeof window !== "undefined" &&
    Boolean((window as unknown as { __OPENTRAPP_DEMO__?: boolean }).__OPENTRAPP_DEMO__);
  const wizardCompletedForRouting = demoOverride || settings.wizardCompleted;

  return (
    <AppContextProvider
      settings={settings}
      settingsLoaded={settingsLoaded}
      updateSettings={updateSettings}
    >
      <ToastProvider>
        <ErrorBoundary>
          <ModeSwitcher />
          <Routes>
            {/* Setup wizard — no layout, outside modes. Always reachable so
                the wizard can run to completion regardless of routing-guard
                state below. */}
            <Route path="/setup" element={<Setup />} />

            {/* Routing guard (Zone 1): if the wizard has not completed yet,
                every non-setup path redirects to /setup. Prevents the dead-end
                where a fresh user lands on Home and sees a silent bootstrap
                failure with no path to recovery. Once `wizardCompleted` is
                true, normal routing resumes. */}
            {!wizardCompletedForRouting ? (
              <Route path="*" element={<Navigate to="/setup" replace />} />
            ) : (
              <>
                {/* Developer mode subtree — only active when mode === 'developer' */}
                {mode === "developer" ? (
                  <Route path="/dev" element={<DevLayout />}>
                    <Route index element={<DevOverview />} />
                    <Route path="components" element={<DevComponents />} />
                    <Route path="components/:id" element={<DevComponentDetail />} />
                    <Route path="logs" element={<DevLogs />} />
                    <Route path="manifests" element={<DevManifests />} />
                    <Route path="security" element={<DevSecurity />} />
                    <Route path="allowlist" element={<DevAllowlist />} />
                    <Route path="shell-levels" element={<DevShellLevels />} />
                    <Route path="preferences" element={<DevPreferences />} />
                  </Route>
                ) : (
                  <Route path="/dev/*" element={<Navigate to="/" replace />} />
                )}

                {/* User mode — UserLayout shell with five icon-sidebar routes */}
                <Route element={<UserLayout />}>
                  <Route
                    index
                    element={
                      mode === "developer" ? (
                        <Navigate to="/dev" replace />
                      ) : (
                        <Home />
                      )
                    }
                  />
                  <Route path="/security" element={<SecurityMonitor />} />
                  <Route path="/discover" element={<Discover />} />
                  <Route path="/preferences" element={<Preferences />} />
                  <Route path="/help" element={<Help />} />
                  {/* Back-compat: /settings used to be the user-mode preferences route. */}
                  <Route path="/settings" element={<Navigate to="/preferences" replace />} />
                  <Route path="*" element={<NotFound />} />
                </Route>
              </>
            )}
          </Routes>
        </ErrorBoundary>
      </ToastProvider>
    </AppContextProvider>
  );
}
