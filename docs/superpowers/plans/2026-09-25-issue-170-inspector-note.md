# Inspector note drafts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A clean inspector draft follows server notes, and a dirty draft does not autosave over a newer note.

**Architecture:** `fieldDraft.ts` tracks `value`, `base`, `dirty`, and `conflict` for title, note, worktree, and branch. Autosave and blur commit only when `shouldCommit` is true. A conflict shows "Updated elsewhere" with Keep mine / Load server. `recoverRevisionConflict` replaces the three empty-catch refresh sites in `useSelection.ts`.

**Tech Stack:** React, vitest, `@testing-library/react`.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` does not move the card
- No markdown preview
- No silent retry after a conflict
- Keep the 400ms note autosave for a dirty draft whose server base has not changed

User already chose sequential inline execution.

## File map

- Create: `packages/ui/src/fieldDraft.ts`
- Create: `packages/ui/src/fieldDraft.test.ts`
- Create: `packages/ui/src/hooks/revisionConflict.ts`
- Create: `packages/ui/src/hooks/revisionConflict.test.ts`
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/Inspector.module.css`
- Modify: `packages/ui/src/Inspector.test.tsx`
- Modify: `packages/ui/src/hooks/useSelection.ts`

---

### Task 1: Draft policy

**Interfaces:**
- Produces: `freshDraft`, `followServer`, `editDraft`, `shouldCommit`, `keepMine`

- [ ] **Step 1: Write `fieldDraft.test.ts`** covering a clean follow, a dirty conflict, a server echo of the draft, and keep-mine.

- [ ] **Step 2: Run** `pnpm --filter @taskboard/ui exec vitest run src/fieldDraft.test.ts` and watch it fail to import.

- [ ] **Step 3: Implement `fieldDraft.ts`** so a clean draft copies the server, a dirty draft whose server value changed sets `conflict`, and `server === value` becomes a fresh draft (the save echo).

- [ ] **Step 4: Re-run the test.** Expected: PASS.

### Task 2: Inspector

- [ ] **Step 1: Add the two tests in `Inspector.test.tsx`** (`keeps an agent note...`, `stops autosave...`).

- [ ] **Step 2: Run** `pnpm --filter @taskboard/ui exec vitest run src/Inspector.test.tsx -t "Inspector note drafts"`. Expected: the clean draft stays `""` and the conflict banner is missing.

- [ ] **Step 3: Store title, note, worktree, and branch as `FieldDraft`.** On `displayId` change, reset. Otherwise `followServer`. Autosave and blur call the handlers only when `shouldCommit`. Render the status banner when any field conflicts.

- [ ] **Step 4: Re-run Inspector and Board tests.** Expected: PASS, including the existing 400ms note saves.

### Task 3: Conflict refresh helper

- [ ] **Step 1: Test `recoverRevisionConflict`** toasts when `taskShow` throws, and ignores non-conflict errors.

- [ ] **Step 2: Implement the helper and call it from the three `useSelection` catch sites.** A refresh failure toasts `errorMessage`.

- [ ] **Step 3: Run** `pnpm --filter @taskboard/ui test`. Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git commit -m "fix(ui): keep a stale inspector draft from overwriting a newer note"
```

## Self-review

- Clean server note is not autosaved: Task 2.
- Dirty note shows keep / load and does not autosave: Task 2.
- Title, worktree, and branch use the same draft policy.
- Three conflict refreshes share one helper; failures toast.
