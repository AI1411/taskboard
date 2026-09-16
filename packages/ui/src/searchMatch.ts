import type { TaskSummary } from "@taskboard/types";

import { BADGE_LABEL } from "./columns";

export function taskMatchesQuery(task: TaskSummary, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  const badge = BADGE_LABEL[task.displayStatus] ?? "";
  const haystack = [
    task.title,
    task.displayId,
    badge,
    task.runMessage ?? "",
    task.waitingReason ?? "",
  ]
    .join("\n")
    .toLowerCase();
  return haystack.includes(q);
}
