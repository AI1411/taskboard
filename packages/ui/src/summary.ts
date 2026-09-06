import type { TaskSummary } from "@taskboard/types";

export function summary(overrides: Partial<TaskSummary> = {}): TaskSummary {
  return {
    id: "t1",
    displayId: "TASK-1",
    projectId: "p1",
    title: "Example",
    column: "todo",
    urgent: false,
    revision: 1,
    displayStatus: "idle",
    runMessage: null,
    ...overrides,
  };
}
