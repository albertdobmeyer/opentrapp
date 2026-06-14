import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

import { pausePerimeter, resumePerimeter } from "@/lib/tauri";

import HeroStatusCard from "./HeroStatusCard";

import type { HeroState } from "@/hooks/useHero";

vi.mock("@/hooks/useBootstrapProgress", () => ({
  useBootstrapProgress: () => ({
    step: null,
    current: 0,
    total: 7,
    label: null,
    detail: null,
    active: false,
    failed: null,
  }),
}));
vi.mock("@/hooks/useToast", () => ({
  useToast: () => ({ addToast: vi.fn(), removeToast: vi.fn() }),
}));
vi.mock("@tauri-apps/plugin-shell", () => ({ open: vi.fn(() => Promise.resolve()) }));
vi.mock("@/lib/tauri", () => ({
  pausePerimeter: vi.fn(() => Promise.resolve()),
  resumePerimeter: vi.fn(() => Promise.resolve()),
  retryBootstrap: vi.fn(() => Promise.resolve()),
}));

const mPause = vi.mocked(pausePerimeter);
const mResume = vi.mocked(resumePerimeter);

function renderHero(state: HeroState, loading = false) {
  render(
    <MemoryRouter>
      <HeroStatusCard state={state} loading={loading} />
    </MemoryRouter>,
  );
}

beforeEach(() => { vi.clearAllMocks(); });

describe("HeroStatusCard (perimeter-state → user copy)", () => {
  // The user must see the TRUTH about their assistant's state. Pin each
  // state's headline so a wrong/misleading status can't ship silently.
  const cases: [HeroState, RegExp][] = [
    ["running_safely", /running safely/i],
    ["paused_by_user", /assistant is stopped/i],
    ["dormant", /sleeping to save memory/i],
    ["error_perimeter", /didn't recover/i],
    ["error_key", /anthropic key isn't working/i],
    ["shell_failed", /background setup needs your help/i],
    ["not_setup", /isn't set up yet/i],
  ];

  test.each(cases)("state '%s' renders its headline", (state, headline) => {
    renderHero(state);
    expect(screen.getByText(headline)).toBeInTheDocument();
  });

  test("a degraded (danger) state is visibly distinct from running-safely", () => {
    renderHero("error_perimeter");
    // The failure copy must not read like the healthy copy.
    expect(screen.queryByText(/running safely/i)).not.toBeInTheDocument();
    expect(screen.getByText(/didn't recover/i)).toBeInTheDocument();
  });

  test("loading renders the skeleton, not a headline", () => {
    renderHero("running_safely", true);
    expect(screen.queryByText(/running safely/i)).not.toBeInTheDocument();
  });
});

describe("HeroStatusCard interactions", () => {
  test("shell_ready_absent fires onLaunch", () => {
    const onLaunch = vi.fn();
    render(
      <MemoryRouter>
        <HeroStatusCard state="shell_ready_absent" loading={false} onLaunch={onLaunch} />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole("button", { name: /launch your assistant/i }));
    expect(onLaunch).toHaveBeenCalledTimes(1);
  });

  test("running_safely → Stop → confirm pauses the perimeter", async () => {
    renderHero("running_safely");
    fireEvent.click(screen.getByRole("button", { name: /stop your assistant/i }));
    // A confirmation appears; the destructive action is gated behind it.
    fireEvent.click(screen.getByRole("button", { name: /stop now/i }));
    await waitFor(() => { expect(mPause).toHaveBeenCalledTimes(1); });
  });

  test("paused_by_user → Resume brings the perimeter back", async () => {
    renderHero("paused_by_user");
    fireEvent.click(screen.getByRole("button", { name: /resume/i }));
    await waitFor(() => { expect(mResume).toHaveBeenCalledTimes(1); });
  });

  test("dormant offers both Open Telegram and Wake now", () => {
    renderHero("dormant");
    expect(screen.getByRole("button", { name: /open telegram/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /wake now/i })).toBeInTheDocument();
  });
});
