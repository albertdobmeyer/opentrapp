import { render, screen, waitFor, fireEvent } from "@testing-library/react";

import WeakeningApprovalsCard from "./WeakeningApprovalsCard";

import { listPendingApprovals, approveWeakening } from "@/lib/tauri";


vi.mock("@/lib/tauri", () => ({
  listPendingApprovals: vi.fn(),
  approveWeakening: vi.fn(),
}));
const mockList = vi.mocked(listPendingApprovals);
const mockApprove = vi.mocked(approveWeakening);

beforeEach(() => {
  mockList.mockReset();
  mockApprove.mockReset();
  mockApprove.mockResolvedValue(true);
});

describe("WeakeningApprovalsCard", () => {
  test("renders a held action with friendly, assistant-first copy", async () => {
    mockList.mockResolvedValue([{ id: "pause", verb: "pause" }]);
    render(<WeakeningApprovalsCard />);
    await waitFor(() =>
      expect(screen.getByText("Pause your assistant")).toBeInTheDocument(),
    );
    // no developer vocabulary leaks into the user surface
    expect(screen.queryByText(/perimeter|container|boundary/i)).toBeNull();
  });

  test("Approve needs a confirming second tap, then applies (the held request, by id)", async () => {
    mockList.mockResolvedValue([{ id: "pause", verb: "pause" }]);
    render(<WeakeningApprovalsCard />);
    const approve = await screen.findByText("Approve");

    fireEvent.click(approve); // arms — must NOT apply yet (ADR-0021 friction)
    expect(mockApprove).not.toHaveBeenCalled();
    expect(screen.getByText("Tap again to confirm")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Tap again to confirm")); // confirms
    await waitFor(() => {
      expect(mockApprove).toHaveBeenCalledWith("pause");
    });
  });

  test("shows an empty state when nothing is waiting", async () => {
    mockList.mockResolvedValue([]);
    render(<WeakeningApprovalsCard />);
    await waitFor(() =>
      expect(screen.getByText(/Nothing is waiting/)).toBeInTheDocument(),
    );
  });

  test("degrades honestly when the assistant isn't running", async () => {
    mockList.mockImplementation(() => Promise.reject(new Error("offline")));
    render(<WeakeningApprovalsCard />);
    await waitFor(() =>
      expect(
        screen.getByText(/once your assistant is running/),
      ).toBeInTheDocument(),
    );
  });
});
