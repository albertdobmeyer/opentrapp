import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

import Setup from "./Setup";

import { useWizardProgress } from "@/hooks/useWizardProgress";


const { navigate } = vi.hoisted(() => ({ navigate: vi.fn() }));

vi.mock("react-router-dom", async (importOriginal) => ({
  ...(await importOriginal<typeof import("react-router-dom")>()),
  useNavigate: () => navigate,
}));

vi.mock("@/hooks/useWizardProgress", () => ({ useWizardProgress: vi.fn() }));
vi.mock("@/hooks/useSettings", () => ({
  useSettings: () => ({ settings: { wizardCompleted: false }, loaded: true }),
}));

// Stub the steps so we drive Setup's own step-machine via their callbacks.
vi.mock("@/components/wizard/WizardProgress", () => ({ default: () => <div>progress-bar</div> }));
vi.mock("@/components/wizard/WelcomeStep", () => ({
  default: (p: { onNext: () => void }) => <button type="button" onClick={p.onNext}>welcome-next</button>,
}));
vi.mock("@/components/wizard/ConnectStep", () => ({
  default: (p: { onContinue: (a: { skippedKeys: boolean }) => void; onBack: () => void }) => (
    <>
      <button type="button" onClick={() => { p.onContinue({ skippedKeys: false }); }}>connect-continue</button>
      <button type="button" onClick={p.onBack}>connect-back</button>
    </>
  ),
}));
vi.mock("@/components/wizard/InstallStep", () => ({
  default: (p: { onComplete: () => void }) => <button type="button" onClick={p.onComplete}>install-complete</button>,
}));
vi.mock("@/components/wizard/ReadyStep", () => ({
  default: (p: { onGoToDashboard: () => void }) => <button type="button" onClick={p.onGoToDashboard}>ready-finish</button>,
}));

const recordStep = vi.fn(() => Promise.resolve());
const complete = vi.fn(() => Promise.resolve());

function mountAt(step: string | undefined) {
  vi.mocked(useWizardProgress).mockReturnValue({
    progress: step ? { step, completedSteps: [] } : null,
    loaded: true,
    recordStep,
    complete,
  } as never);
  return render(<MemoryRouter><Setup /></MemoryRouter>);
}

beforeEach(() => { vi.clearAllMocks(); });

describe("Setup wizard step machine", () => {
  test("starts on the welcome step", async () => {
    mountAt(undefined);
    expect(await screen.findByRole("button", { name: /welcome-next/i })).toBeInTheDocument();
  });

  test("advancing from welcome records the connect step", async () => {
    mountAt(undefined);
    fireEvent.click(await screen.findByRole("button", { name: /welcome-next/i }));
    expect(await screen.findByRole("button", { name: /connect-continue/i })).toBeInTheDocument();
    await waitFor(() => { expect(recordStep).toHaveBeenCalledWith("connect", undefined); });
  });

  test("resumes at the persisted step", async () => {
    mountAt("install");
    expect(await screen.findByRole("button", { name: /install-complete/i })).toBeInTheDocument();
  });

  test("Back from the connect step returns to welcome and records it", async () => {
    mountAt("connect");
    fireEvent.click(await screen.findByRole("button", { name: /connect-back/i }));
    expect(await screen.findByRole("button", { name: /welcome-next/i })).toBeInTheDocument();
    await waitFor(() => { expect(recordStep).toHaveBeenCalledWith("welcome"); });
  });

  test("finishing from the ready step completes and navigates home", async () => {
    mountAt("ready");
    fireEvent.click(await screen.findByRole("button", { name: /ready-finish/i }));
    await waitFor(() => { expect(complete).toHaveBeenCalledTimes(1); });
    await waitFor(() => { expect(navigate).toHaveBeenCalledWith("/"); });
  });
});
