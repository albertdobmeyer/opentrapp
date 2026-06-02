import { render, screen, waitFor, fireEvent } from "@testing-library/react";

import { listEgressApprovals, applyAllowlistDecision } from "@/lib/tauri";

import EgressApprovalsCard from "./EgressApprovalsCard";

vi.mock("@/lib/tauri", () => ({
  listEgressApprovals: vi.fn(),
  applyAllowlistDecision: vi.fn(),
}));
const mockList = vi.mocked(listEgressApprovals);
const mockApply = vi.mocked(applyAllowlistDecision);

const approval = (host: string, reason = "looks fine") => ({
  host,
  reason,
  judged_at_ms: 1,
});

beforeEach(() => {
  mockList.mockReset();
  mockApply.mockReset();
  mockApply.mockResolvedValue(undefined);
});

describe("EgressApprovalsCard", () => {
  test("renders each pending host with the judge's plain-language reason", async () => {
    mockList.mockResolvedValue([
      approval("weather.example.com", "a weather site, consistent with your task"),
    ]);
    render(<EgressApprovalsCard />);
    await waitFor(() =>
      expect(screen.getByText("weather.example.com")).toBeInTheDocument(),
    );
    expect(
      screen.getByText(/consistent with your task/),
    ).toBeInTheDocument();
  });

  test("Allow always needs a confirming second tap, then applies 'always'", async () => {
    mockList.mockResolvedValue([approval("news.example.com")]);
    render(<EgressApprovalsCard />);
    const allow = await screen.findByText("Allow always");

    fireEvent.click(allow); // arms — does NOT apply yet
    expect(mockApply).not.toHaveBeenCalled();
    expect(screen.getByText("Tap again to confirm")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Tap again to confirm")); // confirms
    await waitFor(() =>
      expect(mockApply).toHaveBeenCalledWith("news.example.com", "always"),
    );
  });

  test("Block applies 'deny' (no allowlist write) in one tap", async () => {
    mockList.mockResolvedValue([approval("tracker.example.com")]);
    render(<EgressApprovalsCard />);
    const block = await screen.findByText("Block");
    fireEvent.click(block);
    await waitFor(() =>
      expect(mockApply).toHaveBeenCalledWith("tracker.example.com", "deny"),
    );
  });

  test("shows an empty state when nothing is waiting", async () => {
    mockList.mockResolvedValue([]);
    render(<EgressApprovalsCard />);
    await waitFor(() =>
      expect(screen.getByText(/Nothing is waiting/)).toBeInTheDocument(),
    );
  });

  test("degrades honestly when the assistant isn't running", async () => {
    mockList.mockImplementation(async () => {
      throw new Error("offline");
    });
    render(<EgressApprovalsCard />);
    await waitFor(() =>
      expect(
        screen.getByText(/once your assistant is running/),
      ).toBeInTheDocument(),
    );
  });
});
