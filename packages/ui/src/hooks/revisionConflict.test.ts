import { TransportError } from "@taskboard/client";
import { describe, expect, it, vi } from "vitest";

import { recoverRevisionConflict } from "./revisionConflict";

describe("recoverRevisionConflict", () => {
  it("toasts when the refresh itself fails", async () => {
    const setToast = vi.fn();
    const handled = await recoverRevisionConflict(
      new TransportError({ code: "revision_conflict", message: "stale" }),
      {
        taskShow: async () => {
          throw new Error("offline");
        },
        applyDetail: () => {},
        setToast,
      },
    );
    expect(handled).toBe(true);
    expect(setToast).toHaveBeenLastCalledWith({ message: "offline", error: true });
  });

  it("ignores other errors", async () => {
    const setToast = vi.fn();
    const handled = await recoverRevisionConflict(new Error("nope"), {
      taskShow: async () => {
        throw new Error("unused");
      },
      applyDetail: () => {},
      setToast,
    });
    expect(handled).toBe(false);
    expect(setToast).not.toHaveBeenCalled();
  });
});