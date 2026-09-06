# Linear-dark Kanban Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restyle the local Taskboard web/desktop UI to Linear-dark Kanban: near-black surfaces, Inter, indigo only on primary actions and selection, overlay inspector on a full-width board.

**Architecture:** Keep `packages/ui` components and keyboard/DnD behavior. Replace Native Glass tokens in `theme.css`, flatten CSS modules, and change the shell from a 3-column grid to sidebar + board with a positioned inspector overlay. Empty “Select a card” stays in the DOM (hidden) so existing tests keep passing.

**Tech Stack:** React, CSS Modules, Vitest + Testing Library, Vite (`apps/web`). Spec: `docs/superpowers/specs/2026-09-06-linear-dark-kanban-design.md`.

**Out of scope:** Issue list, command palette, Trash view, URL state, CLI/Rust/API, new font npm packages.

**Verify always:** `pnpm --filter @taskboard/ui test` (28 tests today). After UI CSS/JS: `pnpm --filter web build`.

---

## File map

- Modify: `apps/web/index.html` — Inter `<link>`, `color-scheme` on `<html>`
- Modify: `packages/ui/src/theme.css` — Linear-dark tokens + Inter stack
- Modify: `packages/ui/src/Card.module.css` — flat cards, no glass/shadow
- Modify: `packages/ui/src/Board.module.css` — dark columns, 8px radius
- Modify: `packages/ui/src/Sidebar.module.css` — hairline, no extra hex
- Modify: `packages/ui/src/Search.module.css` — dark field
- Modify: `packages/ui/src/Inspector.module.css` — overlay panel + backdrop
- Modify: `packages/ui/src/TaskboardApp.module.css` — always 2-column shell
- Modify: `packages/ui/src/TaskboardApp.tsx` — overlay dismiss handler
- Modify: `packages/ui/src/Inspector.tsx` — backdrop, `onClose`
- Modify: `packages/ui/src/inspector-design.test.tsx` — overlay tests

Do not split files. Do not add list view components.

---

### Task 1: Inter and dark tokens

**Files:**
- Modify: `apps/web/index.html`
- Modify: `packages/ui/src/theme.css`
- Test: `packages/ui/src/inspector-design.test.tsx`

- [ ] **Step 1: Write the failing test**

Append to `packages/ui/src/inspector-design.test.tsx`:

```tsx
  it("uses a dark color-scheme on the document", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Untitled" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByRole("list", { name: "Todo" });
    const scheme = getComputedStyle(document.documentElement).colorScheme;
    expect(scheme).toMatch(/dark/);
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @taskboard/ui test -- src/inspector-design.test.tsx`

Expected: FAIL — `colorScheme` is `normal` or empty (theme.css has no `color-scheme: dark`).

- [ ] **Step 3: Write minimal implementation**

Replace `apps/web/index.html` with:

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="color-scheme" content="dark" />
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link
      href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;650&display=swap"
      rel="stylesheet"
    />
    <title>Taskboard</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

Replace `packages/ui/src/theme.css` with:

```css
:root {
  --sidebar-bg: #0c0d0e;
  --board-bg: #08090a;
  --accent: #5e6ad2;
  --accent-2: #5e6ad2;
  --urgent: #e24a2b;
  --waiting: #d89a1a;
  --completed: #2f9e62;
  --running: #5e6ad2;
  --sidebar-fg: #e8e8ec;
  --sidebar-muted: #8a8f98;
  --board-fg: #e8e8ec;
  --card-bg: #16171a;
  --card-border: #23242a;
  --failed: #c45c4a;
  --panel-bg: #101113;
  --hairline: #1b1c1f;
  --font: Inter, "SF Pro Text", "Segoe UI", system-ui, sans-serif;
}

*,
*::before,
*::after {
  box-sizing: border-box;
}

html {
  color-scheme: dark;
}

html,
body {
  margin: 0;
  min-height: 100%;
  font-family: var(--font);
  background: var(--board-bg);
  color: var(--board-fg);
}

:focus-visible {
  outline: 2px solid var(--accent-2);
  outline-offset: 2px;
}
```

In `packages/ui/src/TaskboardApp.module.css`, change `.app` `font-family` to `var(--font)`.

