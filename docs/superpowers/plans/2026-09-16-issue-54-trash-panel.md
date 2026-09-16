# Trash Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Open a Trash panel from the sidebar that lists soft-deleted projects and tasks with Restore, and confirm deletes (`Delete {title}?`) from the inspector and Delete/Backspace.

**Architecture:** Domain trash already exists (`trashList`, `taskRestore`, `projectRestore`). The UI adds `TrashPanel` plus a shared `ConfirmDialog`. `TaskboardApp` loads trash when the panel opens, wires restore, and lifts delete-confirm so keyboard and inspector share one dialog. `fakeTransport` keeps deleted entities so tests can restore them.

**Tech Stack:** React, Vitest, Testing Library. Spec §7: `docs/superpowers/specs/2026-09-06-usability-mechanisms-design.md`. Issue: https://github.com/AI1411/taskboard/issues/54. Existing plan Task 7 (Trash / confirm / Delete key only; no links, no project ⋯ menu, no toasts).

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- 30-day retention and purge stay unchanged
- UI copy is English
- Empty trash copy is exactly `Trash is empty`
- Confirm copy is `Delete {title}?` with Delete / Cancel
- Toasts / Undo toast are out of scope (issue #59)
- Link add/remove is out of scope (issue #56)
- Project ⋯ menu is out of scope (issue #57)
- Shortcut legend (`?`) is out of scope (issue #58)

## File map

- Create: `packages/ui/src/TrashPanel.tsx`
- Create: `packages/ui/src/TrashPanel.module.css`
- Create: `packages/ui/src/TrashPanel.test.tsx`
- Create: `packages/ui/src/ConfirmDialog.tsx`
- Create: `packages/ui/src/ConfirmDialog.module.css`
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Modify: `packages/ui/src/fakeTransport.ts`
- Modify: `packages/ui/src/index.ts`
- Modify: `packages/ui/src/inspector-design.test.tsx`
- Modify: `packages/ui/src/keyboard.test.tsx`

---

### Task 1: Trash panel, restore, and delete confirm

**Files:**
- Create: `packages/ui/src/TrashPanel.tsx`, `packages/ui/src/TrashPanel.module.css`, `packages/ui/src/TrashPanel.test.tsx`
- Create: `packages/ui/src/ConfirmDialog.tsx`, `packages/ui/src/ConfirmDialog.module.css`
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Modify: `packages/ui/src/fakeTransport.ts`
- Modify: `packages/ui/src/index.ts`
- Modify: `packages/ui/src/inspector-design.test.tsx`
- Modify: `packages/ui/src/keyboard.test.tsx`

**Interfaces:**
- Consumes: `transport.trashList`, `transport.taskRestore`, `transport.projectRestore`, `transport.taskDelete`
- Produces:
  - `TrashPanel` props `{ trash: Trash; onClose: () => void; onRestoreTask: (displayId: string) => void; onRestoreProject: (slug: string) => void; onSelectTask: (displayId: string) => void }`
  - Restore buttons `aria-label={`Restore ${displayId}`}` / `aria-label={`Restore ${slug}`}`
  - Empty copy `Trash is empty`
  - `ConfirmDialog` is `role="alertdialog"` with `message`, `onConfirm`, `onCancel`; default confirm label `Delete`
  - Inspector confirm is `Delete {title}?` then Delete / Cancel (no `Move to Trash`)
  - Inspector shows Restore (not Delete) when `trashed` is true
  - Board-focused `Delete` / `Backspace` opens the same confirm; `Escape` cancels
  - `fakeTransport.taskDelete` moves the task into `trashList().tasks`; `projectDelete` sets `deletedAt` and `projectList` hides it; restore puts the entity back

- [x] **Step 1: Write the failing tests**

`packages/ui/src/TrashPanel.test.tsx`:

```tsx
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

describe("TrashPanel", () => {
  it("shows empty copy when trash has nothing", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Alpha" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByRole("list", { name: "Todo" });
    await userEvent.click(screen.getByRole("button", { name: "Trash" }));
    expect(await screen.findByText("Trash is empty")).toBeTruthy();
  });

  it("restores a deleted task from the panel", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Gone", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Gone"));
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(transport.taskDelete).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(transport.taskDelete).toHaveBeenCalled());
    await userEvent.click(screen.getByRole("button", { name: "Trash" }));
    expect(await screen.findByText("Gone")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Restore TASK-1" }));
    await waitFor(() => expect(transport.taskRestore).toHaveBeenCalledWith("TASK-1"));
    expect(await screen.findByText("Gone")).toBeTruthy();
  });

  it("restores a deleted project from the panel", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Alpha" });
    await transport.projectDelete("alpha");
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(screen.getByRole("button", { name: "Trash" }));
    expect(await screen.findByText("Alpha")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Restore alpha" }));
    await waitFor(() => expect(transport.projectRestore).toHaveBeenCalledWith("alpha"));
  });

  it("selecting a trashed task shows Restore in the inspector", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Gone", column: "todo" });
    await transport.taskDelete("TASK-1");
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(screen.getByRole("button", { name: "Trash" }));
    await userEvent.click(await screen.findByText("Gone"));
    expect(await screen.findByRole("button", { name: "Restore" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Delete" })).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "Restore" }));
    await waitFor(() => expect(transport.taskRestore).toHaveBeenCalledWith("TASK-1"));
  });
});
```

Add to `packages/ui/src/keyboard.test.tsx`:

```tsx
it("delete key confirms and deletes the selected card", async () => {
  const transport = fakeTransport();
  const project = await transport.projectAdd({ name: "Alpha" });
  await transport.taskCreate(project.slug, { title: "Drop me", column: "todo" });
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(await screen.findByText("Drop me"));
  await userEvent.keyboard("{Delete}");
  expect(transport.taskDelete).not.toHaveBeenCalled();
  expect(screen.getByText("Delete Drop me?")).toBeTruthy();
  await userEvent.keyboard("{Escape}");
  expect(transport.taskDelete).not.toHaveBeenCalled();
  await userEvent.keyboard("{Backspace}");
  await userEvent.click(screen.getByRole("button", { name: "Delete" }));
  await waitFor(() => expect(transport.taskDelete).toHaveBeenCalled());
});
```

Replace `inspector-design.test.tsx` `"Delete asks for confirmation..."` confirm click:

```tsx
await userEvent.click(await screen.findByRole("button", { name: "Delete" }));
expect(transport.taskDelete).not.toHaveBeenCalled();
expect(screen.getByText("Delete Keep me?")).toBeTruthy();
await userEvent.click(screen.getByRole("button", { name: "Delete" }));
await waitFor(() => expect(transport.taskDelete).toHaveBeenCalled());
```

- [x] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test`

Expected: FAIL missing Trash panel / `Restore TASK-1` / `Delete Drop me?` / confirm still labeled `Move to Trash`.

- [x] **Step 3: Implement panel, confirm, fake trash, and wiring**

`ConfirmDialog.tsx`:

```tsx
import styles from "./ConfirmDialog.module.css";

export function ConfirmDialog(props: {
  message: string;
  confirmLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className={styles.dialog} role="alertdialog" aria-label={props.message}>
      <p className={styles.message}>{props.message}</p>
      <div className={styles.row}>
        <button type="button" className={styles.confirm} onClick={props.onConfirm}>
          {props.confirmLabel ?? "Delete"}
        </button>
        <button type="button" className={styles.cancel} onClick={props.onCancel}>
          Cancel
        </button>
      </div>
    </div>
  );
}
```

`TrashPanel.tsx` renders `role="dialog"` `aria-label="Trash"`. When both lists are empty, show `Trash is empty`. Task Restore button: `aria-label={`Restore ${task.displayId}`}` and visible text `Restore`. Project Restore: `aria-label={`Restore ${project.slug}`}`. Close button `aria-label="Close trash"`.

`fakeTransport`:

```ts
const deletedTasks: TaskSummary[] = [];

async projectList(includeArchived) {
  return projects.filter((p) => !p.deletedAt && (includeArchived || !p.archived));
},
async taskDelete(displayId, revision) {
  const idx = tasks.findIndex((t) => t.displayId === displayId);
  if (idx < 0) throw new Error("not found");
  const task = tasks[idx];
  task.revision = (revision ?? task.revision) + 1;
  tasks.splice(idx, 1);
  deletedTasks.push({ ...task });
  return details.get(displayId)!;
},
async taskRestore(displayId) {
  const detail = details.get(displayId);
  if (!detail) throw new Error("not found");
  const i = deletedTasks.findIndex((t) => t.displayId === displayId);
  if (i >= 0) {
    const [task] = deletedTasks.splice(i, 1);
    if (!tasks.some((t) => t.displayId === displayId)) tasks.push(task);
  } else if (!tasks.some((t) => t.displayId === displayId)) {
    tasks.push(detail);
  }
  return { ...detail };
},
async trashList() {
  return {
    projects: projects.filter((p) => p.deletedAt).map((p) => ({ ...p })),
    tasks: deletedTasks.map((t) => ({ ...t })),
  };
},
```

`TaskboardApp`:

```tsx
const [trashOpen, setTrashOpen] = useState(false);
const [trash, setTrash] = useState<Trash>({ projects: [], tasks: [] });
const [confirmDelete, setConfirmDelete] = useState(false);
const [trashedSelection, setTrashedSelection] = useState(false);

async function refreshTrash() {
  setTrash(await transport.trashList());
}

// Delete / Backspace (not typing): setConfirmDelete(true), open inspector
// Escape: if confirmDelete, cancel; else existing behavior
// onTrash: refreshTrash(); setTrashOpen(true)
// restore task: taskRestore; setTrashedSelection(false); reloadBoard(); refreshTrash()
// restore project: projectRestore; reloadBoard(); refreshTrash()
// select trash task: taskShow; setTrashedSelection(true); open inspector
```

Inspector: `trashed?: boolean`, `confirming?: boolean`, `onDeleteRequest`, `onRestore`. When `trashed`, show Restore. When `confirming`, show `<ConfirmDialog message={`Delete ${task.title}?`} />` instead of the Delete button. Initial Delete calls `onDeleteRequest`.

- [x] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS. No remaining UI assertion on `Move to Trash`.

- [x] **Step 5: Commit**

```bash
git add docs/superpowers/plans/2026-09-16-issue-54-trash-panel.md packages/ui
git commit -m "feat: open trash panel with restore and confirm delete"
```

---

## Self-review

1. **Spec coverage:** Trash panel, empty copy, restore tasks/projects, inspector Restore for trashed selection, `Delete {title}?` confirm, Delete/Backspace, Esc cancel (§7). Links, project menu, toasts left to later issues.
2. **Placeholder scan:** No TBD. Test code is complete.
3. **Type consistency:** `TrashPanel` / `ConfirmDialog` props match TaskboardApp wiring; Restore accessible names are `Restore TASK-n` / `Restore {slug}`.
