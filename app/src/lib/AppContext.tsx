import { useCallback, useMemo } from "react";

import type { AppMode, AppSettings } from "@/lib/settings";

import { AppContext, type AppContextValue } from "@/hooks/useAppContext";


interface ProviderProps {
  settings: AppSettings;
  settingsLoaded: boolean;
  updateSettings: (patch: Partial<AppSettings>) => Promise<void>;
  children: React.ReactNode;
}

/**
 * Wraps AppContext.Provider with the mode helpers derived from settings.
 * Use this instead of AppContext.Provider directly so mode logic stays in
 * one place.
 */
export function AppContextProvider({
  settings,
  settingsLoaded,
  updateSettings,
  children,
}: ProviderProps) {
  const setMode = useCallback(
    async (mode: AppMode) => {
      await updateSettings({ mode });
    },
    [updateSettings],
  );

  const toggleMode = useCallback(async () => {
    const next: AppMode = settings.mode === "developer" ? "user" : "developer";
    await updateSettings({ mode: next });
    return next;
  }, [settings.mode, updateSettings]);

  const markAdvancedModeIntroSeen = useCallback(async () => {
    if (!settings.hasSeenAdvancedModeIntro) {
      await updateSettings({ hasSeenAdvancedModeIntro: true });
    }
  }, [settings.hasSeenAdvancedModeIntro, updateSettings]);

  const value = useMemo<AppContextValue>(
    () => ({
      settings,
      settingsLoaded,
      updateSettings,
      mode: settings.mode,
      setMode,
      toggleMode,
      markAdvancedModeIntroSeen,
    }),
    [
      settings,
      settingsLoaded,
      updateSettings,
      setMode,
      toggleMode,
      markAdvancedModeIntroSeen,
    ],
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}
