# Shortcut Legend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Toggle a `Keyboard shortcuts` dialog with `?` that lists the detailed-design keys plus `?`, `i`, `Shift+h`/`Shift+l`, and Delete/Backspace.

**Architecture:** `ShortcutLegend` is a `role="dialog"` overlay. `TaskboardApp` toggles it on `?` when not typing. Escape closes it first. The panel is documentation only.

**Tech Stack:** React, Vitest, Testing Library. Spec §5 / §11. Issue: https://github.com/AI1411/taskboard/issues/58. Existing plan Task 5 (legend only).

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- UI copy is English
- Not a command palette or command runner
- `?` in an input does not open the legend
- Toasts / Shift+h/l behavior stay out of scope (issue #59); the legend may still document them

## File map

- Create: `packages/ui/src/ShortcutLegend.tsx`
- Create: `packages/ui/src/ShortcutLegend.module.css`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Modify: `packages/ui/src/keyboard.test.tsx`
- Modify: `packages/ui/src/index.ts`

---

### Task 1: Shortcut legend overlay

**Files:**
- Create: `packages/ui/src/ShortcutLegend.tsx`, `packages/ui/src/ShortcutLegend.module.css`
- Modify: `packages/ui/src/TaskboardApp.tsx`, `packages/ui/src/keyboard.test.tsx`, `packages/ui/src/index.ts`

**Interfaces:**
- Consumes: none
- Produces:
  - `ShortcutLegend` is `role="dialog"` `aria-label="Keyboard shortcuts"`
  - Rows include New task, Inbox, `?`, `Shift+h` / `Shift+l`, Delete / Backspace
  - `?` toggles; second `?` closes; Escape closes; `?` while typing does not open

- [x] **Step 1: Write the failing tests** in `keyboard.test.tsx` (`question mark toggles shortcut legend`, `question mark in a field does not open the legend`).
- [x] **Step 2: Run `pnpm --filter @taskboard/ui test`** — FAIL missing dialog.
- [x] **Step 3: Implement** `ShortcutLegend` and `?` / Escape wiring.
- [x] **Step 4: Re-run UI tests** — PASS.
- [x] **Step 5: Commit** `feat: toggle keyboard shortcut legend with question mark`

---

## Self-review

1. **Spec coverage:** `?` overlay, English table, not a command runner, typing guard (§5, §11).
2. **Placeholder scan:** No TBD.
3. **Type consistency:** Dialog name is `Keyboard shortcuts`.
