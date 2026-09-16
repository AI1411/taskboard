import { act, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Toast } from "./Toast";

describe("Toast", () => {
  it("dismisses after 2.4s and errors after 8s", async () => {
    vi.useFakeTimers();
    try {
      const onDismiss = vi.fn();
      const { rerender } = render(<Toast message="Hi" onDismiss={onDismiss} />);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(2399);
      });
      expect(onDismiss).not.toHaveBeenCalled();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1);
      });
      expect(onDismiss).toHaveBeenCalledTimes(1);

      const onErrorDismiss = vi.fn();
      rerender(<Toast message="Boom" error onDismiss={onErrorDismiss} />);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(7999);
      });
      expect(onErrorDismiss).not.toHaveBeenCalled();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1);
      });
      expect(onErrorDismiss).toHaveBeenCalledTimes(1);
      expect(screen.getByRole("button", { name: "Dismiss" })).toBeTruthy();
    } finally {
      vi.useRealTimers();
    }
  });
});
