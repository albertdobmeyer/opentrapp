import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { readConfig, writeConfig } from "@/lib/tauri";

import { ConfigRow } from "./ConfigRow";

import type { Config } from "@/lib/types";

vi.mock("@/lib/tauri", () => ({
  readConfig: vi.fn(),
  writeConfig: vi.fn(),
}));

const mRead = vi.mocked(readConfig);
const mWrite = vi.mocked(writeConfig);

const cfg = (over: Partial<Config>): Config => ({
  path: "config.yml",
  format: "yaml",
  editable: true,
  danger: "safe",
  restart_required: false,
  ...over,
});

beforeEach(() => vi.clearAllMocks());

describe("ConfigRow", () => {
  test("shows the path and format pill; read-only when not editable", () => {
    render(<ConfigRow componentId="agent" config={cfg({ path: "allowlist.txt", editable: false })} />);
    expect(screen.getByText("allowlist.txt")).toBeInTheDocument();
    expect(screen.getByText("yaml")).toBeInTheDocument();
    expect(screen.getByText("read-only")).toBeInTheDocument();
  });

  test("expanding lazy-loads the content via readConfig", async () => {
    mRead.mockResolvedValue("key: value");
    render(<ConfigRow componentId="agent" config={cfg({})} />);
    fireEvent.click(screen.getByText("config.yml"));
    await waitFor(() => { expect(mRead).toHaveBeenCalledWith("agent", "config.yml"); });
    expect(screen.getByDisplayValue("key: value")).toBeInTheDocument();
  });

  test("editing then Save calls writeConfig and shows 'saved'", async () => {
    mRead.mockResolvedValue("old");
    mWrite.mockResolvedValue(undefined);
    render(<ConfigRow componentId="agent" config={cfg({})} />);
    fireEvent.click(screen.getByText("config.yml"));
    const textarea = await screen.findByDisplayValue("old");

    fireEvent.change(textarea, { target: { value: "new content" } });
    fireEvent.click(screen.getByRole("button", { name: /save/i }));

    await waitFor(() => { expect(mWrite).toHaveBeenCalledWith("agent", "config.yml", "new content"); });
    await waitFor(() => expect(screen.getByText("saved")).toBeInTheDocument());
  });

  test("a read failure surfaces the error", async () => {
    mRead.mockRejectedValue(new Error("permission denied"));
    render(<ConfigRow componentId="agent" config={cfg({})} />);
    fireEvent.click(screen.getByText("config.yml"));
    await waitFor(() => expect(screen.getByText("permission denied")).toBeInTheDocument());
  });

  test("read-only config has no Save button", async () => {
    mRead.mockResolvedValue("frozen");
    render(<ConfigRow componentId="agent" config={cfg({ editable: false })} />);
    fireEvent.click(screen.getByText("config.yml"));
    await screen.findByDisplayValue("frozen");
    expect(screen.queryByRole("button", { name: /save/i })).not.toBeInTheDocument();
  });
});