- [ ] **Step 4: Run tests**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS (29 tests). If Inter weight `650` 404s in the browser later, drop it to `600` in the Google Fonts URL only — do not add an npm font package.

- [ ] **Step 5: Commit**

```bash
git add apps/web/index.html packages/ui/src/theme.css packages/ui/src/TaskboardApp.module.css packages/ui/src/inspector-design.test.tsx
git commit -m "style: Linear-dark tokens and Inter"
```

---

### Task 2: Flat cards

**Files:**
- Modify: `packages/ui/src/Card.module.css`
- Test: `packages/ui/src/Card.test.tsx` (run only; do not change assertions)

- [ ] **Step 1: Run existing card tests as baseline**

Run: `pnpm --filter @taskboard/ui test -- src/Card.test.tsx`

Expected: PASS (4 tests). These cover badges, ids, lift placeholder — not shadows.

- [ ] **Step 2: Flatten card chrome**

Replace `packages/ui/src/Card.module.css` with:

```css
.card {
  display: grid;
  grid-template-columns: 1fr auto;
  grid-template-rows: auto auto auto;
  gap: 0.2rem 0.5rem;
  padding: 0.6rem 0.7rem;
  border-radius: 6px;
  border: 1px solid var(--card-border);
  background: var(--card-bg);
  color: var(--board-fg);
  cursor: grab;
  box-shadow: none;
}

.card:active {
  cursor: grabbing;
}

.placeholder {
  opacity: 0.4;
  box-shadow: none;
}

.lifted {
  cursor: grabbing;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
  transform: rotate(1.5deg) scale(1.03);
}

@media (prefers-reduced-motion: reduce) {
  .lifted {
    transform: none;
  }
}

.card[aria-selected="true"] {
  border-color: var(--accent);
  box-shadow: none;
}

.card.lifted[aria-selected="true"] {
  border-color: var(--accent);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
}

.title {
  grid-column: 1;
  font-weight: 500;
  font-size: 0.875rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.displayId {
  grid-column: 1;
  color: var(--sidebar-muted);
  font-size: 0.72rem;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0;
}

.meta {
  grid-column: 2;
  grid-row: 1 / span 2;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.35rem;
}

.pip {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--urgent);
}

.badge {
  font-size: 0.65rem;
  font-weight: 600;
  letter-spacing: 0;
  text-transform: none;
  padding: 0.12rem 0.4rem;
  border-radius: 999px;
  color: #fff;
}

.waiting {
  background: var(--waiting);
}

.running {
  background: var(--running);
}

.failed {
  background: var(--failed);
}

.completed {
  background: var(--completed);
}

.runMessage {
  grid-column: 1 / -1;
  font-size: 0.75rem;
  color: var(--sidebar-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
```

No `backdrop-filter`. Selected = 1px accent border only. Lift shadow is allowed on the drag overlay only.

- [ ] **Step 3: Run tests**

