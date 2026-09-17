# Write worktree / branch from the board Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a person set or clear `worktreePath` and `branch` from HTTP, desktop `task_update`, and the inspector so occupancy has a board-side write path.

**Architecture:** Reuse `App::task_update` / `TaskUpdate.worktree_path` / `branch` (`None` omit, `Some(None)` clear, `Some(Some)` set). HTTP `PatchTaskBody` and desktop `TaskPatchArgs` grow the two optional strings; empty string maps through the same `empty_to_none` rule as CLI. Inspector replaces the read-only path line with two labeled inputs that call `transport.taskUpdate`. No git. Occupancy stays #118. No card-face “shared” badge.

**Tech Stack:** Axum `PATCH /api/v1/tasks/{id}`, desktop-commands `TaskPatchArgs`, `packages/types` `TaskPatch`, React Inspector, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` still does not move the card
- Empty string clears, same as CLI `task update --worktree ""`
- HTTP camelCase (`worktreePath`, `branch`); CLI JSON stays snake_case
- No git subprocess
- Occupancy calculation is unchanged from #118
- No “shared” badge on the card face
- Do not change CLI `task update` semantics
- No command palette, no `tb work start`

User already chose sequential inline execution.

## File map

- Create: `docs/superpowers/plans/2026-09-17-issue-135-board-worktree.md`
- Modify: `crates/api/src/dto.rs` — `PatchTaskBody.worktree_path` / `branch`
- Modify: `crates/api/src/routes.rs` — `patch_task` forwards both to `TaskUpdate`
- Modify: `crates/api/tests/routes.rs` — set and clear via camelCase JSON
- Modify: `crates/desktop-commands/src/commands.rs` — `TaskPatchArgs` + `task_update_inner`
- Modify: `apps/desktop/src-tauri/src/commands.rs` — tauri `task_update` args
- Modify: `crates/desktop-commands/tests/commands.rs` — set and clear
- Modify: `packages/types/src/index.ts` — `TaskPatch.worktreePath` / `branch`
- Modify: `packages/ui/src/fakeTransport.ts` — apply / clear workspace fields
- Modify: `packages/ui/src/Inspector.tsx` + `TaskboardApp.tsx` — editable fields
- Modify: `packages/ui/src/Inspector.test.tsx` + `Card.test.tsx`
- Modify: `packages/client/src/http.test.ts` + `tauri.test.ts` — body / args include keys

---

### Task 1: HTTP PATCH accepts worktreePath / branch

**Files:**
- Modify: `crates/api/tests/routes.rs`
- Modify: `crates/api/src/dto.rs`
- Modify: `crates/api/src/routes.rs`

**Interfaces:**
- Consumes: `App::task_update` / `TaskUpdate { worktree_path: Option<Option<String>>, branch: Option<Option<String>>, .. }`
- Produces: `PatchTaskBody { worktree_path: Option<String>, branch: Option<String>, .. }` camelCase; absent omits; `""` clears

- [ ] **Step 1: Write the failing HTTP tests**

Append to `crates/api/tests/routes.rs` after `patch_task_chains_if_match_across_fields`:

```rust
#[tokio::test]
async fn patch_task_sets_and_clears_worktree_and_branch() {
    let s = seeded_task_server().await;
    let client = authed(&s);

    let set = client
        .patch(format!("{}/api/v1/tasks/TASK-1", s.base))
        .json(&json!({ "worktreePath": "/tmp/wt", "branch": "cursor/foo-88ba" }))
        .send()
        .await
        .unwrap();
    assert_eq!(set.status(), 200);
    let body = set.json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["entity"]["worktreePath"], "/tmp/wt");
    assert_eq!(body["entity"]["branch"], "cursor/foo-88ba");

    let shown = client
        .get(format!("{}/api/v1/tasks/TASK-1", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(shown["entity"]["worktreePath"], "/tmp/wt");
    assert_eq!(shown["entity"]["branch"], "cursor/foo-88ba");

    let cleared = client
        .patch(format!("{}/api/v1/tasks/TASK-1", s.base))
        .json(&json!({ "worktreePath": "", "branch": "" }))
        .send()
        .await
        .unwrap();
    assert_eq!(cleared.status(), 200);
    let body = cleared.json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["entity"]["worktreePath"], serde_json::Value::Null);
    assert_eq!(body["entity"]["branch"], serde_json::Value::Null);
}

