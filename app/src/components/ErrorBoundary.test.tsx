import { render, screen } from "@testing-library/react";

import { ErrorBoundary } from "./ErrorBoundary";

// ContactSupport (Level 3) pulls in useToast; stub it so the boundary's
// fallback can render in isolation.
vi.mock("@/hooks/useToast", () => ({ useToast: () => ({ addToast: vi.fn() }) }));

function Boom(): never {
  throw new Error("kaboom");
}

beforeEach(() => { vi.clearAllMocks(); });

describe("ErrorBoundary", () => {
  test("renders children when nothing throws", () => {
    render(
      <ErrorBoundary>
        <p>all good</p>
      </ErrorBoundary>,
    );
    expect(screen.getByText("all good")).toBeInTheDocument();
  });

  test("falls back to the support view when a child throws (forceContactSupport)", () => {
    // React logs the caught error; silence it for a clean test run.
    const spy = vi.spyOn(console, "error").mockImplementation(() => undefined);
    try {
      render(
        <ErrorBoundary forceContactSupport fallbackTitle="Setup hit a snag">
          <Boom />
        </ErrorBoundary>,
      );
      expect(screen.getByRole("heading", { name: /setup hit a snag/i })).toBeInTheDocument();
    } finally {
      spy.mockRestore();
    }
  });
});
