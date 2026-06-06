export type AppMode = "user" | "developer";

export type SetupStep = "welcome" | "connect" | "install" | "ready";

export interface NotificationSettings {
  securityAlerts: boolean;
  updates: boolean;
}

export interface SetupProgress {
  step: SetupStep;
  completedSteps: SetupStep[];
  /** Set when user dismissed the keys form via Skip — assistant stays paused until keys added. */
  skippedKeys?: boolean;
}

export type DismissedAlerts = Record<string, number>;

export interface AppSettings {
  monorepoPathOverride: string | null;
  autoRefreshInterval: number;
  wizardCompleted: boolean;
  /** Resume token for the setup wizard. Null when not in progress. */
  setupProgress: SetupProgress | null;
  lastViewedComponentId: string | null;
  mode: AppMode;
  hasSeenAdvancedModeIntro: boolean;
  autostart: boolean;
  notifications: NotificationSettings;
  theme: "dark";
  minimizeToTray: boolean;
  closeToTray: boolean;
  /** Let the assistant sleep to save memory after an idle period, waking on the
   * next Telegram message. Default on — it's what keeps the footprint near zero
   * on small machines. Requires "Keep it running in the background". */
  idleAutoPause: boolean;
  /** Minutes of inactivity before the assistant sleeps. Must comfortably exceed
   * the assistant's own polling cadence so a quiet-but-live assistant isn't
   * mistaken for idle. */
  idleTimeoutMinutes: number;
  /** Use-case ids the user has favourited on the Discover screen. */
  favoriteUseCaseIds: string[];
  /** Alert ids dismissed in the current Tauri-store generation. Cleared on app reinstall. */
  dismissedAlerts: DismissedAlerts;
  /** Cached `https://t.me/{username}?text=Hi` URL derived from the bot token during Install. */
  telegramBotUrl: string | null;
  /** Cached bot @username (without the `@`) — surfaced on Ready as a fallback when the deep-link doesn't auto-route into the right chat. */
  telegramBotUsername: string | null;
}

export const DEFAULT_SETTINGS: AppSettings = {
  monorepoPathOverride: null,
  autoRefreshInterval: 10000,
  wizardCompleted: false,
  setupProgress: null,
  lastViewedComponentId: null,
  mode: "user",
  hasSeenAdvancedModeIntro: false,
  autostart: true,
  notifications: {
    securityAlerts: true,
    updates: true,
  },
  theme: "dark",
  minimizeToTray: false,
  closeToTray: true,
  idleAutoPause: true,
  idleTimeoutMinutes: 12,
  favoriteUseCaseIds: [],
  dismissedAlerts: {},
  telegramBotUrl: null,
  telegramBotUsername: null,
};
