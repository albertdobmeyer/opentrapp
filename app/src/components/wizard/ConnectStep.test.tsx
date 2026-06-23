import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import ConnectStep from "./ConnectStep";

import { readRuntimeEnv, saveCredentials } from "@/lib/tauri";


const { addToast } = vi.hoisted(() => ({ addToast: vi.fn() }));

vi.mock("@/lib/tauri", () => ({
  saveCredentials: vi.fn(),
  readRuntimeEnv: vi.fn(),
}));
vi.mock("@/hooks/useToast", () => ({ useToast: () => ({ addToast }) }));

const mSave = vi.mocked(saveCredentials);
const mReadEnv = vi.mocked(readRuntimeEnv);

beforeEach(() => {
  vi.clearAllMocks();
  mReadEnv.mockResolvedValue(""); // no pre-existing keys in .env
});

function renderStep() {
  const onContinue = vi.fn();
  const onBack = vi.fn();
  render(<ConnectStep onContinue={onContinue} onBack={onBack} />);
  return { onContinue, onBack };
}

describe("ConnectStep credential flow", () => {
  test("Continue is disabled until a key is entered", () => {
    renderStep();
    const cont = screen.getByRole("button", { name: /continue/i });
    expect(cont).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/anthropic api key/i), {
      target: { value: "sk-ant-abc" },
    });
    expect(screen.getByRole("button", { name: /continue/i })).toBeEnabled();
  });

  test("entering a key + Continue saves credentials then advances (not skipped)", async () => {
    mSave.mockResolvedValue(undefined);
    const { onContinue } = renderStep();
    fireEvent.change(screen.getByLabelText(/anthropic api key/i), {
      target: { value: "sk-ant-key" },
    });
    fireEvent.click(screen.getByRole("button", { name: /continue/i }));

    await waitFor(() => { expect(mSave).toHaveBeenCalledWith("sk-ant-key", ""); });
    expect(onContinue).toHaveBeenCalledWith({ skippedKeys: false });
  });

  test("Skip advances WITHOUT saving credentials", () => {
    const { onContinue } = renderStep();
    fireEvent.click(screen.getByRole("button", { name: /skip/i }));
    expect(mSave).not.toHaveBeenCalled();
    expect(onContinue).toHaveBeenCalledWith({ skippedKeys: true });
  });

  test("a save FAILURE surfaces an error toast and does NOT advance (the v0.6 dead-end guard)", async () => {
    mSave.mockRejectedValue(new Error("read-only bundle"));
    const { onContinue } = renderStep();
    fireEvent.change(screen.getByLabelText(/anthropic api key/i), {
      target: { value: "sk-ant-key" },
    });
    fireEvent.click(screen.getByRole("button", { name: /continue/i }));

    await waitFor(() => { expect(addToast).toHaveBeenCalled(); });
    expect(addToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: "error", duration: 0 }),
    );
    // Critically: the wizard must NOT advance on a failed save — the user can retry.
    expect(onContinue).not.toHaveBeenCalled();
  });

  test("pre-existing keys from .env render as a masked value", async () => {
    mReadEnv.mockResolvedValue("ANTHROPIC_API_KEY=sk-ant-existing12345\n");
    renderStep();
    // The masked existing key shows a "Change" affordance instead of an input.
    await waitFor(() => expect(screen.getByText(/change/i)).toBeInTheDocument());
  });

  test("entering both keys + Continue saves both", async () => {
    mSave.mockResolvedValue(undefined);
    const { onContinue } = renderStep();
    fireEvent.change(screen.getByLabelText(/anthropic api key/i), { target: { value: "sk-ant-key" } });
    fireEvent.change(screen.getByLabelText(/telegram bot/i), { target: { value: "1234567890:ABCdef" } });
    fireEvent.click(screen.getByRole("button", { name: /continue/i }));
    await waitFor(() => { expect(mSave).toHaveBeenCalledWith("sk-ant-key", "1234567890:ABCdef"); });
    expect(onContinue).toHaveBeenCalledWith({ skippedKeys: false });
  });

  test("pasting an Anthropic key into the Telegram field re-routes it to the right field", () => {
    renderStep();
    const telegramInput = screen.getByLabelText(/telegram bot/i);
    fireEvent.paste(telegramInput, {
      clipboardData: { getData: () => "sk-ant-api03-pastedkey1234567890" },
    });
    expect(screen.getByLabelText(/anthropic api key/i)).toHaveValue("sk-ant-api03-pastedkey1234567890");
  });

  test("the show/hide toggle reveals the Anthropic key", () => {
    renderStep();
    const input = screen.getByLabelText(/anthropic api key/i);
    expect(input).toHaveAttribute("type", "password");
    // The first "Show key" toggle belongs to the Anthropic card.
    fireEvent.click(screen.getAllByRole("button", { name: /show key/i })[0]);
    expect(input).toHaveAttribute("type", "text");
  });

  test("the how-to link opens the Anthropic walkthrough modal", () => {
    renderStep();
    fireEvent.click(screen.getByRole("button", { name: /show me how to get one/i }));
    expect(screen.getByText(/open the anthropic console/i)).toBeInTheDocument();
  });
});
