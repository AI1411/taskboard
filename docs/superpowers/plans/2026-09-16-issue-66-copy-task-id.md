# One-click copy TASK-n (issue #66) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make card and inspector `TASK-n` labels one-click copy buttons that write the id to the clipboard and toast `Copied`, without starting a drag or changing the current column.

**Architecture:** The display-id `<span>` on `Card` and the inspector id `<p>` become `<button type="button">`. Clicks `stopPropagation` / `onPointerDown` stop so dnd-kit’s 8px PointerSensor and card `onClick` do not fire. `TaskboardApp.copyId` calls `navigator.clipboard.writeText` and `setToast({ message: "Copied" })` (#59 toast).

**Tech Stack:** React, CSS Modules, Vitest + Testing Library. Issue: https://github.com/AI1411/taskboard/issues/66.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- UI copy is English
- Do not rename ids, add a command palette, or treat titles as IDs
- Drag still starts after 8px (`PointerSensor` unchanged)
- Use the existing `Toast` region; do not add a second toast stack
- Tests: `pnpm --filter @taskboard/ui test`

## File map

- Modify: `packages/ui/src/Card.tsx` — `onCopyId?: (displayId: string) => void`; id button
- Modify: `packages/ui/src/Card.module.css` — unstyled-as-text copy button
- Modify: `packages/ui/src/Board.tsx` — thread `onCopyId` to `SortableCard` / overlay `Card`
- Modify: `packages/ui/src/Inspector.tsx` — id button + `onCopyId`
- Modify: `packages/ui/src/Inspector.module.css` — copy button looks like the old muted id
- Modify: `packages/ui/src/TaskboardApp.tsx` — `copyId` + toast `Copied`
- Test: `packages/ui/src/Card.test.tsx`
- Test: `packages/ui/src/inspector-design.test.tsx`

---

### Task 1: Card copy button

**Files:**
- Modify: `packages/ui/src/Card.tsx`
- Modify: `packages/ui/src/Card.module.css`
- Test: `packages/ui/src/Card.test.tsx`

**Interfaces:**
- Consumes: existing `CardProps`
- Produces: `onCopyId?: (displayId: string) => void`

- [ ] **Step 1: Write the failing test**

Append to `packages/ui/src/Card.test.tsx`:

```tsx
  it("copy button calls onCopyId and does not require a card click", async () => {
    const onCopyId = vi.fn();
    const onClick = vi.fn();
    const { getByRole } = render(
      <Card
        task={summary({ title: "Fix login", displayId: "TASK-7" })}
        selected={false}
        onCopyId={onCopyId}
        onClick={onClick}
      />,
    );
    await userEvent.click(getByRole("button", { name: "Copy TASK-7" }));
    expect(onCopyId).toHaveBeenCalledWith("TASK-7");
    expect(onClick).not.toHaveBeenCalled();
  });
```

Add `import userEvent from "@testing-library/user-event"` and `vi` from `vitest`.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @taskboard/ui test -- src/Card.test.tsx`

Expected: FAIL — no button named `Copy TASK-7`.

- [ ] **Step 3: Write minimal implementation**

Add `onCopyId?: (displayId: string) => void` to `CardProps` (not spread onto the root via `...rest` — destructure it). Replace the id span:

```tsx
      <button
        type="button"
        className={styles.displayId}
        aria-label={`Copy ${task.displayId}`}
        onPointerDown={(event) => event.stopPropagation()}
        onClick={(event) => {
          event.stopPropagation();
          onCopyId?.(task.displayId);
        }}
      >
        {task.displayId}
      </button>
```

In `Card.module.css`, make `.displayId` a button that looks like the old text:

```css
.displayId {
  grid-column: 1;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--sidebar-muted);
  font: inherit;
  font-size: 0.72rem;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0;
  text-align: left;
  cursor: pointer;
}

.displayId:hover {
  text-decoration: underline;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @taskboard/ui test -- src/Card.test.tsx`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/ui/src/Card.tsx packages/ui/src/Card.module.css packages/ui/src/Card.test.tsx
git commit -m "feat(ui): copy TASK-n from the card face"
```

---

### Task 2: Inspector copy + Copied toast + Board wiring

**Files:**
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/Inspector.module.css`
- Modify: `packages/ui/src/Board.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Test: `packages/ui/src/inspector-design.test.tsx`

**Interfaces:**
- Consumes: `Card.onCopyId`, Inspector `onCopyId?: (displayId: string) => void`
- Produces: `copyId(displayId: string): Promise<void>` writes clipboard and `setToast({ message: "Copied" })`

- [ ] **Step 1: Write the failing tests**

Append to `packages/ui/src/inspector-design.test.tsx`:

```tsx
  it("copies TASK-n from the card and inspector and toasts Copied", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Copy me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("button", { name: "Copy TASK-1" }));
    expect(writeText).toHaveBeenCalledWith("TASK-1");
    expect(await screen.findByText("Copied")).toBeTruthy();
    expect(screen.getByRole("listitem", { name: /Copy me/ }).getAttribute("aria-selected")).toBe(
      "false",
    );
    await userEvent.click(screen.getByRole("listitem", { name: /Copy me/ }));
    await userEvent.click(await screen.findByRole("button", { name: "Copy TASK-1" }));
    expect(writeText).toHaveBeenCalledTimes(2);
    expect(await screen.findByText("Copied")).toBeTruthy();
  });
```

After the inspector is open there are two `Copy TASK-1` buttons (card + inspector). First click is only the card (inspector closed). Second click after open: use

```tsx
    const copies = screen.getAllByRole("button", { name: "Copy TASK-1" });
    await userEvent.click(copies[copies.length - 1]);
```

for the inspector button.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @taskboard/ui test -- src/inspector-design.test.tsx`

Expected: FAIL — no copy wiring / no `Copied` toast.

- [ ] **Step 3: Write minimal implementation**

`Inspector` prop `onCopyId?: (displayId: string) => void`. Replace `<p className={styles.displayId}>` with:

```tsx
        <button
          type="button"
          className={styles.displayId}
          aria-label={`Copy ${props.task.displayId}`}
          onClick={() => props.onCopyId?.(props.task.displayId)}
        >
          {props.task.displayId}
        </button>
```

In `Inspector.module.css`, restyle `.displayId` as a text button (same muted size).

`Board` adds `onCopyId?: (displayId: string) => void` and passes it to `SortableCard` and overlay `Card`.

`TaskboardApp`:

```tsx
  const copyId = useCallback(async (displayId: string) => {
    await navigator.clipboard.writeText(displayId);
    setToast({ message: "Copied" });
  }, []);
```

Pass `onCopyId={copyId}` to `Board` and `Inspector`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test`

Expected: all UI tests PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/ui/src/Inspector.tsx packages/ui/src/Inspector.module.css \
  packages/ui/src/Board.tsx packages/ui/src/TaskboardApp.tsx \
  packages/ui/src/inspector-design.test.tsx
git commit -m "feat(ui): copy TASK-n from the inspector and toast Copied"
```

---

## Self-review

| Acceptance | Task |
| --- | --- |
| Clicking `TASK-n` on a card copies that id | Task 1–2 |
| Clicking the inspector id copies the same value | Task 2 |
| Short `Copied` confirmation | Task 2 toast |
| Click does not start a drag or change column selection | `stopPropagation` + pointerdown; card stays unselected |
