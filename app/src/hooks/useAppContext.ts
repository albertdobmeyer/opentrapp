import { createContext, useContext } from "react";

import { DEFAULT_SETTINGS } from "@/lib/settings";

import type { AppMode, AppSettings } from "@/lib/settings";

export interface AppContextValue {
  settings: AppSettings;
  settingsLoaded: boolean;
  updateSettings: (patch: Partial<AppSettings>) => Promise<void>;
  mode: AppMode;
  setMode: (mode: AppMode) => Promise<void>;
  toggleMode: () => Promise<AppMode>;
  markAdvancedModeIntroSeen: () => Promise<void>;
}

export const AppContext = createContext<AppContextValue>({
  settings: DEFAULT_SETTINGS,
  settingsLoaded: false,
  updateSettings: () => Promise.resolve(),
  mode: "user",
  setMode: () => Promise.resolve(),
  toggleMode: () => Promise.resolve("user"),
  markAdvancedModeIntroSeen: () => Promise.resolve(),
});

export function useAppContext() {
  return useContext(AppContext);
}