Run: `pnpm --filter @taskboard/ui test -- src/Card.test.tsx`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add packages/ui/src/Card.module.css
git commit -m "style: flatten Kanban cards to Linear-dark"
```

---

### Task 3: Dark board, sidebar, search

**Files:**
- Modify: `packages/ui/src/Board.module.css`
- Modify: `packages/ui/src/Sidebar.module.css`
- Modify: `packages/ui/src/Search.module.css`

- [ ] **Step 1: Run UI tests as baseline**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS.

- [ ] **Step 2: Restyle board columns and search hint**

In `packages/ui/src/Board.module.css`:

- `.column`: `border-radius: 8px; background: var(--panel-bg); border: 1px solid var(--hairline);` — delete the `color-mix(..., #fff 55%, ...)` background.
- `.header`: `color: var(--sidebar-muted); font-weight: 500;`
- `.count`: `color: var(--sidebar-muted);`
- `.slot`: `border-radius: 6px;`
- `.searchHint`: `background: var(--panel-bg); color: var(--sidebar-muted); border-color: var(--hairline);` — no `#fff` mix.
- `.columnCurrent`: keep inset accent ring via `box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 35%, transparent);`

In `packages/ui/src/Search.module.css` keep structure; ensure `background: var(--card-bg); border-color: var(--hairline); color: var(--board-fg);`

In `packages/ui/src/Sidebar.module.css`:

- `.note textarea`: `border: 1px solid var(--hairline); background: var(--board-bg);` — delete `color-mix(..., #fff ...)`.
- `.newProject` / `.project[aria-current="true"]` already use `var(--accent)`; leave behavior, keep `min-height: 44px`.
- `.toggle[aria-pressed="true"]`: `color: var(--accent);`

Do not add new hex except through tokens already in `theme.css`.

- [ ] **Step 3: Run tests**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add packages/ui/src/Board.module.css packages/ui/src/Sidebar.module.css packages/ui/src/Search.module.css
git commit -m "style: dark board columns and sidebar hairlines"
```

---

### Task 4: Overlay inspector

**Files:**
- Modify: `packages/ui/src/TaskboardApp.module.css`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/Inspector.module.css`
- Test: `packages/ui/src/inspector-design.test.tsx`

Behavior:

- Shell is always `240px + 1fr`. Board never shrinks for details.
- Empty inspector (`open && !task`) stays mounted with “Select a card” and `display: none`.
- Open inspector (`open && task`) is `position: fixed` width `360px`, right/top/bottom 12px, z-index 20.
- Backdrop button `aria-label="Dismiss details"` sits under the panel; click calls `onClose`.
- Escape already sets `inspectorOpen` false in `TaskboardApp` — keep it.
- Overlay transition `transform/opacity` 180ms ease-out; `@media (prefers-reduced-motion: reduce) { transition: none }`.

- [ ] **Step 1: Write the failing tests**

Append to `packages/ui/src/inspector-design.test.tsx`:

```tsx
  it("keeps four columns visible while the inspector is open", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Keep columns", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Keep columns/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    expect(screen.getByRole("list", { name: "Todo" })).toBeTruthy();
    expect(screen.getByRole("list", { name: "In Progress" })).toBeTruthy();
    expect(screen.getByRole("list", { name: "In Review" })).toBeTruthy();
    expect(screen.getByRole("list", { name: "Done" })).toBeTruthy();
  });

  it("closes the inspector when dismiss is clicked", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Closable", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Closable/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Dismiss details" }));
    expect(screen.queryByLabelText("Title")).toBeNull();
    expect(screen.getByText("Select a card")).toBeTruthy();
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test -- src/inspector-design.test.tsx`

Expected: first new test may pass today (columns exist even when grid is 3-col). Second test FAIL — no `Dismiss details` button.

If the four-column test passes before overlay work, keep it as a regression. The dismiss test is the red bar for this task.

- [ ] **Step 3: Overlay CSS**

Replace `packages/ui/src/TaskboardApp.module.css` with:

```css
.app {
  display: grid;
  grid-template-columns: 240px minmax(0, 1fr);
  min-height: 100vh;
  font-family: var(--font);
  background: var(--board-bg);
  position: relative;
}

.main {
  display: flex;
  flex-direction: column;
  min-width: 0;
  padding: 0.9rem 1rem 1.25rem;
  overflow: auto;
}

@media (max-width: 800px) {
  .app {
    grid-template-columns: 1fr;
    grid-template-rows: auto minmax(0, 1fr);
  }
}
```

Delete `.collapsed` and `.collapsed > aside:last-child`. The 3-column template must not return.

Replace overlay-related rules in `packages/ui/src/Inspector.module.css` (keep `.field`, `.note`, `.delete`, `.deleteRow`, `.cancel` as they are, but retarget colors to tokens). Put these at the top of the file, replacing `.panel` and `.empty`:

```css
.backdrop {
  position: fixed;
  inset: 0;
  border: 0;
  padding: 0;
  margin: 0;
  background: rgba(0, 0, 0, 0.35);
  cursor: pointer;
  z-index: 15;
}

.panel {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  position: fixed;
  top: 12px;
  right: 12px;
  bottom: 12px;
  width: 360px;
  max-width: calc(100vw - 24px);
  padding: 1rem 1rem 1.25rem;
  background: var(--panel-bg);
  border: 1px solid var(--hairline);
  border-radius: 10px;
  color: var(--board-fg);
  overflow: auto;
  z-index: 20;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  transition: opacity 180ms ease-out, transform 180ms ease-out;
  transform: translateX(0);
}

.hidden {
  display: none;
}

@media (prefers-reduced-motion: reduce) {
  .panel {
    transition: none;
  }
}
```

Also change `.field input/select/textarea` `background` to `var(--card-bg)` and `border-color` to `var(--hairline)`. Change `.heading` / `.displayId` muted color to `var(--sidebar-muted)`.

- [ ] **Step 4: Overlay markup**

In `packages/ui/src/Inspector.tsx`, add `onClose?: () => void` to props.

Replace the empty-task return:

```tsx
  if (!props.task) {
    return (
      <aside className={`${styles.panel} ${styles.hidden}`}>
        <div className={styles.empty}>
          <EmptyState>Select a card</EmptyState>
        </div>
      </aside>
    );
  }
```

Replace the populated return wrapper with:

```tsx
  return (
    <>
      <button
        type="button"
        className={styles.backdrop}
        aria-label="Dismiss details"
        onClick={() => props.onClose?.()}
      />
      <aside className={styles.panel} role="dialog" aria-label="Task details">
```

Close with `</aside></>` instead of a single `</aside>`.

Do not remove delete confirm (`Move to Trash` / `Cancel`).

In `packages/ui/src/TaskboardApp.tsx`:

1. Change the shell className from `` `${styles.app} ${inspectorOpen && detail ? "" : styles.collapsed}` `` to `styles.app`.
2. Pass `onClose` into `Inspector`:

```tsx
        onClose={() => {
          setInspectorOpen(false);
          inspectorOpenRef.current = false;
        }}
```

Keep `open={inspectorOpen}` so Escape still unmounts via `if (!props.open) return null`. When Escape fires, “Select a card” disappears until the next project apply sets `inspectorOpen` true again — that matches `keyboard.test.tsx` (“Select a card” is null while a titled inspector is open). After dismiss click, `open` is false so “Select a card” is also null unless we keep `open` true.

**Dismiss vs Escape:** Spec wants click-away to close. Existing Escape sets `open` false. For the dismiss test `getByText("Select a card")` after click, `onClose` must hide the dialog but keep `open` true with `detail` cleared, OR keep `open` true and only clear selection.

Use this `onClose` so the empty hidden node remains (matches the dismiss test and Board.test on later project select):

```tsx
        onClose={() => {
          setSelectedId(null);
          selectedIdRef.current = null;
          applyDetail(null);
          setInspectorOpen(true);
          inspectorOpenRef.current = true;
        }}
```

Escape in `TaskboardApp` currently only sets `inspectorOpen` false and does not clear `detail`. Leave Escape as-is (keyboard test). Dismiss click uses `onClose` above.

- [ ] **Step 5: Run tests**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS, including new overlay tests and existing `shows inspector empty copy until a card is selected`.

If `role="dialog"` breaks a query, drop `role="dialog"` and keep `aria-label` on the aside only.

- [ ] **Step 6: Commit**

```bash
git add packages/ui/src/TaskboardApp.tsx packages/ui/src/TaskboardApp.module.css packages/ui/src/Inspector.tsx packages/ui/src/Inspector.module.css packages/ui/src/inspector-design.test.tsx
git commit -m "feat: overlay task inspector on the full-width board"
```

---

### Task 5: Dist build

**Files:** none in git except whatever Task 1–4 already committed. `apps/web/dist` is gitignored.

- [ ] **Step 1: Build web**

Run: `pnpm --filter web build`

Expected: Vite success, CSS/JS hashed into `apps/web/dist`.

- [ ] **Step 2: Smoke the live UI**

If `task start` / `tb serve` is running, reload `http://127.0.0.1:<port>/`. Check: dark board, 4 columns with no card selected, overlay on click, Escape and dismiss close it, New task still works, delete still confirms.

No extra commit unless the smoke forces a one-line CSS fix. If you fix, commit `style: Linear-dark smoke fix` with only that file.

---

## Spec coverage

| Spec section | Task |
|--------------|------|
| §3 tokens, Inter, color-scheme, no glass | 1, 2, 3 |
| §4 cards, 4 columns, no inspector when empty | 2, 4 |
| §5 overlay 360px, Escape, click-away, reduced motion, delete confirm | 4 |
| §6 keep shortcuts / skip list, palette, Trash, URL | constraints (no tasks) |
| §8 tests + web build | 4, 5 |

## Placeholder scan

No TBD. Overlay dismiss vs Escape is specified in Task 4. Font CDN is Google Fonts `<link>` only, no npm package.
