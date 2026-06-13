import { act, renderHook, waitFor } from "@testing-library/react";

import { useWizardProgress } from "./useWizardProgress";

// useSettings persists through the plugin-store mock wired in test-setup.ts.
async function mounted() {
  const hook = renderHook(() => useWizardProgress());
  await waitFor(() => { expect(hook.result.current.loaded).toBe(true); });
  return hook;
}

describe("useWizardProgress", () => {
  test("starts with null progress", async () => {
    const { result } = await mounted();
    expect(result.current.progress).toBeNull();
  });

  test("recordStep adds the step to completedSteps and sets it current", async () => {
    const { result } = await mounted();
    await act(async () => {
      await result.current.recordStep("connect");
    });
    await waitFor(() => { expect(result.current.progress?.step).toBe("connect"); });
    expect(result.current.progress?.completedSteps).toContain("connect");
    expect(result.current.progress?.skippedKeys).toBeUndefined();
  });

  test("recordStep accumulates completed steps without duplicating", async () => {
    const { result } = await mounted();
    await act(async () => {
      await result.current.recordStep("welcome");
    });
    await act(async () => {
      await result.current.recordStep("connect");
    });
    await act(async () => {
      await result.current.recordStep("connect"); // repeat
    });
    await waitFor(() => { expect(result.current.progress?.step).toBe("connect"); });
    const completed = result.current.progress?.completedSteps ?? [];
    expect(completed).toEqual(expect.arrayContaining(["welcome", "connect"]));
    // No duplicate of "connect".
    expect(completed.filter((s) => s === "connect")).toHaveLength(1);
  });

  test("recordStep with skippedKeys sets the flag", async () => {
    const { result } = await mounted();
    await act(async () => {
      await result.current.recordStep("install", { skippedKeys: true });
    });
    await waitFor(() => { expect(result.current.progress?.skippedKeys).toBe(true); });
  });

  test("complete clears progress (resolves without throwing)", async () => {
    const { result } = await mounted();
    await act(async () => {
      await result.current.recordStep("ready");
    });
    await waitFor(() => { expect(result.current.progress).not.toBeNull(); });
    await act(async () => {
      await result.current.complete();
    });
    await waitFor(() => { expect(result.current.progress).toBeNull(); });
  });

  test("resetProgress clears progress without completing", async () => {
    const { result } = await mounted();
    await act(async () => {
      await result.current.recordStep("connect");
    });
    await waitFor(() => { expect(result.current.progress).not.toBeNull(); });
    await act(async () => {
      await result.current.resetProgress();
    });
    await waitFor(() => { expect(result.current.progress).toBeNull(); });
  });
});