#[tokio::test]
async fn patch_task_omits_workspace_when_fields_absent() {
    let s = seeded_task_server().await;
    let client = authed(&s);
    client
        .patch(format!("{}/api/v1/tasks/TASK-1", s.base))
        .json(&json!({ "worktreePath": "/tmp/keep", "branch": "keep-branch" }))
        .send()
        .await
        .unwrap();

    let title_only = client
        .patch(format!("{}/api/v1/tasks/TASK-1", s.base))
        .json(&json!({ "title": "Still assigned" }))
        .send()
        .await
        .unwrap();
    assert_eq!(title_only.status(), 200);
    let body = title_only.json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["entity"]["title"], "Still assigned");
    assert_eq!(body["entity"]["worktreePath"], "/tmp/keep");
    assert_eq!(body["entity"]["branch"], "keep-branch");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-api --test routes patch_task_sets_and_clears_worktree_and_branch patch_task_omits_workspace_when_fields_absent -- --nocapture`

Expected: FAIL — JSON unknown field ignored or values stay null because `PatchTaskBody` has no `worktree_path` / `branch`.

- [ ] **Step 3: Add fields and forward them**

In `crates/api/src/dto.rs` `PatchTaskBody`:

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchTaskBody {
    pub title: Option<String>,
    pub note_markdown: Option<String>,
    pub urgent: Option<bool>,
    pub column: Option<Column>,
    /// Absent: do not reorder. `null`: move to end. String: place before that card.
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub before_display_id: Option<Option<String>>,
    pub worktree_path: Option<String>,
    pub branch: Option<String>,
}
```

In `crates/api/src/dto.rs` (same file, next to `deserialize_present_option`):

```rust
fn empty_to_none(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
```

In `crates/api/src/routes.rs` `patch_task`, after the title block and before note (or after `before_display_id`, either is fine as long as `revision` advances):

```rust
    if body.worktree_path.is_some() || body.branch.is_some() {
        let updated = state
            .app
            .task_update(
                &actor,
                TaskUpdate {
                    display_id: display_id.clone(),
                    worktree_path: body.worktree_path.map(empty_to_none),
                    branch: body.branch.map(empty_to_none),
                    revision,
                    ..Default::default()
                },
            )
            .await
            .map_err(app_error)?;
        revision = advance_if_match(updated.revision);
        task = Some(updated);
    }
```

