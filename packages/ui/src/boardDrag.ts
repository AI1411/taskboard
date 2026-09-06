import type { Column } from "@taskboard/types";

import { COLUMN_IDS } from "./columns";

export type DragEndAction =
  | { kind: "move"; displayId: string; column: Column }
  | { kind: "reorder"; displayId: string; beforeId: string }
  | { kind: "none" };

export function resolveDragEnd(
  activeId: string,
  overId: string | null | undefined,
  tasks: readonly { displayId: string; column: Column }[],
): DragEndAction {
  if (!overId || activeId === overId) return { kind: "none" };
  if ((COLUMN_IDS as readonly string[]).includes(overId)) {
    return { kind: "move", displayId: activeId, column: overId as Column };
  }
  const dragged = tasks.find((t) => t.displayId === activeId);
  const over = tasks.find((t) => t.displayId === overId);
  if (!dragged || !over) return { kind: "none" };
  if (dragged.column !== over.column) {
    return { kind: "move", displayId: activeId, column: over.column };
  }
  return { kind: "reorder", displayId: activeId, beforeId: overId };
}
