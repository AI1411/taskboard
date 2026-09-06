import type { Column, DisplayStatus } from "@taskboard/types";

export const COLUMNS: { id: Column; label: string }[] = [
  { id: "todo", label: "Todo" },
  { id: "in-progress", label: "In Progress" },
  { id: "in-review", label: "In Review" },
  { id: "done", label: "Done" },
];

export const COLUMN_IDS: Column[] = COLUMNS.map((c) => c.id);

export const BADGE_LABEL: Record<DisplayStatus, string | null> = {
  idle: null,
  waiting: "Waiting",
  running: "Running",
  failed: "Failed",
  completed: "Done",
};

export function neighborColumn(column: Column, dir: -1 | 1): Column | null {
  const i = COLUMN_IDS.indexOf(column);
  const next = i + dir;
  if (next < 0 || next >= COLUMN_IDS.length) return null;
  return COLUMN_IDS[next] ?? null;
}
