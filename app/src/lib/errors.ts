/**
 * Legacy taxonomy — kept stable so existing hooks/components keep working.
 * New code should also read `severity` for the spec-06 failure cascade.
 */
export type ErrorCategory =
  | "timeout"
  | "not_found"
  | "permission"
  | "execution"
  | "config"
  | "parse"
  | "unknown";

/**
 * Spec 06 severity axis — drives Level 1 (silent retry) → Level 2 (FriendlyRetry) → Level 3 (ContactSupport).
 */
export type ErrorSeverity =
  | "transient"
  | "connectivity"
  | "authentication"
  | "configuration"
  | "permissions"
  | "resource"
  | "user-input"
  | "platform"
  | "unknown";

export interface ClassifiedError {
  category: ErrorCategory;
  severity: ErrorSeverity;
  /** Short heading suitable for any failure surface. */
  title: string;
  /** Plain-language sentence safe to show end users (no jargon, no stack frames). */
  userMessage: string;
  /** One-line guidance on what to do next. */
  suggestedAction: string;
  /** Raw error message — only render in dev mode or behind "Show technical details". */
  message: string;
  /** Same content as `message` — explicit alias used by Level 2/3 components. */
  technicalDetails: string;
  retryable: boolean;
}

interface Pattern {
  test: RegExp;
  category: ErrorCategory;
  severity: ErrorSeverity;
  title: string;
  userMessage: string;
  suggestedAction: string;
  retryable: boolean;
}

const PATTERNS: Pattern[] = [
  // Connectivity
  {
    test: /econnrefused|etimedout|enetunreach|getaddrinfo|fetch failed|network error/i,
    category: "execution",
    severity: "connectivity",
    title: "Can't reach the network",
    userMessage: "We couldn't reach the network from your computer.",
    suggestedAction: "Check your wifi connection and try again.",
    retryable: true,
  },
  // Authentication
  {
    test: /\b401\b|\bunauthorized\b|invalid api key|invalid token|invalid credentials/i,
    category: "permission",
    severity: "authentication",
    title: "Your AI key isn't working",
    userMessage: "Your AI key isn't being accepted.",
    suggestedAction: "Open Preferences and update your key.",
    retryable: false,
  },
  // Resource — disk/memory
  {
    test: /enospc|disk full|out of memory|cannot allocate/i,
    category: "execution",
    severity: "resource",
    title: "Your computer is out of space",
    userMessage: "Your computer is out of space or memory.",
    suggestedAction: "Free up some space and try again.",
    retryable: false,
  },
  // Permissions (OS file/process). Match only the explicit OS error tokens —
  // the bare phrase "permission denied" appears in higher-level wrappers
  // (e.g. "Config write error: permission denied") that other patterns own.
  {
    test: /\bEACCES\b|\bEPERM\b/,
    category: "permission",
    severity: "permissions",
    title: "Permission denied",
    userMessage: "OpenTrApp doesn't have permission to do that.",
    suggestedAction: "Check the file permissions and try again.",
    retryable: false,
  },
  // Wizard install pipeline — specific failure shapes, ordered before
  // the generic "execution failed" / "Not found" catch-alls.
  {
    test: /some assistant modules failed to download/i,
    category: "execution",
    severity: "connectivity",
    title: "Couldn't download everything",
    userMessage: "Some parts of your assistant didn't finish downloading.",
    suggestedAction: "Check your wifi and try again.",
    retryable: true,
  },
  {
    test: /workflow ended with status:/i,
    category: "execution",
    severity: "transient",
    title: "Safety check didn't finish",
    userMessage: "One of the safety checks didn't finish.",
    suggestedAction: "Try again — it usually works the second time.",
    retryable: true,
  },
  {
    test: /exited with code/i,
    category: "execution",
    severity: "transient",
    title: "A setup step didn't finish",
    userMessage: "That setup step didn't finish successfully.",
    suggestedAction: "Try again — it usually works the second time.",
    retryable: true,
  },
  // Existing patterns — preserve category/retryable for back-compat
  {
    test: /timed out/i,
    category: "timeout",
    severity: "transient",
    title: "That took too long",
    userMessage: "That took too long to finish.",
    suggestedAction: "Let's try again — it usually works the second time.",
    retryable: true,
  },
  {
    test: /component not found/i,
    category: "not_found",
    severity: "configuration",
    title: "We couldn't find that part of the app",
    userMessage: "We couldn't find that part of the app.",
    suggestedAction: "Try reopening the app.",
    retryable: false,
  },
  {
    test: /command not found/i,
    category: "not_found",
    severity: "configuration",
    title: "We couldn't find what to run",
    userMessage: "We couldn't find what to run.",
    suggestedAction: "Try reopening the app.",
    retryable: false,
  },
  {
    test: /config file not found/i,
    category: "config",
    severity: "configuration",
    title: "Settings file is missing",
    userMessage: "We couldn't find your settings file.",
    suggestedAction: "Re-run setup to recreate it.",
    retryable: false,
  },
  {
    test: /config write error/i,
    category: "config",
    severity: "configuration",
    title: "Couldn't save your settings",
    userMessage: "We couldn't save your settings.",
    suggestedAction: "Make sure your computer has free space and try again.",
    retryable: true,
  },
  {
    test: /shell not found/i,
    category: "execution",
    severity: "platform",
    title: "Shell not available",
    userMessage: "OpenTrApp couldn't find the right tools on your computer.",
    suggestedAction: "Re-run setup to install what's missing.",
    retryable: false,
  },
  {
    test: /path traversal/i,
    category: "permission",
    severity: "permissions",
    title: "Access denied",
    userMessage: "That path isn't allowed for safety reasons.",
    suggestedAction: "Pick a different file or folder.",
    retryable: false,
  },
  {
    test: /manifest parse error/i,
    category: "parse",
    severity: "configuration",
    title: "Settings are invalid",
    userMessage: "One of your assistant's settings files looks broken.",
    suggestedAction: "Re-run setup to restore the defaults.",
    retryable: false,
  },
  {
    test: /execution failed/i,
    category: "execution",
    severity: "transient",
    title: "Something didn't finish",
    userMessage: "That didn't finish successfully.",
    suggestedAction: "Try again in a moment.",
    retryable: true,
  },
  // Catch-all "Not found" goes last so more-specific patterns above win
  {
    test: /not found/i,
    category: "not_found",
    severity: "configuration",
    title: "Not found",
    userMessage: "We couldn't find what you were looking for.",
    suggestedAction: "Try refreshing or reopening the app.",
    retryable: false,
  },
];

