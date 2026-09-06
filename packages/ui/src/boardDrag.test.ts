import { describe, expect, it } from "vitest";

import { resolveDragEnd } from "./boardDrag";

describe("resolveDragEnd", () => {
  const tasks = [
    { displayId: "TASK-1", column: "todo" as const },
    { displayId: "TASK-2", column: "done" as const },
    { displayId: "TASK-3", column: "todo" as const },
  ];

  it("moves when dropping onto a card in another column", () => {
    expect(resolveDragEnd("TASK-1", "TASK-2", tasks)).toEqual({
      kind: "move",
      displayId: "TASK-1",
      column: "done",
    });
  });

  it("reorders when dropping onto a card in the same column", () => {
    expect(resolveDragEnd("TASK-1", "TASK-3", tasks)).toEqual({
      kind: "reorder",
      displayId: "TASK-1",
      beforeId: "TASK-3",
    });
  });

  it("moves when dropping onto a column droppable", () => {
    expect(resolveDragEnd("TASK-1", "done", tasks)).toEqual({
      kind: "move",
      displayId: "TASK-1",
      column: "done",
    });
  });
});
