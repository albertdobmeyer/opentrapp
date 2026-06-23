import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import ModeSwitcher from "./ModeSwitcher";

import { useAppContext } from "@/hooks/useAppContext";


const { navigate } = vi.hoisted(() => ({ navigate: vi.fn() }));

vi.mock("react-router-dom", () => ({
  useNavigate: () => navigate,
  useLocation: () => ({ pathname: "/" }),
}));
vi.mock("@/hooks/useAppContext", () => ({ useAppContext: vi.fn() }));

const toggleMode = vi.fn();
const markSeen = vi.fn(() => Promise.resolve());

function setContext(over: Partial<ReturnType<typeof useAppContext>> = {}) {
  vi.mocked(useAppContext).mockReturnValue({
    mode: "user",
    toggleMode,
    markAdvancedModeIntroSeen: markSeen,
    settings: { hasSeenAdvancedModeIntro: false },
    ...over,
  } as never);
}

function pressToggle() {
  fireEvent.keyDown(window, { key: "D", ctrlKey: true, shiftKey: true });
}

beforeEach(() => {
  vi.clearAllMocks();
  setContext();
});

describe("ModeSwitcher", () => {
  test("renders nothing until the advanced-mode intro opens", () => {
    const { container } = render(<ModeSwitcher />);
    expect(container).toBeEmptyDOMElement();
  });

  test("Ctrl+Shift+D into developer mode navigates to /dev and shows the intro", async () => {
    toggleMode.mockResolvedValue("developer");
    render(<ModeSwitcher />);
    pressToggle();
    await waitFor(() => { expect(navigate).toHaveBeenCalledWith("/dev", { replace: false }); });
    expect(await screen.findByRole("dialog", { name: /welcome to advanced mode/i })).toBeInTheDocument();
  });

  test("does not show the intro again once it has been seen", async () => {
    toggleMode.mockResolvedValue("developer");
    setContext({ settings: { hasSeenAdvancedModeIntro: true } } as never);
    render(<ModeSwitcher />);
    pressToggle();
    await waitFor(() => { expect(navigate).toHaveBeenCalled(); });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  test("'Got it' marks the intro seen and closes it", async () => {
    toggleMode.mockResolvedValue("developer");
    render(<ModeSwitcher />);
    pressToggle();
    await screen.findByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: /got it, let me explore/i }));
    await waitFor(() => { expect(markSeen).toHaveBeenCalled(); });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  test("'Actually, go back' reverts to user mode and navigates back", async () => {
    toggleMode.mockResolvedValueOnce("developer").mockResolvedValueOnce("user");
    render(<ModeSwitcher />);
    pressToggle();
    await screen.findByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: /actually, go back/i }));
    await waitFor(() => { expect(markSeen).toHaveBeenCalled(); });
    // toggled once to enter, once to revert.
    await waitFor(() => { expect(toggleMode).toHaveBeenCalledTimes(2); });
    expect(navigate).toHaveBeenLastCalledWith("/", { replace: true });
  });
});