const UNKNOWN_FALLBACK: Omit<Pattern, "test"> = {
  category: "unknown",
  severity: "unknown",
  title: "Something went wrong",
  userMessage: "Something didn't work as expected.",
  suggestedAction: "Let's try again — if it keeps happening, get help.",
  retryable: false,
};

/**
 * Optional context tag passed by callers that know which phase of work was
 * running when the error was thrown. Used to make UNKNOWN_FALLBACK copy
 * specific instead of generic — Karen sees "Your computer check didn't work
 * as expected" rather than "Something went wrong".
 */
export type ErrorContext = "check" | "download" | "build" | "safety";

const CONTEXT_FALLBACKS: Record<ErrorContext, Pick<Pattern, "title" | "userMessage" | "suggestedAction">> = {
  check: {
    title: "Computer check didn't work",
    userMessage: "Checking your computer didn't work as expected.",
    suggestedAction: "Let's try again — if it keeps happening, get help.",
  },
  download: {
    title: "Download didn't finish",
    userMessage: "Downloading your assistant didn't work as expected.",
    suggestedAction: "Check your wifi and try again.",
  },
  build: {
    title: "Building didn't finish",
    userMessage: "Building your assistant didn't work as expected.",
    suggestedAction: "Let's try again — if it keeps happening, get help.",
  },
  safety: {
    title: "Safety checks didn't finish",
    userMessage: "Running the safety checks didn't work as expected.",
    suggestedAction: "Let's try again — if it keeps happening, get help.",
  },
};

export function classifyError(err: unknown, context?: ErrorContext): ClassifiedError {
  const message = err instanceof Error ? err.message : String(err);

  for (const pattern of PATTERNS) {
    if (pattern.test.test(message)) {
      return {
        category: pattern.category,
        severity: pattern.severity,
        title: pattern.title,
        userMessage: pattern.userMessage,
        suggestedAction: pattern.suggestedAction,
        message,
        technicalDetails: message,
        retryable: pattern.retryable,
      };
    }
  }

  const fallback = context ? CONTEXT_FALLBACKS[context] : null;
  return {
    category: UNKNOWN_FALLBACK.category,
    severity: UNKNOWN_FALLBACK.severity,
    title: fallback?.title ?? UNKNOWN_FALLBACK.title,
    userMessage: fallback?.userMessage ?? UNKNOWN_FALLBACK.userMessage,
    suggestedAction: fallback?.suggestedAction ?? UNKNOWN_FALLBACK.suggestedAction,
    message,
    technicalDetails: message,
    retryable: UNKNOWN_FALLBACK.retryable,
  };
}