Import `empty_to_none` from `crate::dto` (make it `pub(crate)`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-api --test routes patch_task_sets_and_clears_worktree_and_branch patch_task_omits_workspace_when_fields_absent`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/dto.rs crates/api/src/routes.rs crates/api/tests/routes.rs docs/superpowers/plans/2026-09-17-issue-135-board-worktree.md
git commit -m "feat(api): PATCH tasks accepts worktreePath and branch"
```

---

### Task 2: Desktop task_update accepts the same fields

**Files:**
- Modify: `crates/desktop-commands/tests/commands.rs`
- Modify: `crates/desktop-commands/src/commands.rs`
- Modify: `apps/desktop/src-tauri/src/commands.rs`

**Interfaces:**
- Consumes: `TaskUpdate` from Task 1’s domain (already exists)
- Produces: `TaskPatchArgs { worktree_path: Option<String>, branch: Option<String>, .. }`; empty string clears

- [ ] **Step 1: Write the failing desktop test**

Add to `crates/desktop-commands/tests/commands.rs`:

```rust
use taskboard_desktop_commands::{
    check_add_inner, check_toggle_inner, comment_add_inner, link_add_inner, project_add_inner,
    sync_inner, task_create_inner, task_update_inner, AppErrorDto, TaskPatchArgs,
};

#[tokio::test]
async fn task_update_sets_and_clears_worktree_and_branch() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    task_create_inner(&app, "renai-sim".into(), "Fix login".into(), None, None)
        .await
        .unwrap();

    let set = task_update_inner(
        &app,
        TaskPatchArgs {
            display_id: "TASK-1".into(),
            title: None,
            note_markdown: None,
            urgent: None,
            column: None,
            before_display_id: None,
            worktree_path: Some("/tmp/wt".into()),
            branch: Some("cursor/foo-88ba".into()),
            revision: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(set.worktree_path.as_deref(), Some("/tmp/wt"));
    assert_eq!(set.branch.as_deref(), Some("cursor/foo-88ba"));

    let cleared = task_update_inner(
        &app,
        TaskPatchArgs {
            display_id: "TASK-1".into(),
            title: None,
            note_markdown: None,
            urgent: None,
            column: None,
            before_display_id: None,
            worktree_path: Some("".into()),
            branch: Some("".into()),
            revision: Some(set.revision),
        },
    )
    .await
    .unwrap();
    assert_eq!(cleared.worktree_path, None);
    assert_eq!(cleared.branch, None);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-desktop-commands --test commands task_update_sets_and_clears_worktree_and_branch`

Expected: FAIL compile — `TaskPatchArgs` has no `worktree_path` / `branch`.

- [ ] **Step 3: Add fields and call `task_update`**

In `crates/desktop-commands/src/commands.rs`:

```rust
pub struct TaskPatchArgs {
    pub display_id: String,
    pub title: Option<String>,
    pub note_markdown: Option<String>,
    pub urgent: Option<bool>,
    pub column: Option<Column>,
    pub before_display_id: Option<Option<String>>,
    pub worktree_path: Option<String>,
    pub branch: Option<String>,
    pub revision: Option<i64>,
}
```

Inside `task_update_inner`, after the title block:

```rust
    if worktree_path.is_some() || branch.is_some() {
        let updated = app
            .task_update(
                &actor,
                TaskUpdate {
                    display_id: display_id.clone(),
                    worktree_path: worktree_path.map(empty_to_none),
                    branch: branch.map(empty_to_none),
                    revision,
                    ..Default::default()
                },
            )
            .await?;
        revision = Some(updated.revision);
        task = Some(updated);
    }
```

Add at the bottom of the same file (or next to `missing`):

```rust
fn empty_to_none(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
```

Destructure the new fields from `TaskPatchArgs`.

In `apps/desktop/src-tauri/src/commands.rs` `task_update`:

```rust
pub async fn task_update(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
    title: Option<String>,
    note_markdown: Option<String>,
    urgent: Option<bool>,
    column: Option<Column>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    before_display_id: Option<Option<String>>,
    worktree_path: Option<String>,
    branch: Option<String>,
    revision: Option<i64>,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    task_update_inner(
        &app,
        TaskPatchArgs {
            display_id,
            title,
            note_markdown,
            urgent,
            column,
            before_display_id,
            worktree_path,
            branch,
            revision,
        },
    )
    .await
    .map(Into::into)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p taskboard-desktop-commands --test commands task_update_sets_and_clears_worktree_and_branch`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/desktop-commands/src/commands.rs crates/desktop-commands/tests/commands.rs apps/desktop/src-tauri/src/commands.rs
git commit -m "feat(desktop): task_update writes worktree_path and branch"
```

---

### Task 3: Types, fake transport, inspector write fields

**Files:**
- Modify: `packages/types/src/index.ts`
- Modify: `packages/ui/src/fakeTransport.ts`
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Modify: `packages/ui/src/Inspector.test.tsx`
- Modify: `packages/client/src/http.test.ts`
- Modify: `packages/client/src/tauri.test.ts`

**Interfaces:**
- Consumes: `transport.taskUpdate(displayId, TaskPatch, revision)`
- Produces:
  - `TaskPatch { worktreePath?: string; branch?: string; .. }`
  - `Inspector` props `onWorkspaceChange: (patch: { worktreePath?: string; branch?: string }) => void`

- [ ] **Step 1: Write the failing inspector + client tests**

Replace `Inspector workspace` in `packages/ui/src/Inspector.test.tsx`:

```tsx
describe("Inspector workspace", () => {
  it("shows recorded worktree and branch in editable fields", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Wt", column: "todo" });
    task.worktreePath = "/tmp/wt";
    task.branch = "cursor/foo-88ba";
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Wt"));
    expect((await screen.findByLabelText("Worktree") as HTMLInputElement).value).toBe("/tmp/wt");
    expect((screen.getByLabelText("Branch") as HTMLInputElement).value).toBe("cursor/foo-88ba");
  });

  it("writes worktree and branch through taskUpdate and clears with empty string", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Wt", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Wt"));
    const worktree = await screen.findByLabelText("Worktree");
    await userEvent.clear(worktree);
    await userEvent.type(worktree, "/tmp/wt");
    await userEvent.tab();
    await waitFor(() =>
      expect(transport.taskUpdate).toHaveBeenCalledWith(
        "TASK-1",
        { worktreePath: "/tmp/wt" },
        expect.anything(),
      ),
    );
    const branch = screen.getByLabelText("Branch");
    await userEvent.clear(branch);
    await userEvent.type(branch, "cursor/foo-88ba");
    await userEvent.tab();
    await waitFor(() =>
      expect(transport.taskUpdate).toHaveBeenCalledWith(
        "TASK-1",
        { branch: "cursor/foo-88ba" },
        expect.anything(),
      ),
    );
    await userEvent.clear(worktree);
    await userEvent.tab();
    await waitFor(() =>
      expect(transport.taskUpdate).toHaveBeenCalledWith(
        "TASK-1",
        { worktreePath: "" },
        expect.anything(),
      ),
    );
    expect((worktree as HTMLInputElement).value).toBe("");
  });
});
```

Add to `packages/client/src/http.test.ts`:

```ts
  it("sends worktreePath and branch on taskUpdate", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(
        JSON.stringify({
          ok: true,
          entity: { displayId: "TASK-1", worktreePath: "/tmp/wt", branch: "cursor/foo-88ba" },
          revision: 2,
        }),
        { headers: { "Content-Type": "application/json" } },
      );
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    await t.taskUpdate("TASK-1", { worktreePath: "/tmp/wt", branch: "cursor/foo-88ba" }, 1);
    const body = (await fetches[0].json()) as Record<string, unknown>;
    assert.equal(body.worktreePath, "/tmp/wt");
    assert.equal(body.branch, "cursor/foo-88ba");
    assert.equal("worktree_path" in body, false);
  });
