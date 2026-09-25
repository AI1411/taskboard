import { describe, expect, it } from "vitest";
import type { InboxItem } from "@taskboard/types";

import { inboxItemsForScope } from "./inboxScope";

function item(projectSlug: string): InboxItem {
  return {
    id: projectSlug,
    displayId: "TASK-1",
    projectId: "p",
    projectSlug,
    projectName: projectSlug,
    title: "t",
    column: "todo",
    urgent: true,
    revision: 1,
    displayStatus: "idle",
    runMessage: null,
    waitingReason: null,
    reason: "",
    stale: false,
    updatedAt: "2026-09-25T00:00:00.000Z",
  };
}

describe("inboxItemsForScope", () => {
  it("keeps the current project when scope is this", () => {
    const all = [item("alpha"), item("beta")];
    expect(inboxItemsForScope(all, "this", "alpha").map((row) => row.projectSlug)).toEqual([
      "alpha",
    ]);
  });

  it("returns every project when scope is all", () => {
    const all = [item("alpha"), item("beta")];
    expect(inboxItemsForScope(all, "all", "alpha")).toHaveLength(2);
  });
});
