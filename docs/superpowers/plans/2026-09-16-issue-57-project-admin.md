# Project Admin UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose rename, repository path, archive, soft-delete, and reorder for the selected project from the sidebar.

**Architecture:** The selected project name becomes an input committed on blur via `projectUpdate({ name })`. A `⋯` menu (`aria-label="Project actions"`) hosts Set repository path, Archive/Unarchive, Delete (confirm), and Move up/down (`projectReorder` with adjacent slugs).

**Tech Stack:** React, Vitest, Testing Library. Spec §9. Issue: https://github.com/AI1411/taskboard/issues/57. Existing plan Task 7 (project name + ⋯ menu only).

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Archived toggle remains a filter using `projectList(true)`
- Soft-delete uses existing `projectDelete`; restore stays in Trash
- UI copy is English
- Last-project persistence is out of scope (issue #59)

## File map

- Create: `packages/ui/src/Sidebar.test.tsx`
- Modify: `packages/ui/src/Sidebar.tsx`
- Modify: `packages/ui/src/Sidebar.module.css`
- Modify: `packages/ui/src/TaskboardApp.tsx`

---

### Task 1: Selected name and project actions menu

**Files:**
- Create: `packages/ui/src/Sidebar.test.tsx`
- Modify: `packages/ui/src/Sidebar.tsx`
- Modify: `packages/ui/src/Sidebar.module.css`
- Modify: `packages/ui/src/TaskboardApp.tsx`

**Interfaces:**
- Consumes: `projectUpdate`, `projectArchive`, `projectDelete`, `projectReorder`
- Produces:
  - Selected name input `aria-label="Project name"`; blur with trimmed non-empty name calls `onRename(name)`
  - Menu button `aria-label="Project actions"`
  - Menu items: `Set repository path`, `Archive` or `Unarchive`, `Delete project`, `Move up`, `Move down`
  - Path field placeholder `Repository path`; blur/Enter calls `onSetPath(value)`
  - Delete project uses `ConfirmDialog` message `Delete {name}?`
  - Move up/down swap adjacent slugs and call `onReorder(slugs)`

- [x] **Step 1: Write the failing tests** in `Sidebar.test.tsx` (rename on blur; path/archive/delete/reorder from the menu).
- [x] **Step 2: Run `pnpm --filter @taskboard/ui test`** — FAIL missing Project actions.
- [x] **Step 3: Implement** name input + menu + TaskboardApp handlers.
- [x] **Step 4: Re-run UI tests** — PASS.
- [x] **Step 5: Commit** `feat: add sidebar project rename path archive delete and reorder`

---

## Self-review

1. **Spec coverage:** Rename, path, archive, delete confirm, move up/down (§9).
2. **Placeholder scan:** No TBD.
3. **Type consistency:** `projectUpdate({ name })` / `{ repoPath }`, `projectArchive(slug, bool)`, `projectDelete`, `projectReorder(slugs)`.
