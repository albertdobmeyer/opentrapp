import { render, screen } from "@testing-library/react";

import WizardProgress from "./WizardProgress";

describe("WizardProgress", () => {
  test("marks the current step and labels every step", () => {
    render(<WizardProgress currentStep="install" completedSteps={["welcome", "connect"]} />);
    expect(screen.getByRole("navigation", { name: /setup progress/i })).toBeInTheDocument();
    // The current step's dot is labelled "(current)".
    expect(screen.getByLabelText(/step 3 of 4: install \(current\)/i)).toBeInTheDocument();
    // A future step is neither current nor done.
    expect(screen.getByLabelText(/^step 4 of 4: ready$/i)).toBeInTheDocument();
  });

  test("steps before the current index render as done", () => {
    render(<WizardProgress currentStep="ready" completedSteps={[]} />);
    // welcome/connect/install precede ready → "(done)" even with empty completedSteps.
    expect(screen.getByLabelText(/step 1 of 4: welcome \(done\)/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/step 4 of 4: ready \(current\)/i)).toBeInTheDocument();
  });
});
