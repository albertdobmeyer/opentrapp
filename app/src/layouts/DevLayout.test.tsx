import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

import DevLayout from "./DevLayout";

const { setMode } = vi.hoisted(() => ({ setMode: vi.fn(() => Promise.resolve()) }));

vi.mock("@/hooks/useAppContext", () => ({ useAppContext: () => ({ setMode }) }));

beforeEach(() => { vi.clearAllMocks(); });

describe("DevLayout", () => {
  test("renders the developer navigation sections and links", () => {
    render(<MemoryRouter><DevLayout /></MemoryRouter>);
    expect(screen.getByRole("navigation", { name: /developer navigation/i })).toBeInTheDocument();
    expect(screen.getByText("System")).toBeInTheDocument();
    expect(screen.getByText("Security")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /all components/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /allowlist/i })).toBeInTheDocument();
  });

  test("'Exit Advanced' switches back to user mode", async () => {
    render(<MemoryRouter><DevLayout /></MemoryRouter>);
    fireEvent.click(screen.getByRole("button", { name: /exit advanced/i }));
    await waitFor(() => { expect(setMode).toHaveBeenCalledWith("user"); });
  });
});
