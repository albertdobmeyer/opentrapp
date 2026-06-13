import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

import App from "./App";

// Stub the heavy route targets so this test isolates the ROUTING GUARD logic
// (does an un-set-up user get forced to /setup?) rather than every page.
vi.mock("@/pages/Setup", () => ({ default: () => <div>SETUP PAGE</div> }));
vi.mock("@/pages/user/Home", () => ({ default: () => <div>HOME PAGE</div> }));
vi.mock("@/components/UserLayout", async () => {
  const { Outlet } = await import("react-router-dom");
  return { default: () => <Outlet /> };
});
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => undefined)),
}));

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App />
    </MemoryRouter>,
  );
}

afterEach(() => {
  delete (window as unknown as { __OPENTRAPP_DEMO__?: boolean }).__OPENTRAPP_DEMO__;
});

describe("App routing guard (setup gate)", () => {
  test("a fresh (wizard-incomplete) user on '/' is forced to /setup", async () => {
    // Default settings → wizardCompleted = false.
    renderAt("/");
    await waitFor(() => expect(screen.getByText("SETUP PAGE")).toBeInTheDocument());
    expect(screen.queryByText("HOME PAGE")).not.toBeInTheDocument();
  });

  test("a deep protected path also redirects to /setup before setup is done", async () => {
    renderAt("/security");
    await waitFor(() => expect(screen.getByText("SETUP PAGE")).toBeInTheDocument());
  });

  test("/setup is always reachable", async () => {
    renderAt("/setup");
    await waitFor(() => expect(screen.getByText("SETUP PAGE")).toBeInTheDocument());
  });

  test("once setup is complete, '/' renders Home (guard lifts)", async () => {
    // The demo override flips wizardCompletedForRouting true without seeding the store.
    (window as unknown as { __OPENTRAPP_DEMO__?: boolean }).__OPENTRAPP_DEMO__ = true;
    renderAt("/");
    await waitFor(() => expect(screen.getByText("HOME PAGE")).toBeInTheDocument());
    expect(screen.queryByText("SETUP PAGE")).not.toBeInTheDocument();
  });
});
