import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import {
  commitActivation,
  deriveTelegramBotUrl,
  readRuntimeEnv,
  saveCredentials,
  telegramDeleteWebhook,
  telegramPollForStart,
  validateAnthropicKey,
} from "@/lib/tauri";

import ActivationModal from "./ActivationModal";

// Keep the real tauri surface (its functions call the mocked invoke); only
// override what we drive/assert.
vi.mock("@/lib/tauri", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/tauri")>()),
  validateAnthropicKey: vi.fn(),
  readRuntimeEnv: vi.fn(),
  deriveTelegramBotUrl: vi.fn(),
  telegramDeleteWebhook: vi.fn(),
  telegramPollForStart: vi.fn(),
  saveCredentials: vi.fn(),
  commitActivation: vi.fn(),
}));
vi.mock("@/hooks/useSettings", () => ({
  useSettings: () => ({ update: vi.fn(() => Promise.resolve()) }),
}));

const mValidate = vi.mocked(validateAnthropicKey);
const mReadEnv = vi.mocked(readRuntimeEnv);
const mDeriveBot = vi.mocked(deriveTelegramBotUrl);
const mDeleteWebhook = vi.mocked(telegramDeleteWebhook);
const mPoll = vi.mocked(telegramPollForStart);
const mSave = vi.mocked(saveCredentials);
const mCommit = vi.mocked(commitActivation);

const VALID_KEY = "sk-ant-api03-abcdefghijklmnop";
const VALID_TELEGRAM = "1234567890:ABCdefGHIjklmnopQRSTuvwxyz012345678";

// Drive through step 1 (Anthropic) onto step 2 (Telegram).
async function advanceToTelegramStep(onClose: () => void = vi.fn()) {
  mValidate.mockResolvedValue("ok");
  render(<ActivationModal onClose={onClose} />);
  fireEvent.change(screen.getByPlaceholderText(/sk-ant/i), { target: { value: VALID_KEY } });
  fireEvent.click(screen.getByRole("button", { name: /validate key/i }));
  fireEvent.click(await screen.findByRole("button", { name: /continue/i }));
  return screen.getByPlaceholderText(/1234567890/);
}

// Drive Anthropic → Telegram → validate → reach the deep-link 'Start' screen.
async function reachDeepLink(onClose: () => void = vi.fn()) {
  mDeriveBot.mockResolvedValue({ username: "my_bot", url: "https://t.me/my_bot?text=Hi" });
  mDeleteWebhook.mockResolvedValue(undefined);
  const input = await advanceToTelegramStep(onClose);
  fireEvent.change(input, { target: { value: VALID_TELEGRAM } });
  fireEvent.click(screen.getByRole("button", { name: /validate bot/i }));
  await screen.findByRole("button", { name: /open @my_bot in telegram/i });
}

