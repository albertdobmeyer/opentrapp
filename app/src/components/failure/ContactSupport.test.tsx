import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";


import ContactSupport from "./ContactSupport";

import type { ClassifiedError } from "@/lib/errors";


const { addToast } = vi.hoisted(() => ({ addToast: vi.fn() }));

vi.mock("@/hooks/useToast", () => ({ useToast: () => ({ addToast }) }));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: vi.fn(() => Promise.resolve()) }));

const mInvoke = vi.mocked(invoke);

const CLASSIFIED = {
  severity: "error",
  userMessage: "Something went wrong.",
  technicalDetails: "stack trace here",
} as unknown as ClassifiedError;

beforeEach(() => { vi.clearAllMocks(); });

describe("ContactSupport", () => {
  test("renders the support options", () => {
    render(<ContactSupport classified={CLASSIFIED} />);
    expect(screen.getByRole("heading", { name: /still having trouble/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /copy to clipboard/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /email support/i })).toBeInTheDocument();
  });

  test("a custom title override is shown", () => {
    render(<ContactSupport classified={CLASSIFIED} titleOverride="Setup failed" />);
    expect(screen.getByRole("heading", { name: /setup failed/i })).toBeInTheDocument();
  });

  test("copying diagnostics generates the bundle and confirms with a success toast", async () => {
    mInvoke.mockResolvedValue("diagnostic-bundle-text");
    render(<ContactSupport classified={CLASSIFIED} />);
    fireEvent.click(screen.getByRole("button", { name: /copy to clipboard/i }));
    await waitFor(() => { expect(mInvoke).toHaveBeenCalledWith("generate_diagnostic_bundle"); });
    await waitFor(() => {
      expect(addToast).toHaveBeenCalledWith(expect.objectContaining({ type: "success" }));
    });
  });

  test("a diagnostics failure surfaces an error toast", async () => {
    mInvoke.mockRejectedValue(new Error("nope"));
    render(<ContactSupport classified={CLASSIFIED} />);
    fireEvent.click(screen.getByRole("button", { name: /copy to clipboard/i }));
    await waitFor(() => {
      expect(addToast).toHaveBeenCalledWith(expect.objectContaining({ type: "error" }));
    });
  });

  test("technical details toggle reveals the stack trace", () => {
    render(<ContactSupport classified={CLASSIFIED} />);
    expect(screen.queryByText(/stack trace here/)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /show technical details/i }));
    expect(screen.getByText(/stack trace here/)).toBeInTheDocument();
  });

  test("the optional retry callback fires", () => {
    const onRetry = vi.fn();
    render(<ContactSupport classified={CLASSIFIED} onRetry={onRetry} />);
    fireEvent.click(screen.getByRole("button", { name: /try once more/i }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});
