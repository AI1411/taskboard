# Widen search (issue #67) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `/` search matches title, `TASK-n`, status badges (Waiting / Running / Failed / Done), `runMessage`, and `waitingReason` (case-insensitive). Placeholder becomes `Search title, TASK-n, status`. `j`/`k` use the same matcher.

**Architecture:** Extract `taskMatchesQuery(task, query)` in `packages/ui/src/searchMatch.ts`. `Board` filters visible cards with it. `TaskboardApp.selectInColumn` uses the same function. No custom filter UI.

**Tech Stack:** React, Vitest. Issue: https://github.com/AI1411/taskboard/issues/67.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- UI copy is English
- No custom filter UI
- Inbox membership rules unchanged
- Tests: `pnpm --filter @taskboard/ui test`

## File map

- Create: `packages/ui/src/searchMatch.ts`
- Test: `packages/ui/src/searchMatch.test.ts`
- Modify: `packages/ui/src/Board.tsx` — use `taskMatchesQuery`
- Modify: `packages/ui/src/Search.tsx` — placeholder
- Modify: `packages/ui/src/TaskboardApp.tsx` — `j`/`k` filter
- Test: `packages/ui/src/Board.test.tsx`

---

### Task 1: `taskMatchesQuery` + placeholder + Board / j/k

**Files:**
- Create: `packages/ui/src/searchMatch.ts`
- Test: `packages/ui/src/searchMatch.test.ts`
- Modify: `packages/ui/src/Board.tsx`, `Search.tsx`, `TaskboardApp.tsx`, `Board.test.tsx`

**Interfaces:**
- Produces: `export function taskMatchesQuery(task: TaskSummary, query: string): boolean`

- [ ] **Step 1: Write the failing tests**

`packages/ui/src/searchMatch.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { summary } from "./summary";
import { taskMatchesQuery } from "./searchMatch";

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
```

Append to `Board.test.tsx`:

```tsx
  it("filters by TASK-n, status badge, and run text", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const a = await transport.taskCreate(project.slug, { title: "Alpha task", column: "todo" });
    const b = await transport.taskCreate(project.slug, { title: "Beta item", column: "todo" });
    a.displayStatus = "waiting";
    a.waitingReason = "Need spec";
    b.runMessage = "compiling kernel";
    render(<TaskboardApp transport={transport} />);
    expect(await screen.findByText("Alpha task")).toBeTruthy();
    expect(screen.getByPlaceholderText("Search title, TASK-n, status")).toBeTruthy();
    await userEvent.keyboard("/");
    await userEvent.keyboard("task-2");
    expect(screen.queryByText("Alpha task")).toBeNull();
    expect(screen.getByText("Beta item")).toBeTruthy();
    await userEvent.keyboard("{Escape}");
    await userEvent.keyboard("/");
    await userEvent.keyboard("waiting");
    expect(screen.getByText("Alpha task")).toBeTruthy();
    expect(screen.queryByText("Beta item")).toBeNull();
    await userEvent.keyboard("{Escape}");
    await userEvent.keyboard("/");
    await userEvent.keyboard("kernel");
    expect(screen.getByText("Beta item")).toBeTruthy();
    expect(screen.queryByText("Alpha task")).toBeNull();
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test -- src/searchMatch.test.ts src/Board.test.tsx`

Expected: FAIL — `taskMatchesQuery` missing; placeholder still `Search`; title-only filter.

- [ ] **Step 3: Write minimal implementation**

`packages/ui/src/searchMatch.ts`:

```ts
import type { TaskSummary } from "@taskboard/types";
import { BADGE_LABEL } from "./columns";

export function taskMatchesQuery(task: TaskSummary, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  const badge = BADGE_LABEL[task.displayStatus] ?? "";
  const haystack = [task.title, task.displayId, badge, task.runMessage ?? "", task.waitingReason ?? ""]
    .join("\n")
    .toLowerCase();
  return haystack.includes(q);
}
```

`Board.tsx`: `props.tasks.filter((t) => taskMatchesQuery(t, q))` (always, empty query matches all).

`Search.tsx`: `placeholder="Search title, TASK-n, status"`

`TaskboardApp.tsx` `selectInColumn`:

```ts
      const inCol = tasksRef.current.filter(
        (t) => t.column === col && taskMatchesQuery(t, q),
      );
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/ui/src/searchMatch.ts packages/ui/src/searchMatch.test.ts \
  packages/ui/src/Board.tsx packages/ui/src/Search.tsx \
  packages/ui/src/TaskboardApp.tsx packages/ui/src/Board.test.tsx \
  docs/superpowers/plans/2026-09-16-issue-67-widen-search.md
git commit -m "feat(ui): widen search to id, status, and run text"
```

---

## Self-review

Acceptance: title / TASK-n / status / runMessage / waitingReason / case-insensitive / same for j/k / placeholder. No custom filters.
