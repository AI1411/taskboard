# Sidebar scroll and collapsed notes (issue #72) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The project list scrolls on its own. The selected project note is collapsed by default and toggled from a `Project note` header. A filled dot shows when the note has text. Archived / Trash stay pinned at the bottom.

**Architecture:** `Sidebar` keeps `noteOpen` (default false, reset on project change). Only the project `<ul>` gets `overflow-y: auto` and `max-height: 50vh`. Footer (`Archived projects`, `Trash`) uses `margin-top: auto`.

**Tech Stack:** React, CSS modules, Vitest. Issue: https://github.com/AI1411/taskboard/issues/72.

## Global Constraints

- Rename / ⋯ menu stay as #57
- Four columns unchanged
- Tests: `pnpm --filter @taskboard/ui test`

## File map

- Modify: `packages/ui/src/Sidebar.tsx`, `Sidebar.module.css`
- Test: `packages/ui/src/Sidebar.test.tsx`

---

### Task 1: Scroll list and collapse note

**Files:**
- Modify: `packages/ui/src/Sidebar.tsx`, `Sidebar.module.css`
- Test: `packages/ui/src/Sidebar.test.tsx`

**Interfaces:**
- Produces: `button` named `Project note` with `aria-expanded`
- Produces: `data-filled` dot when note text is non-empty
- Produces: project list `data-scroll="projects"`

- [ ] **Step 1: Write the failing test**

```tsx
  it("collapses the project note and marks when it has text", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    project.noteMarkdown = "ship notes";
    render(<TaskboardApp transport={transport} />);
    const toggle = await screen.findByRole("button", { name: "Project note" });
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(toggle.querySelector("[data-filled]")).toBeTruthy();
    expect(screen.queryByLabelText("Project note")).toBeNull();
    await userEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect((screen.getByLabelText("Project note") as HTMLTextAreaElement).value).toBe("ship notes");
    expect(screen.getByRole("list").getAttribute("data-scroll")).toBe("projects");
    expect(screen.getByRole("button", { name: "Archived projects" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Trash" })).toBeTruthy();
  });
```

- [ ] **Step 2: Run to verify fail**

- [ ] **Step 3: Implement**

- [ ] **Step 4: Run** `pnpm --filter @taskboard/ui test` — PASS

- [ ] **Step 5: Commit** `feat(ui): scroll the project list and collapse notes`

---

## Self-review

List-only scroll, note default closed, filled dot, footer pinned.