```

In `packages/client/src/tauri.test.ts`, change the existing `taskUpdate` call to include workspace keys and assert args:

```ts
    await t.taskUpdate(
      "TASK-1",
      { title: "U", noteMarkdown: "md", worktreePath: "/tmp/wt", branch: "cursor/foo-88ba" },
      1,
    );
```

and update `calls[11].args` to:

```ts
    assert.deepEqual(calls[11].args, {
      display_id: "TASK-1",
      title: "U",
      note_markdown: "md",
      worktree_path: "/tmp/wt",
      branch: "cursor/foo-88ba",
      revision: 1,
    });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test -- src/Inspector.test.tsx`

Expected: FAIL — no `Worktree` / `Branch` labels.

- [ ] **Step 3: Minimal types + UI**

`packages/types/src/index.ts`:

```ts
export interface TaskPatch {
  title?: string;
  noteMarkdown?: string;
  urgent?: boolean;
  column?: Column;
  beforeDisplayId?: string | null;
  worktreePath?: string;
  branch?: string;
}
```

`packages/ui/src/fakeTransport.ts` `taskUpdate`:

```ts
      if (patch.worktreePath !== undefined) {
        task.worktreePath = patch.worktreePath.trim() === "" ? null : patch.worktreePath;
      }
      if (patch.branch !== undefined) {
        task.branch = patch.branch.trim() === "" ? null : patch.branch;
      }
```

`Inspector` props add:

```ts
  onWorkspaceChange?: (patch: { worktreePath?: string; branch?: string }) => void;
```

State:

```ts
  const [worktree, setWorktree] = useState(props.task?.worktreePath ?? "");
  const [branch, setBranch] = useState(props.task?.branch ?? "");
