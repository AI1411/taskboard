# Collapsible Done column (issue #69) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Done column header toggles collapse. Collapsed shows header + count only (`Done` + `14`). Expanded shows cards again. State is session-only. Inbox Done exclusion is unchanged.

**Architecture:** `useState` in `Board` (`doneCollapsed`, default false). Done header is a button (`aria-expanded`). When collapsed, skip card/slot children; count still uses the filtered Done list length.

**Tech Stack:** React, Vitest. Issue: https://github.com/AI1411/taskboard/issues/69.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- No fifth column, no stored filter
- Inbox membership unchanged
- Tests: `pnpm --filter @taskboard/ui test`

## File map

- Modify: `packages/ui/src/Board.tsx`
- Modify: `packages/ui/src/Board.module.css`
- Test: `packages/ui/src/Board.test.tsx`

---

### Task 1: Toggle Done

**Files:**
- Modify: `packages/ui/src/Board.tsx`, `Board.module.css`
- Test: `packages/ui/src/Board.test.tsx`

**Interfaces:**
- Produces: Done header `button` named `Done` with `aria-expanded`

- [ ] **Step 1: Write the failing test**

```tsx
  it("collapses and expands the Done column for the session", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Finished", column: "done" });
    render(<TaskboardApp transport={transport} />);
    expect(await screen.findByRole("listitem", { name: /Finished/ })).toBeTruthy();
    const toggle = screen.getByRole("button", { name: "Done" });
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    await userEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByRole("listitem", { name: /Finished/ })).toBeNull();
    expect(screen.getByText("1")).toBeTruthy();
    await userEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("listitem", { name: /Finished/ })).toBeTruthy();
  });
```

- [ ] **Step 2: Run to verify fail**

Run: `pnpm --filter @taskboard/ui test -- src/Board.test.tsx`

Expected: FAIL — no Done button.

- [ ] **Step 3: Implement**

In `Board`: `const [doneCollapsed, setDoneCollapsed] = useState(false);`

Pass `collapsible` / `collapsed` / `onToggle` into `ColumnDrop` when `col.id === "done"`.

Header:

```tsx
      <h2 className={styles.header}>
        {props.onToggle ? (
          <button
            type="button"
            className={styles.collapse}
            aria-expanded={!props.collapsed}
            onClick={props.onToggle}
          >
            {props.label}
          </button>
        ) : (
          props.label
        )}
        <span className={styles.count}>{props.count}</span>
      </h2>
```

Skip children when `props.collapsed`.

- [ ] **Step 4: Run** `pnpm --filter @taskboard/ui test` — PASS

- [ ] **Step 5: Commit** `feat(ui): collapse the Done column for the session`

---

## Self-review

Toggle, count visible when collapsed, session-only, Inbox unchanged.
