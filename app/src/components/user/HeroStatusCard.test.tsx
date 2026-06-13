import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

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
vi.mock("@/lib/tauri", () => ({
  pausePerimeter: vi.fn(),
  resumePerimeter: vi.fn(),
  retryBootstrap: vi.fn(),
}));

function renderHero(state: HeroState) {
  render(
    <MemoryRouter>
      <HeroStatusCard state={state} loading={false} />
    </MemoryRouter>,
  );
}

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
});
