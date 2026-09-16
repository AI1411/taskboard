# Inspector Link Add/Remove Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the inspector add and remove task links through existing `linkAdd` / `linkRemove`.

**Architecture:** Inspector Links section gains a text field and Add link. Values containing `://` are URLs; otherwise paths. Each row has Remove. `TaskboardApp` applies the returned `TaskDetail`.

**Tech Stack:** React, Vitest, Testing Library. Spec §7. Issue: https://github.com/AI1411/taskboard/issues/56. Existing plan Task 7 (links only).

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Inspector section order: Title, id, column, urgent, note, links, runs, activity, Delete
- UI copy is English
- Placeholder is exactly `https:// or /path`
- Toasts for validation errors are out of scope unless already present (issue #59)
- Trash panel, project menu, markdown preview, start-run UI are out of scope

## File map

- Create: `packages/ui/src/Inspector.test.tsx`
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/Inspector.module.css`
- Modify: `packages/ui/src/TaskboardApp.tsx`

---

### Task 1: Inspector link add and remove

**Files:**
- Create: `packages/ui/src/Inspector.test.tsx`
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/Inspector.module.css`
- Modify: `packages/ui/src/TaskboardApp.tsx`

**Interfaces:**
- Consumes: `transport.linkAdd(displayId, { kind, value })`, `transport.linkRemove(linkId)`
- Produces:
  - Inspector props `onLinkAdd: (value: string) => void`, `onLinkRemove: (linkId: string) => void`
  - Kind is `"url"` when `value.includes("://")`, else `"path"`
  - Add does nothing when trimmed value is empty
  - Remove button `aria-label={`Remove ${link.value}`}`

- [x] **Step 1: Write the failing tests**

```tsx
it("adds a URL and a path then removes the URL", async () => {
  const transport = fakeTransport();
  const project = await transport.projectAdd({ name: "Alpha" });
  await transport.taskCreate(project.slug, { title: "Linked", column: "todo" });
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(await screen.findByText("Linked"));
  await userEvent.type(screen.getByPlaceholderText("https:// or /path"), "https://example.com");
  await userEvent.click(screen.getByRole("button", { name: "Add link" }));
  await waitFor(() =>
    expect(transport.linkAdd).toHaveBeenCalledWith("TASK-1", {
      kind: "url",
      value: "https://example.com",
    }),
  );
  expect(await screen.findByText("https://example.com")).toBeTruthy();
  await userEvent.type(screen.getByPlaceholderText("https:// or /path"), "/tmp/notes.md");
  await userEvent.click(screen.getByRole("button", { name: "Add link" }));
  await waitFor(() =>
    expect(transport.linkAdd).toHaveBeenCalledWith("TASK-1", {
      kind: "path",
      value: "/tmp/notes.md",
    }),
  );
  await userEvent.click(screen.getByRole("button", { name: "Remove https://example.com" }));
  await waitFor(() => expect(transport.linkRemove).toHaveBeenCalledWith("l1"));
  expect(screen.queryByText("https://example.com")).toBeNull();
});
```

- [x] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test`

Expected: FAIL missing placeholder / Add link.

- [x] **Step 3: Implement**

Inspector local `linkValue` state. Add link button calls `onLinkAdd(trimmed)` and clears the field. Rows: `{link.value}` plus Remove. TaskboardApp:

```tsx
const onLinkAdd = async (value: string) => {
  const id = selectedIdRef.current;
  if (!id) return;
  const kind = value.includes("://") ? "url" : "path";
  applyDetail(await transport.linkAdd(id, { kind, value }));
};
const onLinkRemove = async (linkId: string) => {
  applyDetail(await transport.linkRemove(linkId));
};
```

- [x] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add packages/ui docs/superpowers/plans/2026-09-16-issue-56-link-add-remove.md
git commit -m "feat: add and remove inspector links"
```

---

## Self-review

1. **Spec coverage:** URL add, path add, remove, section order, placeholder (§7). Toasts left to #59.
2. **Placeholder scan:** No TBD.
3. **Type consistency:** `linkAdd` kind is `"url" | "path"`.
