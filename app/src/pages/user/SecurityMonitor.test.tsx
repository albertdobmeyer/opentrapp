import { render, screen } from "@testing-library/react";

import { useHero } from "@/hooks/useHero";

import SecurityMonitor from "./SecurityMonitor";

import type { AssistantStatusSnapshot, BackendAlert } from "@/lib/tauri";

vi.mock("@/hooks/useHero", () => ({ useHero: vi.fn() }));
// Child cards each have their own tauri-backed tests; stub them here so this
// test isolates the SECURITY-STATUS DISPLAY logic (alerts vs. all-healthy).
vi.mock("@/components/user/EgressApprovalsCard", () => ({ default: () => null }));
vi.mock("@/components/user/CleanedSkillsCard", () => ({ default: () => null }));
vi.mock("@/components/user/SentinelActivityBadge", () => ({ default: () => null }));
vi.mock("@tauri-apps/plugin-shell", () => ({ open: vi.fn() }));

const mHero = vi.mocked(useHero);

const mkAlert = (over: Partial<BackendAlert>): BackendAlert => ({
  id: "a",
  severity: "warning",
  title: "Alert title",
  body: null,
  cta_label: null,
  cta_to: null,
  dismissable: true,
  suppress_during_wizard: false,
  ...over,
});

function setHero(alerts: BackendAlert[], loading = false) {
  const snapshot: AssistantStatusSnapshot = {
    status: "ok",
    alerts,
    last_checked_unix_ms: 1,
    bootstrap_failure: null,
  };
  mHero.mockReturnValue({ state: "running_safely", snapshot, loading });
}

beforeEach(() => vi.clearAllMocks());

describe("SecurityMonitor (protection-status surface)", () => {
  test("no alerts + loaded → shows the all-healthy banner", () => {
    setHero([]);
    render(<SecurityMonitor />);
    expect(screen.getByText(/all five layers are running and healthy/i)).toBeInTheDocument();
  });

  test("alerts present → shows the attention banner with each alert, NOT the healthy banner", () => {
    setHero([
      mkAlert({ id: "k", title: "Your Anthropic key isn't working" }),
      mkAlert({ id: "p", title: "Perimeter degraded" }),
    ]);
    render(<SecurityMonitor />);
    expect(screen.getByText(/2 things need your attention/i)).toBeInTheDocument();
    expect(screen.getByText("Your Anthropic key isn't working")).toBeInTheDocument();
    expect(screen.getByText("Perimeter degraded")).toBeInTheDocument();
    expect(screen.queryByText(/all five layers are running and healthy/i)).not.toBeInTheDocument();
  });

  test("a single alert uses the singular phrasing", () => {
    setHero([mkAlert({ id: "one", title: "One issue" })]);
    render(<SecurityMonitor />);
    expect(screen.getByText(/1 thing needs your attention/i)).toBeInTheDocument();
  });

  test("the five defense-in-depth layers always render", () => {
    setHero([]);
    render(<SecurityMonitor />);
    expect(screen.getByText("Assistant runs walled off")).toBeInTheDocument();
    expect(screen.getByText("Every skill is scanned")).toBeInTheDocument();
    expect(screen.getByText(/api key is never seen/i)).toBeInTheDocument();
    expect(screen.getByText(/only allowlisted destinations/i)).toBeInTheDocument();
    expect(screen.getByText(/blocked at the kernel level/i)).toBeInTheDocument();
  });
});
