import { render, screen, fireEvent, act } from "@testing-library/react";

import { ToastProvider } from "./ToastContext";

import { useToast } from "@/hooks/useToast";


// Helper component to trigger toasts
function ToastTrigger(props: {
  type: "success" | "error" | "warning" | "info";
  title: string;
  message?: string;
  details?: string;
  duration?: number;
  retryFn?: () => void;
}) {
  const { addToast } = useToast();
  return (
    <button onClick={() => addToast(props)}>
      Trigger
    </button>
  );
}

describe("Toast system", () => {
  test("renders a toast with title and message", () => {
    render(
      <ToastProvider>
        <ToastTrigger type="error" title="Error!" message="Something failed" duration={0} />
      </ToastProvider>,
    );

    fireEvent.click(screen.getByText("Trigger"));
    expect(screen.getByText("Error!")).toBeInTheDocument();
    expect(screen.getByText("Something failed")).toBeInTheDocument();
  });

  test("dismisses toast on close button", () => {
    render(
      <ToastProvider>
        <ToastTrigger type="info" title="Info toast" duration={0} />
      </ToastProvider>,
    );

    fireEvent.click(screen.getByText("Trigger"));
    expect(screen.getByText("Info toast")).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText("Dismiss"));
    expect(screen.queryByText("Info toast")).not.toBeInTheDocument();
  });

  test("auto-dismisses after duration", () => {
    vi.useFakeTimers();

    render(
      <ToastProvider>
        <ToastTrigger type="success" title="Quick toast" duration={2000} />
      </ToastProvider>,
    );

    fireEvent.click(screen.getByText("Trigger"));
    expect(screen.getByText("Quick toast")).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(2500);
    });

    expect(screen.queryByText("Quick toast")).not.toBeInTheDocument();

    vi.useRealTimers();
  });

  test("shows expandable details", () => {
    render(
      <ToastProvider>
        <ToastTrigger
          type="error"
          title="Error"
          details="Stack trace here"
          duration={0}
        />
      </ToastProvider>,
    );

    fireEvent.click(screen.getByText("Trigger"));
    expect(screen.queryByText("Stack trace here")).not.toBeInTheDocument();

    fireEvent.click(screen.getByText("Show details"));
    expect(screen.getByText("Stack trace here")).toBeInTheDocument();
  });

  test("calls retry function and dismisses", () => {
    const retryFn = vi.fn();

    render(
      <ToastProvider>
        <ToastTrigger type="error" title="Failed" retryFn={retryFn} duration={0} />
      </ToastProvider>,
    );

    fireEvent.click(screen.getByText("Trigger"));
    fireEvent.click(screen.getByText("Retry"));

    expect(retryFn).toHaveBeenCalledOnce();
    expect(screen.queryByText("Failed")).not.toBeInTheDocument();
  });
});
