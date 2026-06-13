import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { readRuntimeEnv, validateAnthropicKey } from "@/lib/tauri";

import ActivationModal from "./ActivationModal";

// Keep the real tauri surface (its functions call the mocked invoke); only
// override what we drive/assert. The telegram poll loop is gated on the
// "deep_link" phase, so it never runs in these step-1 tests.
vi.mock("@/lib/tauri", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/tauri")>()),
  validateAnthropicKey: vi.fn(),
  readRuntimeEnv: vi.fn(),
}));

const mValidate = vi.mocked(validateAnthropicKey);
const mReadEnv = vi.mocked(readRuntimeEnv);

const VALID_KEY = "sk-ant-api03-abcdefghijklmnop";

beforeEach(() => {
  vi.clearAllMocks();
  mReadEnv.mockResolvedValue("");
});

describe("ActivationModal", () => {
  test("renders the activation dialog on the Anthropic step", () => {
    render(<ActivationModal onClose={vi.fn()} />);
    expect(screen.getByRole("dialog", { name: /launch your assistant/i })).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/sk-ant/i)).toBeInTheDocument();
  });

  test("Escape key closes the modal", () => {
    const onClose = vi.fn();
    render(<ActivationModal onClose={onClose} />);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  test("the Cancel control closes the modal", () => {
    const onClose = vi.fn();
    render(<ActivationModal onClose={onClose} />);
    fireEvent.click(screen.getByRole("button", { name: /cancel/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  test("validating a well-formed key calls validate_anthropic_key and advances to Continue", async () => {
    mValidate.mockResolvedValue("ok");
    render(<ActivationModal onClose={vi.fn()} />);
    fireEvent.change(screen.getByPlaceholderText(/sk-ant/i), { target: { value: VALID_KEY } });
    fireEvent.click(screen.getByRole("button", { name: /validate key/i }));

    await waitFor(() => { expect(mValidate).toHaveBeenCalledWith(VALID_KEY); });
    await waitFor(() => expect(screen.getByRole("button", { name: /continue/i })).toBeInTheDocument());
  });

  test("a validation network failure surfaces an error and does not advance", async () => {
    mValidate.mockRejectedValue(new Error("network down"));
    render(<ActivationModal onClose={vi.fn()} />);
    fireEvent.change(screen.getByPlaceholderText(/sk-ant/i), { target: { value: VALID_KEY } });
    fireEvent.click(screen.getByRole("button", { name: /validate key/i }));

    await waitFor(() => { expect(mValidate).toHaveBeenCalled(); });
    // Still on the Anthropic step (no Continue button appeared).
    expect(screen.queryByRole("button", { name: /continue/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /validate key/i })).toBeInTheDocument();
  });
});
