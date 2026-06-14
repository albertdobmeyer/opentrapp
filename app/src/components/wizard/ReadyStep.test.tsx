import { open as openUrl } from "@tauri-apps/plugin-shell";
import { act, fireEvent, render, screen } from "@testing-library/react";


import ReadyStep from "./ReadyStep";

let settings = {
  telegramBotUrl: null as string | null,
  telegramBotUsername: null as string | null,
};

vi.mock("@/hooks/useSettings", () => ({ useSettings: () => ({ settings }) }));
vi.mock("@tauri-apps/plugin-shell", () => ({ open: vi.fn(() => Promise.resolve()) }));

const mOpen = vi.mocked(openUrl);

beforeEach(() => {
  vi.clearAllMocks();
  settings = { telegramBotUrl: null, telegramBotUsername: null };
});

describe("ReadyStep", () => {
  test("renders the celebration and both CTAs", () => {
    render(<ReadyStep onGoToDashboard={vi.fn()} />);
    expect(screen.getByText(/your assistant is ready/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /open telegram/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /go to dashboard/i })).toBeInTheDocument();
  });

  test("shows the @username hint when known", () => {
    settings.telegramBotUsername = "my_assistant_bot";
    render(<ReadyStep onGoToDashboard={vi.fn()} />);
    expect(screen.getByText(/@my_assistant_bot/)).toBeInTheDocument();
  });

  test("'Open Telegram' opens the deep-link built from the username", () => {
    settings.telegramBotUsername = "my_assistant_bot";
    render(<ReadyStep onGoToDashboard={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /open telegram/i }));
    expect(mOpen).toHaveBeenCalledWith("https://t.me/my_assistant_bot?text=Hi");
  });

  test("'Go to dashboard' invokes the callback", () => {
    const go = vi.fn();
    render(<ReadyStep onGoToDashboard={go} />);
    fireEvent.click(screen.getByRole("button", { name: /go to dashboard/i }));
    expect(go).toHaveBeenCalledTimes(1);
  });

  test("auto-advances to the dashboard after the countdown elapses", () => {
    vi.useFakeTimers();
    try {
      const go = vi.fn();
      render(<ReadyStep onGoToDashboard={go} />);
      // 3s grace period before the countdown starts, then 5s counting down.
      act(() => { vi.advanceTimersByTime(3000); });
      expect(screen.getByText(/taking you to dashboard/i)).toBeInTheDocument();
      // Each 1s tick re-renders and registers the next timer, so step through
      // them one at a time rather than advancing the whole window at once.
      for (let i = 0; i < 6; i++) {
        act(() => { vi.advanceTimersByTime(1000); });
      }
      expect(go).toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  test("'Stay here' cancels the auto-advance countdown", () => {
    vi.useFakeTimers();
    try {
      const go = vi.fn();
      render(<ReadyStep onGoToDashboard={go} />);
      act(() => { vi.advanceTimersByTime(3000); });
      fireEvent.click(screen.getByRole("button", { name: /stay here/i }));
      act(() => { vi.advanceTimersByTime(10000); });
      expect(go).not.toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });
});
