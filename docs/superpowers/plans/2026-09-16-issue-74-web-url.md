# Web URL for project and card (issue #74) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Web loads `/?project=<slug>&task=TASK-n`, selects that project, and opens the inspector when the card exists. Switching project or card updates the URL with `history.replaceState`. Invalid slug or missing id still shows a board with no card selected. Desktop stays URL-less.

**Architecture:** Optional `route` / `onRouteChange` on `TaskboardApp`. Boot prefers `route.project` over `last_project`. Web `App` reads `window.location.search` once and writes via `replaceState`. Desktop does not pass these props.

**Tech Stack:** React, Vitest. Issue: https://github.com/AI1411/taskboard/issues/74.

## Global Constraints

- Desktop: no URL
- No command palette
- No error screen
- Tests: `pnpm --filter @taskboard/ui test` and `pnpm --filter web test`

## File map

- Modify: `packages/ui/src/TaskboardApp.tsx`, `packages/ui/src/index.ts`
- Create: `packages/ui/src/route.test.tsx`
- Modify: `apps/web/src/App.tsx`
- Create: `apps/web/src/route.ts`, `apps/web/src/route.test.ts`

---

### Task 1: Route props and web replaceState

- [ ] Write failing tests
- [ ] Implement
- [ ] Run tests
- [ ] Commit `feat(ui): sync web URL to project and task`

---

## Self-review

URL wins over last_project. Invalid ids do not error. Desktop unchanged.
