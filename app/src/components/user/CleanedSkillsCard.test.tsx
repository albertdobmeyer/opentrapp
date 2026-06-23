import { render, screen, waitFor } from "@testing-library/react";

import CleanedSkillsCard from "./CleanedSkillsCard";

import { runCommand } from "@/lib/tauri";


vi.mock("@/lib/tauri", () => ({ runCommand: vi.fn() }));
const mockRun = vi.mocked(runCommand);

const result = (stdout: string, exit_code = 0) => ({
  stdout,
  stderr: "",
  exit_code,
  duration_ms: 1,
});

beforeEach(() => mockRun.mockReset());

describe("CleanedSkillsCard", () => {
  test("renders cleaned skills + their plain-language disarm report", async () => {
    mockRun.mockResolvedValue(
      result(
        JSON.stringify({
          cleaned: [
            { skill: "csv-helper", report: "Removed: reads your saved passwords" },
          ],
          count: 1,
        }),
      ),
    );
    render(<CleanedSkillsCard />);
    await waitFor(() =>
      expect(screen.getByText("csv-helper")).toBeInTheDocument(),
    );
    expect(screen.getByText(/reads your saved passwords/)).toBeInTheDocument();
  });

  test("shows an empty state when nothing has needed cleaning", async () => {
    mockRun.mockResolvedValue(result(JSON.stringify({ cleaned: [], count: 0 })));
    render(<CleanedSkillsCard />);
    await waitFor(() =>
      expect(
        screen.getByText(/No skills have needed cleaning yet/),
      ).toBeInTheDocument(),
    );
  });

  test("degrades honestly when the command can't reach the container (non-zero exit)", async () => {
    mockRun.mockResolvedValue(result("", 1));
    render(<CleanedSkillsCard />);
    await waitFor(() =>
      expect(
        screen.getByText(/once your assistant is running/),
      ).toBeInTheDocument(),
    );
  });

  test("degrades honestly when the output can't be parsed", async () => {
    mockRun.mockResolvedValue(result("not json"));
    render(<CleanedSkillsCard />);
    await waitFor(() =>
      expect(
        screen.getByText(/once your assistant is running/),
      ).toBeInTheDocument(),
    );
  });
});
