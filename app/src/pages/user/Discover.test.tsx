import { open as openUrl } from "@tauri-apps/plugin-shell";
import { fireEvent, render, screen, within } from "@testing-library/react";


import Discover from "./Discover";

const { update } = vi.hoisted(() => ({ update: vi.fn(() => Promise.resolve()) }));

let settings = {
  favoriteUseCaseIds: [] as string[],
  telegramBotUrl: null as string | null,
  telegramBotUsername: null as string | null,
};

vi.mock("@/hooks/useSettings", () => ({
  useSettings: () => ({ settings, update }),
}));
vi.mock("@tauri-apps/plugin-shell", () => ({ open: vi.fn(() => Promise.resolve()) }));

const mOpen = vi.mocked(openUrl);

beforeEach(() => {
  vi.clearAllMocks();
  settings = { favoriteUseCaseIds: [], telegramBotUrl: null, telegramBotUsername: null };
});

describe("Discover", () => {
  test("renders the heading and a grid of use-case cards", () => {
    render(<Discover />);
    expect(screen.getByText(/discover what your assistant can do/i)).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /try this/i }).length).toBeGreaterThan(0);
  });

  test("a non-matching search shows the empty state", () => {
    render(<Discover />);
    fireEvent.change(screen.getByLabelText(/search ideas/i), {
      target: { value: "zzz-no-such-idea-zzz" },
    });
    expect(screen.getByText(/no ideas match that search/i)).toBeInTheDocument();
  });

  test("selecting a category filter keeps the view rendered", () => {
    render(<Discover />);
    const tabs = screen.getAllByRole("tab");
    fireEvent.click(tabs[1]); // a non-"all" category
    expect(tabs[1]).toHaveAttribute("aria-selected", "true");
  });

  test("toggling a favorite persists via settings.update", () => {
    render(<Discover />);
    const firstCard = screen.getAllByRole("button", { name: /try this/i })[0].closest("div");
    const favBtn = within(firstCard as HTMLElement).getByRole("button", { name: /add to favorites/i });
    fireEvent.click(favBtn);
    expect(update).toHaveBeenCalledWith(
      expect.objectContaining({ favoriteUseCaseIds: expect.arrayContaining([expect.any(String)]) }),
    );
  });

  test("'Try this' opens the prompt in Telegram (deep-link via the cached username)", () => {
    settings.telegramBotUsername = "my_assistant_bot";
    render(<Discover />);
    fireEvent.click(screen.getAllByRole("button", { name: /try this/i })[0]);
    expect(mOpen).toHaveBeenCalledWith(
      expect.stringMatching(/^https:\/\/t\.me\/my_assistant_bot\?text=/),
    );
  });

  test("'Try this' falls back to telegram.org when no bot is known", () => {
    render(<Discover />);
    fireEvent.click(screen.getAllByRole("button", { name: /try this/i })[0]);
    expect(mOpen).toHaveBeenCalledWith("https://telegram.org");
  });
});
