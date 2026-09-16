# Non-modal inspector (issue #65) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Drop the full-board inspector backdrop so the board, sidebar, and Inbox stay clickable; clicking another card swaps panel content; Esc or empty board space closes the panel and clears selection without leaving a hidden `Select a card` shell.

**Architecture:** Keep the 360px `position: fixed` right-hand panel. Remove the `z-index: 15` backdrop button. `Inspector` renders only when `open && task`. `TaskboardApp` starts with `inspectorOpen=false`, closes with a single `closeInspector()` that also clears selection, and Board empty-space clicks call that helper. Focus moves to the panel when it opens and back to the board section when it closes.

**Tech Stack:** React, CSS Modules, Vitest + Testing Library. Shared UI in `packages/ui` (web + desktop). Issue: https://github.com/AI1411/taskboard/issues/65. Source: `docs/ui-improvements.md` §1.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- UI copy is English
- Do not add a command palette
- Do not change inspector section order
- Do not change link add/remove (#56)
- Usability epic #51 is closed; this PR may land on main
- Desktop and web share `packages/ui`; no URL state here (#74)
- Inbox, Trash, composers, project admin stay as they landed on `origin/main`
- Tests: `pnpm --filter @taskboard/ui test`

## File map

- Modify: `packages/ui/src/Inspector.tsx` — drop backdrop and empty `Select a card` shell; focus the panel when open
- Modify: `packages/ui/src/Inspector.module.css` — delete `.backdrop` / unused `.hidden` / `.empty` if unused
- Modify: `packages/ui/src/TaskboardApp.tsx` — `inspectorOpen` defaults false; `closeInspector()`; Esc clears selection; Board gets `onBackgroundClick`
- Modify: `packages/ui/src/Board.tsx` — empty-column / column-padding click dismisses
- Modify: `packages/ui/src/Board.module.css` — `tabIndex` target on `.board` if needed (`outline: none` on focus)
- Test: `packages/ui/src/inspector-design.test.tsx` — replace dismiss-backdrop cases
- Test: `packages/ui/src/Board.test.tsx` — flip “Select a card on load”
- Test: `packages/ui/src/keyboard.test.tsx` — Esc closes and clears selection

Do not split files. Do not add a new Inspector wrapper.

---

### Task 1: No backdrop; board and sidebar stay clickable

**Files:**
- Modify: `packages/ui/src/inspector-design.test.tsx`
- Modify: `packages/ui/src/Board.test.tsx`
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/Inspector.module.css`
- Modify: `packages/ui/src/TaskboardApp.tsx`

**Interfaces:**
- Consumes: existing `Inspector` props (`task`, `open`, `onClose`)
- Produces: Inspector returns `null` when `!open || !task`. No `Dismiss details` button. `inspectorOpen` initial value is `false`.

- [ ] **Step 1: Write the failing tests**

Replace the “closes the inspector when dismiss is clicked” test in `packages/ui/src/inspector-design.test.tsx` and add the no-backdrop / sidebar cases:

```tsx
  it("does not render a full-board dismiss backdrop while a card is open", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Open me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Open me/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Dismiss details" })).toBeNull();
    expect(screen.getByRole("list", { name: "Todo" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Taskboard" })).toBeTruthy();
  });

  it("lets the sidebar switch projects while the inspector is open", async () => {
    const transport = fakeTransport();
    const alpha = await transport.projectAdd({ name: "Alpha" });
    await transport.projectAdd({ name: "Beta" });
    await transport.taskCreate(alpha.slug, { title: "Alpha card", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Alpha card/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Beta" }));
    expect(screen.queryByLabelText("Title")).toBeNull();
    expect(screen.queryByText("Select a card")).toBeNull();
    expect(screen.getByRole("button", { name: "Beta" }).getAttribute("aria-current")).toBe("true");
  });
```

In `packages/ui/src/Board.test.tsx`, replace `shows inspector empty copy until a card is selected` with:

```tsx
  it("does not show inspector empty copy until a card is selected", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Untitled" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByRole("list", { name: "Todo" });
    expect(screen.queryByText("Select a card")).toBeNull();
    expect(screen.queryByLabelText("Title")).toBeNull();
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test -- src/inspector-design.test.tsx src/Board.test.tsx`

Expected: FAIL — `Dismiss details` is still present; `Select a card` still appears on load; clicking Beta may still leave the hidden empty panel.

- [ ] **Step 3: Write minimal implementation**

In `packages/ui/src/Inspector.tsx`:

1. Remove the `EmptyState` import if unused.
2. Change the early returns so a missing task never mounts a panel:

```tsx
  if (!props.open || !props.task) return null;
```

Delete the `if (!props.task) { return ( <aside className={...hidden}>Select a card</aside> ); }` block.

3. Delete the backdrop `<button aria-label="Dismiss details" ... />`. Return only the `<aside className={styles.panel} aria-label="Task details">...</aside>` (no fragment wrapper unless still needed).

In `packages/ui/src/Inspector.module.css`, delete `.backdrop`, `.hidden`, and `.empty`.

In `packages/ui/src/TaskboardApp.tsx`:

- Change `useState(true)` / `useRef(true)` for `inspectorOpen` to `false`.
- In `applyProject`, set `setInspectorOpen(false)` and `inspectorOpenRef.current = false` (do not reopen an empty panel when switching projects).
- In `addProject` / named project create, same: leave inspector closed until a card is selected or created.
- Keep `selectCard` and `createTask` setting `inspectorOpen` true.
- Change `onClose` to set `inspectorOpen` false (not true):

```tsx
        onClose={() => {
          setSelectedId(null);
          selectedIdRef.current = null;
          applyDetail(null);
          setConfirmDelete(false);
          confirmDeleteRef.current = false;
          setTrashedSelection(false);
          setInspectorOpen(false);
          inspectorOpenRef.current = false;
        }}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test -- src/inspector-design.test.tsx src/Board.test.tsx`

Expected: PASS for the new tests. Existing “keeps four columns visible while the inspector is open” still passes.

- [ ] **Step 5: Commit**

```bash
git add packages/ui/src/Inspector.tsx packages/ui/src/Inspector.module.css \
  packages/ui/src/TaskboardApp.tsx packages/ui/src/inspector-design.test.tsx \
  packages/ui/src/Board.test.tsx
git commit -m "fix(ui): drop inspector backdrop and empty Select a card shell"
```

---

### Task 2: Clicking a second card swaps inspector content

**Files:**
- Modify: `packages/ui/src/inspector-design.test.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx` (only if selectCard fails the test)

**Interfaces:**
- Consumes: `selectCard(displayId)` already sets `inspectorOpen=true` and `taskShow`
- Produces: same inspector panel; title/displayId update without an explicit close

- [ ] **Step 1: Write the failing test**

Append to `packages/ui/src/inspector-design.test.tsx`:

```tsx
  it("swaps inspector content when a second card is clicked", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "First card", column: "todo" });
    await transport.taskCreate(project.slug, { title: "Second card", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /First card/ }));
    expect(await screen.findByLabelText("Title")).toHaveValue("First card");
    expect(screen.getByText("TASK-1")).toBeTruthy();
    await userEvent.click(screen.getByRole("listitem", { name: /Second card/ }));
    await waitFor(() => expect(screen.getByLabelText("Title")).toHaveValue("Second card"));
    expect(screen.getByText("TASK-2")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Dismiss details" })).toBeNull();
  });
```

- [ ] **Step 2: Run test to verify it fails or already passes**

Run: `pnpm --filter @taskboard/ui test -- src/inspector-design.test.tsx`

Expected after Task 1: this should PASS because the backdrop no longer intercepts the second click and `selectCard` already loads the new detail. If it fails because the first card’s title input still shows, the Inspector `lastId` effect is not updating — then fix `Inspector` to reset title when `props.task.displayId` changes (already present; debug if stale).

- [ ] **Step 3: Write minimal implementation only if the test failed**

If the second click never reaches `selectCard`, confirm Board `onSelectCard` fires (Task 1 already removed the backdrop). No extra state machine.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @taskboard/ui test -- src/inspector-design.test.tsx`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/ui/src/inspector-design.test.tsx packages/ui/src/TaskboardApp.tsx packages/ui/src/Inspector.tsx
git commit -m "test(ui): swap inspector details when a second card is clicked"
```

If only the test file changed, commit just that file.

---

### Task 3: Esc and empty board space close and clear selection

**Files:**
- Modify: `packages/ui/src/keyboard.test.tsx`
- Modify: `packages/ui/src/inspector-design.test.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Modify: `packages/ui/src/Board.tsx`

**Interfaces:**
- Consumes: Board columns / empty slots; existing Escape handler
- Produces:
  - `closeInspector(): void` — sets `inspectorOpen` false, `selectedId` null, `detail` null, `confirmDelete` false, `trashedSelection` false
  - `Board` prop `onBackgroundClick?: () => void`
  - Escape (when not clearing search / confirm / trash) calls `closeInspector()`
  - Click on a column that is not a `role="listitem"` calls `onBackgroundClick`

- [ ] **Step 1: Write the failing tests**

Append to `packages/ui/src/keyboard.test.tsx`:

```tsx
  it("Escape closes the inspector and clears the selection", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Inspect me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Inspect me/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByLabelText("Title")).toBeNull();
    expect(screen.queryByText("Select a card")).toBeNull();
    expect(screen.getByRole("listitem", { name: /Inspect me/ }).getAttribute("aria-selected")).toBe(
      "false",
    );
  });
```

Append to `packages/ui/src/inspector-design.test.tsx`:

```tsx
  it("closes the inspector when empty board space is clicked", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Closable", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Closable/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    await userEvent.click(screen.getByRole("list", { name: "In Progress" }));
    expect(screen.queryByLabelText("Title")).toBeNull();
    expect(screen.queryByText("Select a card")).toBeNull();
    expect(screen.getByRole("listitem", { name: /Closable/ }).getAttribute("aria-selected")).toBe(
      "false",
    );
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test -- src/keyboard.test.tsx src/inspector-design.test.tsx`

Expected: FAIL — Escape hides the panel but leaves `aria-selected="true"`; clicking In Progress does not close.

- [ ] **Step 3: Write minimal implementation**

In `packages/ui/src/TaskboardApp.tsx`, add `closeInspector` next to the other callbacks:

```tsx
  const closeInspector = useCallback(() => {
    setSelectedId(null);
    selectedIdRef.current = null;
    applyDetail(null);
    setConfirmDelete(false);
    confirmDeleteRef.current = false;
    setTrashedSelection(false);
    setInspectorOpen(false);
    inspectorOpenRef.current = false;
  }, []);
```

Wire `Inspector` `onClose={closeInspector}`.

In the Escape handler, replace the bare `setInspectorOpen(false)` with `closeInspector()` (keep confirm-delete, trash, and search-clear branches first).

Pass the new Board prop:

```tsx
          <Board
            tasks={tasks}
            selectedId={selectedId}
            currentColumn={currentColumn}
            query={query}
            searchRef={searchRef}
            onQueryChange={(value) => {
              setQuery(value);
              queryRef.current = value;
            }}
            onSelectCard={(id) => void selectCard(id)}
            onMove={(id, column) => void onMove(id, column)}
            onReorder={(id, beforeId) => void onReorder(id, beforeId)}
            onBackgroundClick={closeInspector}
            composing={composingTask}
            composerRef={taskComposerRef}
            onComposerSubmit={(title) => void createTask(title)}
            onComposerCancel={() => setComposingTask(false)}
            onNewTask={() => setComposingTask(true)}
          />
```

In `packages/ui/src/Board.tsx`, add `onBackgroundClick?: () => void` to `Board` props. Thread it into `ColumnDrop`:

```tsx
              <ColumnDrop
                key={col.id}
                column={col.id}
                label={col.label}
                count={columnTasks.length}
                current={props.currentColumn === col.id}
                onBackgroundClick={props.onBackgroundClick}
              >
```

```tsx
function ColumnDrop(props: {
  column: Column;
  label: string;
  count: number;
  current: boolean;
  onBackgroundClick?: () => void;
  children: ReactNode;
}) {
  const { setNodeRef } = useDroppable({ id: props.column });
  return (
    <div
      className={`${styles.column} ${props.current ? styles.columnCurrent : ""}`}
      onClick={(event) => {
        if ((event.target as HTMLElement).closest('[role="listitem"]')) return;
        props.onBackgroundClick?.();
      }}
    >
      <h2 className={styles.header}>
        {props.label}
        <span className={styles.count}>{props.count}</span>
      </h2>
      <div ref={setNodeRef} role="list" aria-label={props.label} className={styles.list}>
        {props.children}
      </div>
    </div>
  );
}
```

Card clicks stay on the listitem and do not dismiss.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test -- src/keyboard.test.tsx src/inspector-design.test.tsx`

Expected: PASS. Existing `Enter opens the inspector` still works after `j` (selection via keyboard, no leftover empty panel).

- [ ] **Step 5: Commit**

```bash
git add packages/ui/src/TaskboardApp.tsx packages/ui/src/Board.tsx \
  packages/ui/src/keyboard.test.tsx packages/ui/src/inspector-design.test.tsx
git commit -m "fix(ui): close inspector and clear selection on Esc or empty space"
```

---

### Task 4: Focus stays in the panel while open; returns to the board on close

**Files:**
- Modify: `packages/ui/src/inspector-design.test.tsx`
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/Board.tsx`
- Modify: `packages/ui/src/Board.module.css`
- Modify: `packages/ui/src/TaskboardApp.tsx`

**Interfaces:**
- Consumes: `closeInspector`, Inspector `open` + `task`
- Produces:
  - Inspector `<aside tabIndex={-1} ref={panelRef}>`; focus when `open && task.displayId` changes
  - Board `<section tabIndex={-1} aria-label="Board" ref={boardRef}>`
  - `closeInspector` calls `boardRef.current?.focus()`

- [ ] **Step 1: Write the failing test**

Append to `packages/ui/src/inspector-design.test.tsx`:

```tsx
  it("moves focus into the inspector on open and back to the board on close", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Focus me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Focus me/ }));
    const panel = await screen.findByRole("complementary", { name: "Task details" });
    expect(panel.contains(document.activeElement)).toBe(true);
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("complementary", { name: "Task details" })).toBeNull();
    expect(document.activeElement).toBe(screen.getByRole("region", { name: "Board" }));
  });
```

`<aside aria-label="Task details">` is `complementary`. Board section needs `role="region"` + `aria-label="Board"` (or keep `<section aria-label="Board">`, which Testing Library maps to region).

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @taskboard/ui test -- src/inspector-design.test.tsx`

Expected: FAIL — activeElement is the clicked card or `<body>`; no `region` named Board.

- [ ] **Step 3: Write minimal implementation**

In `packages/ui/src/Inspector.tsx`:

```tsx
  const panelRef = useRef<HTMLElement>(null);

  useEffect(() => {
    if (!props.open || !props.task) return;
    panelRef.current?.focus();
  }, [props.open, props.task]);
```

On the aside:

```tsx
      <aside
        ref={panelRef}
        className={styles.panel}
        aria-label="Task details"
        tabIndex={-1}
      >
```

In `packages/ui/src/Board.tsx`, add optional `boardRef?: Ref<HTMLElement>` **or** set attributes on the existing `<section>`:

```tsx
  return (
    <section className={styles.board} aria-label="Board" tabIndex={-1}>
```

If `closeInspector` needs a ref, add `boardRef` to Board props and attach it to that section. In `TaskboardApp.tsx`:

```tsx
  const boardRef = useRef<HTMLElement>(null);
```

```tsx
  const closeInspector = useCallback(() => {
    setSelectedId(null);
    selectedIdRef.current = null;
    applyDetail(null);
    setConfirmDelete(false);
    confirmDeleteRef.current = false;
    setTrashedSelection(false);
    setInspectorOpen(false);
    inspectorOpenRef.current = false;
    boardRef.current?.focus();
  }, []);
```

Pass `boardRef={boardRef}` into Board.

In `packages/ui/src/Board.module.css`:

```css
.board:focus {
  outline: none;
}
```

Do not steal focus from the title input after `e` (the effect runs on `props.task` identity; `e` only focuses title while the same task is open — `props.task` reference may change on refresh). Depend on `props.task?.displayId` instead of `props.task` so note autosave does not yank focus:

```tsx
  useEffect(() => {
    if (!props.open || !props.task) return;
    panelRef.current?.focus();
  }, [props.open, props.task?.displayId]);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test -- src/inspector-design.test.tsx src/keyboard.test.tsx src/Board.test.tsx`

Expected: PASS. `e` still focuses the Title field (press `e` after open; title input is inside the panel so `panel.contains(activeElement)` stays true).

- [ ] **Step 5: Commit**

```bash
git add packages/ui/src/Inspector.tsx packages/ui/src/Board.tsx \
  packages/ui/src/Board.module.css packages/ui/src/TaskboardApp.tsx \
  packages/ui/src/inspector-design.test.tsx
git commit -m "fix(ui): keep inspector focus in the panel and restore it to the board"
```

---

### Task 5: Full UI test pass and Inbox clickability check

**Files:**
- Modify: `packages/ui/src/inspector-design.test.tsx` (Inbox toggle case)

**Interfaces:**
- Consumes: `InboxStrip` `aria-label="Inbox"` and toggle button
- Produces: no behavior change beyond proving the strip is not covered

- [ ] **Step 1: Write the failing test**

`fakeTransport` Inbox needs a waiting/failed/urgent card so the strip mounts. Use a waiting run if the fake supports `runStart` / `runWait`; otherwise set `displayStatus` via the fake’s task factory. On current `origin/main`, `fakeTransport` exposes `runStart` / `runWait` like the real transport. Append:

```tsx
  it("lets Inbox expand while the inspector is open", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const created = await transport.taskCreate(project.slug, { title: "Blocked", column: "todo" });
    const run = await transport.runStart(created.displayId, { agent: "cursor" });
    await transport.runWait(run.displayId, { reason: "need review" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Blocked/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    const inbox = screen.getByRole("region", { name: "Inbox" });
    await userEvent.click(screen.getByRole("button", { name: /Inbox/ }));
    expect(inbox.textContent).toMatch(/Blocked/);
  });
```

If `runStart` / `runWait` names differ, read `packages/ui/src/fakeTransport.ts` and `packages/ui/src/InboxStrip.test.tsx` and copy the same setup those tests use (do not invent a second fake).

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @taskboard/ui test -- src/inspector-design.test.tsx`

Expected: FAIL only if Inbox is still covered or the strip does not expand. After Task 1 this may already PASS — keep the test either way.

- [ ] **Step 3: Write minimal implementation only if it failed**

No extra z-index. Inbox lives in `main` under the board; without the backdrop it is already clickable.

- [ ] **Step 4: Run the full UI suite**

Run: `pnpm --filter @taskboard/ui test`

Expected: all `@taskboard/ui` tests PASS. Then `pnpm --filter web build` if JS/CSS changed.

- [ ] **Step 5: Commit**

```bash
git add packages/ui/src/inspector-design.test.tsx
git commit -m "test(ui): Inbox stays usable while the inspector is open"
```

---

## Self-review

**Spec coverage**

| Acceptance | Task |
| --- | --- |
| Opening a card does not cover sidebar, Inbox, or the four columns with a backdrop | Task 1, Task 5 |
| Clicking a second card updates the panel without an explicit close | Task 2 |
| Esc / empty-space click closes the panel and clears the selection | Task 3 |
| Closing does not leave a hidden `Select a card` panel behind | Task 1, Task 3 |
| Focus in panel while open; back to board on close | Task 4 |
| Keep 360px right panel | unchanged CSS width |
| Out of scope: command palette, section order, #56 links | no tasks touch those |

**Placeholder scan:** no TBD / “add tests later” / “similar to Task N”.

**Type consistency:** `closeInspector(): void`, `onBackgroundClick?: () => void`, `boardRef` on Board, Inspector still `open: boolean` + `task: TaskDetail | null`.
