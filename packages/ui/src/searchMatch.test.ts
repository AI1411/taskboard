import { describe, expect, it } from "vitest";

import { taskMatchesQuery } from "./searchMatch";
import { summary } from "./summary";

describe("taskMatchesQuery", () => {
  it("matches empty query against every card", () => {
    expect(taskMatchesQuery(summary({ title: "Alpha" }), "")).toBe(true);
    expect(taskMatchesQuery(summary({ title: "Alpha" }), "   ")).toBe(true);
  });

  it("matches title, displayId, badge, runMessage, and waitingReason case-insensitively", () => {
    const waiting = summary({
      title: "Ship login",
      displayId: "TASK-12",
      displayStatus: "waiting",
      runMessage: "blocked on review",
      waitingReason: "Need spec",
    });
    expect(taskMatchesQuery(waiting, "login")).toBe(true);
    expect(taskMatchesQuery(waiting, "task-12")).toBe(true);
    expect(taskMatchesQuery(waiting, "WAITING")).toBe(true);
    expect(taskMatchesQuery(waiting, "blocked")).toBe(true);
    expect(taskMatchesQuery(waiting, "spec")).toBe(true);
    expect(taskMatchesQuery(waiting, "missing")).toBe(false);
  });
});
