import { fireEvent, render, screen } from "@testing-library/react";

import { useHero, type HeroState } from "@/hooks/useHero";

import Home from "./Home";

vi.mock("@/hooks/useHero", () => ({ useHero: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => undefined)),
}));

// Stub the children so Home's own logic (securityFromHero + activation gating)
// is what we exercise. Each stub surfaces the props we assert on.
vi.mock("@/components/user/ProactiveAlertsBanner", () => ({ default: () => <div>alerts-banner</div> }));
vi.mock("@/components/user/SpendingTile", () => ({ default: () => <div>spending-tile</div> }));
vi.mock("@/components/user/TipOfTheDay", () => ({ default: () => <div>tip-of-the-day</div> }));
vi.mock("@/components/user/HeroStatusCard", () => ({
  default: (p: { onLaunch: () => void }) => (
    <button type="button" onClick={p.onLaunch}>hero-launch</button>
  ),
}));
vi.mock("@/components/user/StatTile", () => ({
  default: (p: { value: string; subline: string; tone: string }) => (
    <div>stat:{p.value}:{p.subline}:{p.tone}</div>
  ),
}));
vi.mock("@/components/ActivationModal", () => ({
  default: (p: { onClose: () => void }) => (
    <div role="dialog">activation-modal<button type="button" onClick={p.onClose}>close-activation</button></div>
  ),
}));

const mHero = vi.mocked(useHero);

function setHero(state: HeroState, loading = false) {
  mHero.mockReturnValue({
    state,
    loading,
    snapshot: { bootstrap_failure: null } as never,
  });
}

beforeEach(() => { vi.clearAllMocks(); });

describe("Home — security cell mapping", () => {
  test.each<[HeroState, string]>([
    ["running_safely", "Safe"],
    ["shell_ready_absent", "Ready"],
    ["shell_failed", "Needs attention"],
    ["paused_by_user", "Stopped"],
    ["dormant", "Sleeping"],
    ["error_perimeter", "Needs attention"],
    ["not_setup", "Not set up"],
  ])("state %s renders security value '%s'", (state, value) => {
    setHero(state);
    render(<Home />);
    expect(screen.getByText(new RegExp(`stat:${value}:`))).toBeInTheDocument();
  });
});

describe("Home — activation modal", () => {
  test("auto-opens activation when the shell is ready but the agent is absent", () => {
    setHero("shell_ready_absent", false);
    render(<Home />);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  test("does not auto-open while still loading", () => {
    setHero("shell_ready_absent", true);
    render(<Home />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  test("the hero launch button opens, and close dismisses, the modal", () => {
    setHero("running_safely", false);
    render(<Home />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /hero-launch/i }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /close-activation/i }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
