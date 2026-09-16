# Stronger run-state faces (issue #70) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Waiting / Failed / Running cards show a 2px left accent (Running pulses). Completed badges become muted text so Done is quieter.

**Architecture:** `Card` sets `data-face` and a face class from `displayStatus`. CSS draws an inset 2px bar; Running uses a short pulse unless `prefers-reduced-motion` or `stale`. Completed badge drops the green pill. Failed still uses the existing one-line `runMessage` / `waitingReason` summary.

**Tech Stack:** React, CSS modules, Vitest. Issue: https://github.com/AI1411/taskboard/issues/70.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Inbox strip is not replaced
- Stale badge from #82 stays Stale (no Running pulse)
- Tests: `pnpm --filter @taskboard/ui test`

## File map

- Modify: `packages/ui/src/Card.tsx`
- Modify: `packages/ui/src/Card.module.css`
- Test: `packages/ui/src/Card.test.tsx`

---

### Task 1: Face accents and muted Done

**Files:**
- Modify: `packages/ui/src/Card.tsx`, `Card.module.css`
- Test: `packages/ui/src/Card.test.tsx`

**Interfaces:**
- Produces: `data-face="waiting|failed|running|completed"` on the listitem
- Produces: completed badge uses muted (not pill) styling

- [ ] **Step 1: Write the failing test**

```tsx
  it("marks attention faces and mutes the completed badge", () => {
    const { getByRole, getByText, rerender } = render(
      <Card task={summary({ displayStatus: "waiting" })} selected={false} />,
    );
    expect(getByRole("listitem").getAttribute("data-face")).toBe("waiting");
    rerender(
      <Card
        task={summary({ displayStatus: "failed", waitingReason: "boom" })}
        selected={false}
      />,
    );
    expect(getByRole("listitem").getAttribute("data-face")).toBe("failed");
    expect(getByText("boom")).toBeTruthy();
    rerender(<Card task={summary({ displayStatus: "running" })} selected={false} />);
    expect(getByRole("listitem").getAttribute("data-face")).toBe("running");
    rerender(<Card task={summary({ displayStatus: "running", stale: true })} selected={false} />);
    expect(getByRole("listitem").getAttribute("data-face")).toBeNull();
    rerender(<Card task={summary({ displayStatus: "completed" })} selected={false} />);
    expect(getByRole("listitem").getAttribute("data-face")).toBe("completed");
    expect(getByText("Done").className).toMatch(/muted/);
  });
```

- [ ] **Step 2: Run to verify fail**

Run: `pnpm --filter @taskboard/ui test -- src/Card.test.tsx`

Expected: FAIL — no `data-face`.

- [ ] **Step 3: Implement**

On `Card` root: `data-face` for waiting / failed / running (not stale) / completed.

CSS: `box-shadow: inset 2px 0 0 <color>` for waiting (amber `--waiting`), failed (`--failed`), running (`--running`). Running adds a short pulse; `@media (prefers-reduced-motion: reduce)` disables it.

`.completed` badge: transparent background, muted text, no pill padding. Add `.muted` class on that badge.

- [ ] **Step 4: Run** `pnpm --filter @taskboard/ui test` — PASS

- [ ] **Step 5: Commit** `feat(ui): strengthen waiting, failed, and running card faces`

---

## Self-review

2px accents, Running pulse only, reduced-motion static, Failed still one-line, Completed muted, Stale unchanged.
