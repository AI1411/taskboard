# Empty board vs zero search results (issue #68) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Distinguish a project with no tasks from a search that hid every card. Empty board shows `No tasks — New task or press n` under the toolbar. Zero matches replace dashed slots with `No matching tasks` plus an Esc hint. Dashed slots remain only when a column has no cards.

**Architecture:** `Board` already has `tasks` (unfiltered) and `query`. Compute `visible` via `taskMatchesQuery`. Banner when `tasks.length === 0`. Search-empty when `query.trim()` and `visible.length === 0`. Skip `.slot` in the search-empty case.

**Tech Stack:** React, CSS Modules, Vitest. Issue: https://github.com/AI1411/taskboard/issues/68.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- UI copy is English
- No custom filter UI
- Tests: `pnpm --filter @taskboard/ui test`

## File map

- Modify: `packages/ui/src/Board.tsx`
- Modify: `packages/ui/src/Board.module.css`
- Test: `packages/ui/src/Board.test.tsx`

---

### Task 1: Empty and zero-match copy

**Files:**
- Modify: `packages/ui/src/Board.tsx`, `Board.module.css`
- Test: `packages/ui/src/Board.test.tsx`

**Interfaces:**
- Consumes: `Board.tasks`, `Board.query`, `taskMatchesQuery`
- Produces: banner text `No tasks — New task or press n`; empty-search text `No matching tasks` + `Esc`

- [ ] **Step 1: Write the failing tests**

Append to `Board.test.tsx`:

```tsx
  it("shows next-step copy when the project has no tasks", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Alpha" });
    render(<TaskboardApp transport={transport} />);
    expect(await screen.findByText("No tasks — New task or press n")).toBeTruthy();
    expect(screen.queryByText("No matching tasks")).toBeNull();
  });

  it("shows no-matching copy instead of empty slots when search hides every card", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Alpha task", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    expect(await screen.findByText("Alpha task")).toBeTruthy();
    expect(screen.queryByText("No tasks — New task or press n")).toBeNull();
    await userEvent.keyboard("/");
    await userEvent.keyboard("zzzz");
    expect(screen.queryByText("Alpha task")).toBeNull();
    expect(screen.getByText("No matching tasks")).toBeTruthy();
    expect(screen.getByText("Esc")).toBeTruthy();
    expect(document.querySelector("[class*='slot']")).toBeNull();
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test -- src/Board.test.tsx`

Expected: FAIL — neither string exists.

- [ ] **Step 3: Write minimal implementation**

In `Board.tsx` after `visible`:

```tsx
  const emptyBoard = props.tasks.length === 0;
  const noMatches = Boolean(props.query.trim()) && visible.length === 0;
```

Under the toolbar:

```tsx
      {emptyBoard ? (
        <p className={styles.banner}>No tasks — New task or press n</p>
      ) : null}
      {noMatches ? (
        <p className={styles.banner}>
          No matching tasks <kbd className={styles.searchHint}>Esc</kbd>
        </p>
      ) : null}
```

Change slot:

```tsx
                  {columnTasks.length === 0 && !noMatches ? <div className={styles.slot} /> : null}
```

`Board.module.css`:

```css
.banner {
  margin: 0;
  font-size: 0.85rem;
  color: var(--sidebar-muted);
}
```

- [ ] **Step 4: Run tests**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat(ui): distinguish empty board from zero search results"
```

---

## Self-review

Empty board banner; search-zero copy + Esc; slots only for empty columns when not search-zero.
