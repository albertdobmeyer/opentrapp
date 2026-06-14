import { open as openUrl } from "@tauri-apps/plugin-shell";
import { fireEvent, render, screen } from "@testing-library/react";


import Help from "./Help";

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

describe("Help", () => {
  test("renders the help sections", () => {
    render(<Help />);
    expect(screen.getByRole("heading", { name: /help & support/i })).toBeInTheDocument();
    expect(screen.getByText(/where are my keys stored/i)).toBeInTheDocument();
    expect(screen.getByText(/how do i know it's safe/i)).toBeInTheDocument();
  });

  test("hides the Telegram CTA when no bot link is known", () => {
    render(<Help />);
    expect(screen.queryByRole("button", { name: /ask on telegram/i })).not.toBeInTheDocument();
  });

  test("shows and opens the Telegram CTA when a bot username is known", () => {
    settings.telegramBotUsername = "my_bot";
    render(<Help />);
    fireEvent.click(screen.getByRole("button", { name: /ask on telegram/i }));
    expect(mOpen).toHaveBeenCalledWith("https://t.me/my_bot?text=Hi");
  });

  test("the GitHub CTA opens the issues page", () => {
    render(<Help />);
    fireEvent.click(screen.getByRole("button", { name: /report a problem on github/i }));
    expect(mOpen).toHaveBeenCalledWith(expect.stringContaining("github.com/albertdobmeyer/opentrapp/issues"));
  });
});
