import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

import Preferences from "./Preferences";

import { readRuntimeEnv, restartPerimeter, saveCredentials } from "@/lib/tauri";


const { addToast, removeToast } = vi.hoisted(() => ({
  addToast: vi.fn(() => "toast-id"),
  removeToast: vi.fn(),
}));

vi.mock("@/lib/tauri", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/tauri")>()),
  saveCredentials: vi.fn(),
  readRuntimeEnv: vi.fn(),
  restartPerimeter: vi.fn(),
}));
vi.mock("@/hooks/useToast", () => ({ useToast: () => ({ addToast, removeToast }) }));
vi.mock("@/lib/osIntegration", () => ({
  getAutostartEnabled: vi.fn(() => Promise.resolve(false)),
  setAutostartEnabled: vi.fn(() => Promise.resolve(false)),
  ensureNotificationPermission: vi.fn(() => Promise.resolve("unavailable")),
}));

const mSave = vi.mocked(saveCredentials);
const mReadEnv = vi.mocked(readRuntimeEnv);
const mRestart = vi.mocked(restartPerimeter);

const VALID = "sk-ant-api03-abcdefghijklmnop";

beforeEach(() => {
  vi.clearAllMocks();
  mReadEnv.mockResolvedValue(""); // no keys set → "Set" buttons
});

// Open the Anthropic key editor (the first key row) and return its input.
async function openAnthropicEditor() {
  render(
    <MemoryRouter>
      <Preferences />
    </MemoryRouter>,
  );
  // Both rows render "Set"; Anthropic is the first.
  const setButtons = await screen.findAllByRole("button", { name: /^set$/i });
  fireEvent.click(setButtons[0]);
  return screen.getByPlaceholderText(/sk-ant/i);
}

describe("Preferences key rotation", () => {
  test("rotating the Anthropic key saves it, restarts the perimeter, and confirms", async () => {
    mSave.mockResolvedValue(undefined);
    mRestart.mockResolvedValue(undefined);
    const input = await openAnthropicEditor();

    fireEvent.change(input, { target: { value: VALID } });
    fireEvent.click(screen.getByRole("button", { name: /^save$/i }));

    await waitFor(() => { expect(mSave).toHaveBeenCalledWith(VALID, ""); });
    await waitFor(() => { expect(mRestart).toHaveBeenCalledTimes(1); });
    // A success toast confirms the new key is live.
    await waitFor(() =>
      { expect(addToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: "success" }),
      ); },
    );
  });

  test("an invalid key format is rejected before any save", async () => {
    const input = await openAnthropicEditor();
    fireEvent.change(input, { target: { value: "not-a-key" } });
    fireEvent.click(screen.getByRole("button", { name: /^save$/i }));

    await waitFor(() =>
      { expect(addToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: "error", title: "That doesn't look right" }),
      ); },
    );
    expect(mSave).not.toHaveBeenCalled();
  });

  test("a save failure surfaces a 'couldn't save' error and does not restart", async () => {
    mSave.mockRejectedValue(new Error("disk full"));
    const input = await openAnthropicEditor();
    fireEvent.change(input, { target: { value: VALID } });
    fireEvent.click(screen.getByRole("button", { name: /^save$/i }));

    await waitFor(() =>
      { expect(addToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: "error" }),
      ); },
    );
    expect(mRestart).not.toHaveBeenCalled();
  });
});