```

Reset both to `""` when `!props.task`. When `lastId` changes, set from `props.task.worktreePath ?? ""` and `props.task.branch ?? ""`.

Replace the read-only paragraph with:

```tsx
        <label className={styles.field}>
          Worktree
          <input
            value={worktree}
            onChange={(e) => setWorktree(e.target.value)}
            onBlur={() => props.onWorkspaceChange?.({ worktreePath: worktree })}
          />
        </label>
        <label className={styles.field}>
          Branch
          <input
            value={branch}
            onChange={(e) => setBranch(e.target.value)}
            onBlur={() => props.onWorkspaceChange?.({ branch })}
          />
        </label>
```

In `TaskboardApp.tsx` next to `onTitleCommit`:

```ts
  const onWorkspaceChange = useCallback(
    async (patch: { worktreePath?: string; branch?: string }) => {
      const id = selectedIdRef.current;
      if (!id) return;
      try {
        const updated = await transport.taskUpdate(id, patch, detailRef.current?.revision);
        applyDetail(updated);
        const project = selectedProjectRef.current;
        if (project) await refreshTasks(project.slug);
      } catch (err) {
        if (errorCode(err) === "revision_conflict") {
          setToast({ message: "Updated elsewhere", error: true });
          try {
            applyDetail(await transport.taskShow(id));
          } catch {
            /* keep current detail */
          }
        } else {
          setToast({ message: errorMessage(err), error: true });
        }
      }
    },
    [transport, refreshTasks],
  );
```

Pass `onWorkspaceChange={(patch) => void onWorkspaceChange(patch)}` to `Inspector`.

Do not call git. Do not add occupancy UI.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```
pnpm --filter @taskboard/ui test -- src/Inspector.test.tsx
pnpm --filter @taskboard/client test
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/types/src/index.ts packages/ui/src/fakeTransport.ts packages/ui/src/Inspector.tsx packages/ui/src/TaskboardApp.tsx packages/ui/src/Inspector.test.tsx packages/client/src/http.test.ts packages/client/src/tauri.test.ts
git commit -m "feat(ui): inspector writes worktree and branch"
```

---

### Task 4: Card face stays path/branch only (no shared badge)

**Files:**
- Modify: `packages/ui/src/Card.test.tsx`
- Test: occupancy tests are not edited

**Interfaces:**
- Consumes: existing `Card` path/branch spans
- Produces: assertion that “shared” is absent

- [ ] **Step 1: Write the failing extra assertion**

In `packages/ui/src/Card.test.tsx` `shows recorded worktree and branch`:

```tsx
  it("shows recorded worktree and branch", () => {
    const { getByText, queryByText } = render(
      <Card
        task={summary({ worktreePath: "/tmp/wt", branch: "cursor/foo-88ba" })}
        selected={false}
      />,
    );
    expect(getByText("/tmp/wt")).toBeTruthy();
    expect(getByText("cursor/foo-88ba")).toBeTruthy();
    expect(queryByText(/shared/i)).toBeNull();
  });
```

- [ ] **Step 2: Run the test**

Run: `pnpm --filter @taskboard/ui test -- src/Card.test.tsx`

Expected: PASS without `Card.tsx` changes (no shared badge exists). If it fails because copy contains “shared”, remove that copy — do not add a badge.

- [ ] **Step 3: Confirm occupancy tests still pass unchanged**

Run: `cargo test -p taskboard-application --test occupancy`

Expected: PASS, files under `crates/application/tests/occupancy.rs` unmodified.

- [ ] **Step 4: Commit if the Card test changed**

```bash
git add packages/ui/src/Card.test.tsx
git commit -m "test(ui): card face has no shared badge"
```

---

## Self-review

1. Spec coverage: HTTP PATCH set/clear; desktop `task_update`; `TaskPatch` keys; inspector edit; occupancy unchanged; no shared badge; no git.
2. Placeholder scan: none.
3. Types: `worktreePath` / `branch` camelCase on HTTP and TS; snake_case only on Rust structs and Tauri IPC.
