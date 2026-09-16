# Open existing links (issue #71) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Inspector links are actionable: `http`/`https`/`file` open in a new tab; path links copy on click. Add/remove stays as #56.

**Architecture:** `openableHref` classifies a link value. Openable values render as `<a target="_blank" rel="noreferrer">`. Paths render as a button that reuses `onCopyId` (toast `Copied`). No new backend or Tauri reveal command.

**Tech Stack:** React, Vitest. Issue: https://github.com/AI1411/taskboard/issues/71.

## Global Constraints

- Add/remove controls stay as they are
- No markdown preview
- Desktop uses the same `<a>` / clipboard UI (webview / OS opener)
- Tests: `pnpm --filter @taskboard/ui test`

## File map

- Create: `packages/ui/src/openableHref.ts` (optional helper)
- Modify: `packages/ui/src/Inspector.tsx`, `Inspector.module.css`
- Test: `packages/ui/src/Inspector.test.tsx`

---

### Task 1: Open and copy links

**Files:**
- Modify: `packages/ui/src/Inspector.tsx`, `Inspector.module.css`
- Test: `packages/ui/src/Inspector.test.tsx`

**Interfaces:**
- Produces: `role=link` for `https://` and `file:`
- Produces: path click copies via `onCopyId`

- [ ] **Step 1: Write the failing test**

```tsx
  it("opens URL and file links and copies a path", async () => {
    const writeText = vi.fn();
    Object.assign(navigator, { clipboard: { writeText } });
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Linked", column: "todo" });
    task.links = [
      { id: "l1", taskId: task.id, kind: "url", value: "https://example.com", sortOrder: 0 },
      { id: "l2", taskId: task.id, kind: "url", value: "file:///tmp/log.txt", sortOrder: 1 },
      { id: "l3", taskId: task.id, kind: "path", value: "/tmp/notes.md", sortOrder: 2 },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Linked"));
    const url = await screen.findByRole("link", { name: "https://example.com" });
    expect(url.getAttribute("href")).toBe("https://example.com");
    expect(url.getAttribute("target")).toBe("_blank");
    expect(screen.getByRole("link", { name: "file:///tmp/log.txt" }).getAttribute("href")).toBe(
      "file:///tmp/log.txt",
    );
    await userEvent.click(screen.getByRole("button", { name: "/tmp/notes.md" }));
    expect(writeText).toHaveBeenCalledWith("/tmp/notes.md");
    expect(await screen.findByText("Copied")).toBeTruthy();
  });
```

- [ ] **Step 2: Run to verify fail** — no `role=link`

- [ ] **Step 3: Implement** — `<a>` for openable hrefs; path button calls `onCopyId(value)`; hover underline

- [ ] **Step 4: Run** `pnpm --filter @taskboard/ui test` — PASS

- [ ] **Step 5: Commit** `feat(ui): open URL links and copy path links`

---

## Self-review

Add/remove unchanged. Hover underline. No Reveal without an opener API.
