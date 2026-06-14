import { fireEvent, render, screen } from "@testing-library/react";

import { MissingRuntimeCard } from "./MissingRuntimeCard";

describe("MissingRuntimeCard", () => {
  test("explains the missing runtime and links to a guide", () => {
    render(<MissingRuntimeCard onBack={vi.fn()} onRetry={vi.fn()} />);
    expect(screen.getByText(/one thing missing/i)).toBeInTheDocument();
    expect(screen.getByText(/sandbox runner/i)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /open guide/i })).toBeInTheDocument();
  });

  test("Back and Check-again invoke their callbacks", () => {
    const onBack = vi.fn();
    const onRetry = vi.fn();
    render(<MissingRuntimeCard onBack={onBack} onRetry={onRetry} />);
    fireEvent.click(screen.getByRole("button", { name: /back/i }));
    fireEvent.click(screen.getByRole("button", { name: /check again/i }));
    expect(onBack).toHaveBeenCalledTimes(1);
    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});
