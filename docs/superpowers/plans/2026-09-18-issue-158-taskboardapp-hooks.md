# Split TaskboardApp into concern hooks (#158) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `TaskboardApp` mostly wiring by moving board data, selection, keyboard, and route sync into hooks; unify Inspector mutate happy-path; cut manual state/ref mirrors via `useLatestRef`.

**Architecture:** Extract `packages/ui/src/hooks/{useLatestRef,useBoardData,useSelection,useBoardKeyboard,useRouteSync}.ts`. Shell owns layout/JSX and toast/composer/trash UI flags that are purely presentational. `mutateSelected` lives on selection (or board) and wraps “call transport → `taskShow` → `applyDetail` → optional `refreshTasks`”. Do **not** change revision-conflict empty `catch {}` bodies.

**Tech Stack:** React 18, Vitest, `@taskboard/ui`, existing `keyboard.test.tsx` / `route.test.tsx` / related UI tests.

## Global Constraints

- Behavior-preserving only — no visual redesign, no new board features
- Do not touch conflict-refresh empty `catch` behavior
- Inspector section split / shared `AddRow` is out of scope
- Existing UI tests must stay green

User already chose sequential inline execution — do not ask Subagent-Driven vs Inline.

## File map

- Create: `packages/ui/src/hooks/useLatestRef.ts`
- Create: `packages/ui/src/hooks/useBoardData.ts`
- Create: `packages/ui/src/hooks/useSelection.ts`
- Create: `packages/ui/src/hooks/useBoardKeyboard.ts`
- Create: `packages/ui/src/hooks/useRouteSync.ts`
- Modify: `packages/ui/src/TaskboardApp.tsx` — thin shell
- Test: `packages/ui` vitest suite (`pnpm --filter @taskboard/ui test` or `npm test` in package)

---

### Task 1: `useLatestRef` + replace render-sync mirrors

**Files:**
- Create: `packages/ui/src/hooks/useLatestRef.ts`
- Modify: `packages/ui/src/TaskboardApp.tsx`

**Interfaces:**
- Produces: `function useLatestRef<T>(value: T): MutableRefObject<T>`

- [ ] **Step 1: Add helper**

```ts
import { useRef, type MutableRefObject } from "react";

export function useLatestRef<T>(value: T): MutableRefObject<T> {
  const ref = useRef(value);
  ref.current = value;
  return ref;
}
```

- [ ] **Step 2: Replace `fooRef.current = foo` block** with `useLatestRef` for read-only snapshots used by effects/handlers. Keep intentional `ref.current = …` writes inside setters that must sync before the next paint (`closeInspector`, `selectCard`, etc.).

- [ ] **Step 3:** `pnpm --filter @taskboard/ui test` (or package script) — PASS

---

### Task 2: `useRouteSync` + `useBoardKeyboard`

**Files:**
- Create: `packages/ui/src/hooks/useRouteSync.ts`
- Create: `packages/ui/src/hooks/useBoardKeyboard.ts`
- Modify: `TaskboardApp.tsx`

**Interfaces:**
- `useRouteSync({ transport, route, onRouteChange, applyProject, selectCard, selectedProject, selectedId, tasksRef })`
- `useBoardKeyboard({ …refs, handlers, setters })` — owns the keydown `useEffect` + `isTypingTarget`

- [ ] Move bootstrap effect (project list + route/remembered project + optional `selectCard`) and `onRouteChange` emit effect into `useRouteSync`.
- [ ] Move ~150-line keydown effect into `useBoardKeyboard`.
- [ ] Run `keyboard` + `route` tests — PASS

---

### Task 3: `useBoardData` + `useSelection` + `mutateSelected`

**Files:**
- Create: `packages/ui/src/hooks/useBoardData.ts`
- Create: `packages/ui/src/hooks/useSelection.ts`
- Modify: `TaskboardApp.tsx`

**Interfaces:**
- `useBoardData(transport)` → projects/tasks/inbox/status/occupancy, `reloadBoard`, `applyProject`, `refreshTasks`, `refreshInbox`, `addProject`, …
- `useSelection({ transport, refreshTasks, … })` → `selectedId`, `detail`, `applyDetail`, `selectCard`, `closeInspector`, `mutateSelected`

```ts
async function mutateSelected(
  fn: (id: string) => Promise<void>,
  opts?: { refreshBoard?: boolean },
): Promise<void> {
  const id = selectedIdRef.current;
  if (!id) return;
  try {
    await fn(id);
    applyDetail(await transport.taskShow(id));
    if (opts?.refreshBoard) {
      const project = selectedProjectRef.current;
      if (project) await refreshTasks(project.slug);
    }
  } catch (err) {
    setToast({ message: errorMessage(err), error: true });
  }
}
```

Wire Inspector happy-path callbacks (`onSpawn`, `onComment*`, `onCheck*`, `onReview`, `onRunCancel`, `onBlockedByAdd` where applicable) through `mutateSelected`. Leave revision-conflict `catch { }` blocks unchanged.

- [ ] Shell JSX only wires hook outputs to Sidebar/Board/Inspector/…
- [ ] Full UI test suite PASS
- [ ] Commit + PR

## Self-review

1. Spec: four hooks + mutateSelected + useLatestRef + untouched conflict catches — covered.
2. No placeholders.
3. Hook names match issue acceptance.
