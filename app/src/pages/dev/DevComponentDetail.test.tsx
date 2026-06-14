import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { getComponent, getStatus } from "@/lib/tauri";

import DevComponentDetail from "./DevComponentDetail";

import type { DiscoveredComponent } from "@/lib/types";


vi.mock("react-router-dom", () => ({ useParams: () => ({ id: "vault-agent" }) }));
vi.mock("@/lib/tauri", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/tauri")>()),
  getComponent: vi.fn(),
  getStatus: vi.fn(),
}));

const mGet = vi.mocked(getComponent);
const mStatus = vi.mocked(getStatus);

const COMPONENT: DiscoveredComponent = {
  component_dir: "/x",
  manifest: {
    identity: { id: "vault-agent", name: "Agent Runtime", version: "1.2.3", description: "Runtime containment", role: "runtime" },
    status: { states: [{ id: "running", label: "Running" }] },
    commands: [],
    configs: [],
    health: [],
    workflows: [],
  },
} as unknown as DiscoveredComponent;

beforeEach(() => { vi.clearAllMocks(); });

describe("DevComponentDetail", () => {
  test("renders the manifest identity once loaded", async () => {
    mGet.mockResolvedValue(COMPONENT);
    mStatus.mockResolvedValue({ state_id: "running" } as never);
    render(<DevComponentDetail />);

    expect(await screen.findByRole("heading", { name: /agent runtime/i })).toBeInTheDocument();
    expect(screen.getByText(/vault-agent · v1\.2\.3/)).toBeInTheDocument();
    // Status panel marks the current state.
    expect(screen.getByText(/running · current/i)).toBeInTheDocument();
  });

  test("still renders the component when the status probe fails (best-effort)", async () => {
    mGet.mockResolvedValue(COMPONENT);
    mStatus.mockRejectedValue(new Error("probe down"));
    render(<DevComponentDetail />);
    expect(await screen.findByRole("heading", { name: /agent runtime/i })).toBeInTheDocument();
  });

  test("shows an error card with Retry when the component fails to load", async () => {
    mGet.mockRejectedValue(new Error("not found"));
    render(<DevComponentDetail />);

    expect(await screen.findByText(/could not load this component/i)).toBeInTheDocument();
    expect(screen.getByText(/not found/)).toBeInTheDocument();

    // Retry re-attempts the load — this time succeeding.
    mGet.mockResolvedValue(COMPONENT);
    mStatus.mockResolvedValue({ state_id: "running" } as never);
    fireEvent.click(screen.getByRole("button", { name: /retry/i }));
    await waitFor(() => { expect(screen.getByRole("heading", { name: /agent runtime/i })).toBeInTheDocument(); });
  });
});
