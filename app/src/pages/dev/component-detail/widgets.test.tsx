import { fireEvent, render, screen } from "@testing-library/react";

import { ArgFields, DangerPill, Panel, ResultPanel } from "./widgets";

import type { ArgLike } from "./helpers";
import type { CommandResult } from "@/lib/types";

const arg = (over: Partial<ArgLike>): ArgLike => ({
  id: "x",
  name: "X",
  type: "string",
  required: false,
  options: [],
  ...over,
});

const result = (over: Partial<CommandResult>): CommandResult => ({
  stdout: "",
  stderr: "",
  exit_code: 0,
  duration_ms: 12,
  ...over,
});

describe("Panel", () => {
  test("renders a titled card with children", () => {
    render(
      <Panel title="My Section">
        <span>inner body</span>
      </Panel>,
    );
    expect(screen.getByRole("heading", { name: "My Section" })).toBeInTheDocument();
    expect(screen.getByText("inner body")).toBeInTheDocument();
  });
});

describe("DangerPill", () => {
  test("maps each danger level to its class (destructive/caution/neutral)", () => {
    const { rerender } = render(<DangerPill danger="destructive" />);
    expect(screen.getByText("destructive").className).toContain("pill-danger");
    rerender(<DangerPill danger="caution" />);
    expect(screen.getByText("caution").className).toContain("pill-warning");
    rerender(<DangerPill danger="safe" />);
    expect(screen.getByText("safe").className).toContain("pill-neutral");
  });
});

describe("ArgFields", () => {
  test("renders nothing when there are no args", () => {
    const { container } = render(<ArgFields args={[]} values={{}} onChange={() => undefined} />);
    expect(container.firstChild).toBeNull();
  });

  test("dispatches field type: boolean+enum → selects, string → text input", () => {
    const args = [
      arg({ id: "flag", name: "Flag", type: "boolean" }),
      arg({ id: "mode", name: "Mode", type: "enum", options: ["a", "b"] }),
      arg({ id: "name", name: "Name", type: "string", required: true }),
    ];
    render(<ArgFields args={args} values={{}} onChange={() => undefined} />);
    expect(screen.getAllByRole("combobox")).toHaveLength(2); // boolean + enum
    expect(screen.getAllByRole("textbox")).toHaveLength(1); // string
    // The required arg shows a "*" marker.
    expect(screen.getByText(/Name/).textContent).toContain("*");
  });

  test("number arg renders a number input and onChange merges the new value", () => {
    const onChange = vi.fn();
    render(
      <ArgFields args={[arg({ id: "n", name: "N", type: "number" })]} values={{ n: "1" }} onChange={onChange} />,
    );
    const input = screen.getByDisplayValue("1");
    expect(input).toHaveAttribute("type", "number");
    fireEvent.change(input, { target: { value: "5" } });
    expect(onChange).toHaveBeenCalledWith({ n: "5" });
  });
});

describe("ResultPanel", () => {
  test("exit 0 → safe pill + stdout shown", () => {
    render(<ResultPanel result={result({ exit_code: 0, stdout: "hello-out" })} />);
    expect(screen.getByText("exit 0").className).toContain("pill-safe");
    expect(screen.getByText("hello-out")).toBeInTheDocument();
  });

  test("nonzero exit → danger pill + stderr shown", () => {
    render(<ResultPanel result={result({ exit_code: 2, stderr: "bad-err" })} />);
    expect(screen.getByText("exit 2").className).toContain("pill-danger");
    expect(screen.getByText("bad-err")).toBeInTheDocument();
  });
});
