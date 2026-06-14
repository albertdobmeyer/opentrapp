import { open as openUrl } from "@tauri-apps/plugin-shell";
import { fireEvent, render, screen } from "@testing-library/react";


import TipOfTheDay from "./TipOfTheDay";

const { navigate } = vi.hoisted(() => ({ navigate: vi.fn() }));

let settings = {
  telegramBotUrl: null as string | null,
  telegramBotUsername: null as string | null,
};

vi.mock("@/hooks/useSettings", () => ({ useSettings: () => ({ settings }) }));
vi.mock("react-router-dom", () => ({ useNavigate: () => navigate }));
vi.mock("@tauri-apps/plugin-shell", () => ({ open: vi.fn(() => Promise.resolve()) }));

const mOpen = vi.mocked(openUrl);

beforeEach(() => {
  vi.clearAllMocks();
  settings = { telegramBotUrl: null, telegramBotUsername: null };
});

describe("TipOfTheDay", () => {
  test("renders the tip header and both actions", () => {
    render(<TipOfTheDay />);
    expect(screen.getByText(/tip of the day/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /try this/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /explore more ideas/i })).toBeInTheDocument();
  });

  test("'Try this' opens the prompt in Telegram", () => {
    settings.telegramBotUsername = "my_bot";
    render(<TipOfTheDay />);
    fireEvent.click(screen.getByRole("button", { name: /try this/i }));
    expect(mOpen).toHaveBeenCalledWith(expect.stringMatching(/^https:\/\/t\.me\/my_bot\?text=/));
  });

  test("'Explore more ideas' navigates to /discover", () => {
    render(<TipOfTheDay />);
    fireEvent.click(screen.getByRole("button", { name: /explore more ideas/i }));
    expect(navigate).toHaveBeenCalledWith("/discover");
  });
});