beforeEach(() => {
  vi.clearAllMocks();
  mReadEnv.mockResolvedValue("");
  // The poll loop (deep_link phase) awaits this — keep it pending so it never
  // resolves and the test inspects the deep-link UI without the loop advancing.
  mPoll.mockReturnValue(new Promise(() => { /* never resolves */ }));
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

  test("an auth_failure outcome shows the key-rejected message", async () => {
    mValidate.mockResolvedValue("auth_failure");
    render(<ActivationModal onClose={vi.fn()} />);
    fireEvent.change(screen.getByPlaceholderText(/sk-ant/i), { target: { value: VALID_KEY } });
    fireEvent.click(screen.getByRole("button", { name: /validate key/i }));
    expect(await screen.findByText(/isn't being accepted/i)).toBeInTheDocument();
  });

  test("pasting a well-formed key populates the input", () => {
    render(<ActivationModal onClose={vi.fn()} />);
    const input = screen.getByPlaceholderText(/sk-ant/i);
    fireEvent.paste(input, { clipboardData: { getData: () => VALID_KEY } });
    expect(input).toHaveValue(VALID_KEY);
  });

  test("the show/hide toggle reveals the key", () => {
    render(<ActivationModal onClose={vi.fn()} />);
    const input = screen.getByPlaceholderText(/sk-ant/i);
    expect(input).toHaveAttribute("type", "password");
    fireEvent.click(screen.getByRole("button", { name: /show key/i }));
    expect(input).toHaveAttribute("type", "text");
  });

  test("the how-to link opens the Anthropic walkthrough", () => {
    render(<ActivationModal onClose={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /show me how to get one/i }));
    expect(screen.getByText(/open the anthropic console/i)).toBeInTheDocument();
  });

  test("advancing to the Telegram step renders the bot-token input", async () => {
    const input = await advanceToTelegramStep();
    expect(input).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /telegram bot/i })).toBeInTheDocument();
  });

  test("validating a Telegram token reaches the deep-link 'Start' prompt", async () => {
    mDeriveBot.mockResolvedValue({ username: "my_bot", url: "https://t.me/my_bot?text=Hi" });
    mDeleteWebhook.mockResolvedValue(undefined);
    const input = await advanceToTelegramStep();
    fireEvent.change(input, { target: { value: VALID_TELEGRAM } });
    fireEvent.click(screen.getByRole("button", { name: /validate bot/i }));

    await waitFor(() => { expect(mDeriveBot).toHaveBeenCalledWith(VALID_TELEGRAM); });
    expect(await screen.findByRole("button", { name: /open @my_bot in telegram/i })).toBeInTheDocument();
  });

  test("a Telegram validation failure surfaces an error", async () => {
    mDeriveBot.mockRejectedValue(new Error("bad token"));
    const input = await advanceToTelegramStep();
    fireEvent.change(input, { target: { value: VALID_TELEGRAM } });
    fireEvent.click(screen.getByRole("button", { name: /validate bot/i }));

    await waitFor(() => { expect(mDeriveBot).toHaveBeenCalled(); });
    // Stayed on the token-entry view (Validate bot still present).
    expect(screen.getByRole("button", { name: /validate bot/i })).toBeInTheDocument();
  });

  test("'Skip and test later' saves the credentials, commits, and closes", async () => {
    mSave.mockResolvedValue(undefined);
    mCommit.mockResolvedValue(undefined);
    const onClose = vi.fn();
    await reachDeepLink(onClose);
    fireEvent.click(screen.getByRole("button", { name: /skip and test later/i }));

    await waitFor(() => { expect(mSave).toHaveBeenCalledWith(VALID_KEY, VALID_TELEGRAM); });
    await waitFor(() => { expect(mCommit).toHaveBeenCalledTimes(1); });
    await waitFor(() => { expect(onClose).toHaveBeenCalledTimes(1); });
  });

  test("a save failure surfaces an error and does not close", async () => {
    mSave.mockRejectedValue(new Error("disk full"));
    const onClose = vi.fn();
    await reachDeepLink(onClose);
    fireEvent.click(screen.getByRole("button", { name: /skip and test later/i }));

    await waitFor(() => { expect(mSave).toHaveBeenCalled(); });
    expect(mCommit).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  test("re-credential mode prefills the Telegram token from the runtime env", async () => {
    mReadEnv.mockResolvedValue(`ANTHROPIC_API_KEY=sk-old\nTELEGRAM_BOT_TOKEN=${VALID_TELEGRAM}\n`);
    mValidate.mockResolvedValue("ok");
    mSave.mockResolvedValue(undefined);
    mCommit.mockResolvedValue(undefined);
    const onClose = vi.fn();
    render(<ActivationModal onClose={onClose} reCredential />);

    fireEvent.change(screen.getByPlaceholderText(/sk-ant/i), { target: { value: VALID_KEY } });
    fireEvent.click(screen.getByRole("button", { name: /validate key/i }));
    // With a prefilled token, the Anthropic CTA commits directly.
    fireEvent.click(await screen.findByRole("button", { name: /launch my assistant/i }));
    await waitFor(() => { expect(mSave).toHaveBeenCalledWith(VALID_KEY, VALID_TELEGRAM); });
  });
});
