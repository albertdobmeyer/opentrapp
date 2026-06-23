import { fireEvent, render, screen } from "@testing-library/react";

import ProactiveAlertsBanner from "./ProactiveAlertsBanner";

import { useAlerts } from "@/hooks/useAlerts";


const { navigate } = vi.hoisted(() => ({ navigate: vi.fn() }));

vi.mock("react-router-dom", () => ({ useNavigate: () => navigate }));
vi.mock("@/hooks/useAlerts", () => ({ useAlerts: vi.fn() }));

const dismiss = vi.fn();

function setAlerts(alerts: unknown[]) {
  vi.mocked(useAlerts).mockReturnValue({ alerts, dismiss } as never);
}

beforeEach(() => { vi.clearAllMocks(); });

describe("ProactiveAlertsBanner", () => {
  test("renders nothing when there are no alerts", () => {
    setAlerts([]);
    const { container } = render(<ProactiveAlertsBanner />);
    expect(container).toBeEmptyDOMElement();
  });

  test("renders an alert with its title, body and CTA", () => {
    setAlerts([
      { id: "a1", severity: "warning", title: "Heads up", body: "Something to know", cta: { label: "Open security", to: "/security" }, dismissable: true },
    ]);
    render(<ProactiveAlertsBanner />);
    expect(screen.getByText("Heads up")).toBeInTheDocument();
    expect(screen.getByText("Something to know")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /open security/i }));
    expect(navigate).toHaveBeenCalledWith("/security");
  });

  test("the dismiss control calls dismiss with the alert id", () => {
    setAlerts([{ id: "a2", severity: "danger", title: "Problem", dismissable: true }]);
    render(<ProactiveAlertsBanner />);
    fireEvent.click(screen.getByRole("button", { name: /dismiss "problem"/i }));
    expect(dismiss).toHaveBeenCalledWith("a2");
  });
});
