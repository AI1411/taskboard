# Readable run and activity rows (issue #73) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Inspector run rows show `RUN-n · agent · status · relative time` plus message / waiting reason / summary. Activity rows show `Moved · cursor · 2m ago`. Unknown operations fall back to a human label. No markdown preview.

**Architecture:** `relativeTime(iso, now)` for `2m ago`. Expand `ACTIVITY_LABELS`; unknown ops use the last dotted segment title-cased. Run status uses the same Waiting/Running/Failed/Done words as badges.

**Tech Stack:** React, Vitest. Issue: https://github.com/AI1411/taskboard/issues/73.

## Global Constraints

- No markdown preview
- Inbox unchanged
- Tests: `pnpm --filter @taskboard/ui test`

## File map

- Create: `packages/ui/src/relativeTime.ts`, `relativeTime.test.ts`
- Modify: `packages/ui/src/Inspector.tsx`, `Inspector.module.css`
- Test: `packages/ui/src/Inspector.test.tsx`

---

### Task 1: Readable rows

**Files:** as above

- [ ] **Step 1: Write failing tests** for `relativeTime` and inspector rows
- [ ] **Step 2: Run to verify fail**
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run** `pnpm --filter @taskboard/ui test` — PASS
- [ ] **Step 5: Commit** `feat(ui): make run and activity rows readable`

---

## Self-review

Run head + body lines, activity has relative time, unknown ops humanized.
