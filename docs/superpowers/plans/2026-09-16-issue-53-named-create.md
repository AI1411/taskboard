# Named Create Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop the UI from inserting `Untitled` cards or projects. `n` / New task and `p` / New project focus an inline composer; nothing is written until a non-empty name is committed.

**Architecture:** A shared `Composer` input lives in the current column (tasks) and the sidebar (projects). `TaskboardApp` no longer calls `taskCreate` / `projectAdd` from the key or button handlers. Submit with a trimmed non-empty title creates and selects; empty Enter or Escape cancels.

**Tech Stack:** React, Vitest, Testing Library. Spec §5: `docs/superpowers/specs/2026-09-06-usability-mechanisms-design.md`. Issue: https://github.com/AI1411/taskboard/issues/53. Existing plan Task 5 (composer only; no `?` legend).

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- UI and `n`/`p` never create an `Untitled` entity
- Empty title is `validation_error` on `title` if a client sends it; the composer must not send empty titles
- CLI `--title` stays required; do not invent a default title on the server
- UI copy is English
- Shortcut legend (`?`) is out of scope (issue #58)
- Toasts / last project are out of scope (issue #59)

## File map

- Create: `packages/ui/src/Composer.tsx`
- Create: `packages/ui/src/Composer.module.css`
- Create: `packages/ui/src/Composer.test.tsx`
- Modify: `packages/ui/src/Board.tsx`
- Modify: `packages/ui/src/Sidebar.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Modify: `packages/ui/src/index.ts`
- Modify: `packages/ui/src/Board.test.tsx`
- Modify: `packages/ui/src/keyboard.test.tsx`

---

### Task 1: Composer and named create

**Files:**
- Create: `packages/ui/src/Composer.tsx`, `packages/ui/src/Composer.module.css`, `packages/ui/src/Composer.test.tsx`
- Modify: `packages/ui/src/Board.tsx` (composer at top of `currentColumn`)
- Modify: `packages/ui/src/Sidebar.tsx` (project composer)
- Modify: `packages/ui/src/TaskboardApp.tsx` (`createTask` / `addProject` take a name; `n`/`p` only open composers)
- Modify: `packages/ui/src/index.ts`
- Modify: `packages/ui/src/Board.test.tsx`
- Modify: `packages/ui/src/keyboard.test.tsx`

**Interfaces:**
- Consumes: `transport.taskCreate`, `transport.projectAdd`
- Produces:
  - `Composer` props `{ placeholder: string; onSubmit: (title: string) => void; onCancel: () => void; inputRef?: Ref<HTMLInputElement> }`
  - Enter with trimmed non-empty value calls `onSubmit`; empty Enter or Escape calls `onCancel` and writes nothing
  - `n` / New task focuses the task composer at the top of `currentColumn` and does not call `taskCreate`
  - `p` / New project focuses the project composer and does not call `projectAdd` until submit
  - After task submit, create in `currentColumn` and select the new card
  - When no project exists, New task stays hidden (Board already only renders when a project is selected)

- [x] **Step 1: Write the failing tests**

`packages/ui/src/Composer.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { Composer } from "./Composer";

describe("Composer", () => {
  it("submits trimmed title on Enter and cancels empty Enter", async () => {
    const onSubmit = vi.fn();
    const onCancel = vi.fn();
    render(<Composer placeholder="Task title" onSubmit={onSubmit} onCancel={onCancel} />);
    const input = screen.getByPlaceholderText("Task title");
    await userEvent.type(input, "  Fix login  {Enter}");
    expect(onSubmit).toHaveBeenCalledWith("Fix login");
    onSubmit.mockClear();
    await userEvent.clear(input);
    await userEvent.type(input, "{Enter}");
    expect(onSubmit).not.toHaveBeenCalled();
    expect(onCancel).toHaveBeenCalled();
  });
});
```

Replace `Board.test.tsx` `"New project calls projectAdd with Untitled"` with:

```tsx
it("New project composer commits a named project", async () => {
  const transport = fakeTransport();
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(screen.getByRole("button", { name: "New project" }));
  expect(transport.projectAdd).not.toHaveBeenCalled();
  await userEvent.type(screen.getByPlaceholderText("Project name"), "Renai Sim{Enter}");
  await waitFor(() => expect(transport.projectAdd).toHaveBeenCalledWith({ name: "Renai Sim" }));
});
```

Replace `keyboard.test.tsx` `"n creates a task through transport"`, `"p creates a project..."`, and `"1-4 jump columns so n creates in the current column"`:

```tsx
it("n opens composer and Enter creates in the current column", async () => {
  const transport = fakeTransport();
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(screen.getByRole("button", { name: "New project" }));
  await userEvent.type(screen.getByPlaceholderText("Project name"), "Alpha{Enter}");
  await screen.findByRole("list", { name: "Todo" });
  await userEvent.keyboard("2");
  await userEvent.keyboard("n");
  expect(transport.taskCreate).not.toHaveBeenCalled();
  await userEvent.type(screen.getByPlaceholderText("Task title"), "Ship it{Enter}");
  await waitFor(() =>
    expect(transport.taskCreate).toHaveBeenCalledWith("alpha", {
      title: "Ship it",
      column: "in-progress",
    }),
  );
});

it("p opens project composer and does not insert Untitled", async () => {
  const transport = fakeTransport();
  render(<TaskboardApp transport={transport} />);
  await userEvent.keyboard("p");
  expect(transport.projectAdd).not.toHaveBeenCalled();
  await userEvent.type(screen.getByPlaceholderText("Project name"), "Beta{Enter}");
  await waitFor(() => expect(transport.projectAdd).toHaveBeenCalledWith({ name: "Beta" }));
});
```

Keep other keyboard tests. They may still seed via `transport.projectAdd({ name: "Untitled" })` — that is test data, not a UI create path.

- [x] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test`

Expected: FAIL missing Composer / New project still calls `projectAdd` with Untitled / `n` still creates Untitled.

- [x] **Step 3: Implement Composer and rewire create**

`Composer` is a single-line `<input autoFocus>`. On keydown:

```tsx
if (e.key === "Enter") {
  const title = value.trim();
  if (title) props.onSubmit(title);
  else props.onCancel();
}
if (e.key === "Escape") props.onCancel();
```

`Board` accepts `composing: boolean`, `composerRef`, `onComposerSubmit`, `onComposerCancel`. When `composing`, render `<Composer placeholder="Task title" />` as the first child of the current column list.

`Sidebar` accepts `composing: boolean`, `composerRef`, `onComposerSubmit`, `onComposerCancel`. When `composing`, render `<Composer placeholder="Project name" />` under the New project button.

`TaskboardApp`:

```tsx
const [composingTask, setComposingTask] = useState(false);
const [composingProject, setComposingProject] = useState(false);
const taskComposerRef = useRef<HTMLInputElement>(null);
const projectComposerRef = useRef<HTMLInputElement>(null);

const createTask = useCallback(async (title: string) => {
  const project = selectedProjectRef.current;
  if (!project) return;
  const created = await transport.taskCreate(project.slug, {
    title,
    column: currentColumnRef.current,
  });
  setComposingTask(false);
  await refreshTasks(project.slug);
  selectedIdRef.current = created.displayId;
  setSelectedId(created.displayId);
  applyDetail(created);
  setInspectorOpen(true);
}, [transport, refreshTasks]);

const addProject = useCallback(async (name: string) => {
  const project = await transport.projectAdd({ name });
  setComposingProject(false);
  // keep the rest of today's apply-new-project bookkeeping
}, [transport, refreshTasks]);
```

`n` sets `composingTask` true (does not call `taskCreate`). `p` / New project sets `composingProject` true. New task button same as `n`. `isTypingTarget` already returns early for INPUT so `?`/`n` while composing do not collide; Escape in the composer is handled by Composer.

- [x] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS. Grep UI tests: no assertion that the UI create path sends `{ name: "Untitled" }` or `{ title: "Untitled" }`.

- [x] **Step 5: Commit**

```bash
git add packages/ui docs/superpowers/plans/2026-09-16-issue-53-named-create.md
git commit -m "feat: name tasks and projects in composers instead of Untitled"
```

---

## Self-review

1. **Spec coverage:** New Task composer, New project composer, `n`/`p` no longer insert Untitled (§5). Legend/toasts left to other issues.
2. **Placeholder scan:** No TBD. Test code is complete.
3. **Type consistency:** `Composer` props and `createTask(title)` / `addProject(name)` match across steps.
