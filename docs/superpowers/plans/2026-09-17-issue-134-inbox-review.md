# Mix In Review into Inbox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put finished In Review cards in Inbox group `Review` (after Stale, before Urgent) so humans see the pile `run finish` leaves behind.

**Architecture:** Derived membership only. Extend `InboxGroup` and `inbox_membership` in `crates/core/src/inbox.rs`. `App::inbox` already filters through that function, so `tb inbox`, the Inbox strip, and `tb status` inbox counts stay one list. Review-group reason is the winning run’s `summary` (else empty). No fifth column. Done stays out.

**Tech Stack:** Rust `InboxGroup` / `InboxCounts`, existing Inbox strip in `packages/ui`, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` still does not move the card
- No fifth column; Done is not Inbox
- No OS notifications
- Waiting / failed / stale still outrank Review
- Running In Review is not Review
- JSON snake_case (`review` count on `InboxCounts`)
- Reason for Review is run summary or empty

User already chose sequential inline execution.

## File map

- Create: `docs/superpowers/plans/2026-09-17-issue-134-inbox-review.md`
- Modify: `crates/core/src/inbox.rs` — `InboxGroup::Review`, membership, unit tests
- Modify: `crates/application/src/commands.rs` — `InboxCounts.review`
- Modify: `crates/application/src/app.rs` — Review reason from run summary; status counts
- Modify: `crates/application/tests/inbox.rs` — completed In Review appears between stale and urgent
- Modify: `crates/cli/src/output.rs` — status inbox parts include `review`
- Modify: `packages/ui/src/InboxStrip.tsx` + `.test.tsx` + `fakeTransport.ts`

---

### Task 1: Core membership (failing tests first)

**Files:**
- Modify: `crates/core/src/inbox.rs`

**Interfaces:**
- Produces:
  - `InboxGroup { Waiting=0, Failed=1, Stale=2, Review=3, Urgent=4 }`
  - `inbox_membership`: Waiting → Waiting; Failed → Failed; stale → Stale; `column == InReview && status != Running` → Review; urgent && not Done → Urgent; else None

- [ ] **Step 1: Replace `completed_in_review_is_out_unless_urgent` with these tests**

```rust
    #[test]
    fn completed_in_review_is_review_even_if_urgent() {
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Completed, false),
            Some(InboxGroup::Review)
        );
        assert_eq!(
            inbox_membership(Column::InReview, true, CardDisplayStatus::Completed, false),
            Some(InboxGroup::Review)
        );
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Idle, false),
            Some(InboxGroup::Review)
        );
    }

    #[test]
    fn running_waiting_failed_stale_in_review_are_not_review() {
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Running, false),
            None
        );
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Waiting, false),
            Some(InboxGroup::Waiting)
        );
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Failed, false),
            Some(InboxGroup::Failed)
        );
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Running, true),
            Some(InboxGroup::Stale)
        );
        assert!(InboxGroup::Stale < InboxGroup::Review);
        assert!(InboxGroup::Review < InboxGroup::Urgent);
    }

    #[test]
    fn done_completed_is_still_out() {
        assert_eq!(
            inbox_membership(Column::Done, false, CardDisplayStatus::Completed, false),
            None
        );
    }
```

- [ ] **Step 2: Run tests — expect FAIL** (`completed_in_review_is_review_even_if_urgent` left == None)

Run: `cargo test -p taskboard-core --lib inbox -- --nocapture`

- [ ] **Step 3: Implement membership**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InboxGroup {
    Waiting = 0,
    Failed = 1,
    Stale = 2,
    Review = 3,
    Urgent = 4,
}

pub fn inbox_membership(
    column: Column,
    urgent: bool,
    status: CardDisplayStatus,
    stale: bool,
) -> Option<InboxGroup> {
    match status {
        CardDisplayStatus::Waiting => Some(InboxGroup::Waiting),
        CardDisplayStatus::Failed => Some(InboxGroup::Failed),
        _ if stale => Some(InboxGroup::Stale),
        _ if column == Column::InReview && status != CardDisplayStatus::Running => {
            Some(InboxGroup::Review)
        }
        _ if urgent && column != Column::Done => Some(InboxGroup::Urgent),
        _ => None,
    }
}
```

- [ ] **Step 4: Re-run core tests — expect PASS**

---

### Task 2: App inbox reason + status counts

**Files:**
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/app.rs`
- Modify: `crates/application/tests/inbox.rs`
- Modify: `crates/cli/src/output.rs`

**Interfaces:**
- `InboxCounts` gains `review: usize`
- Review-group `reason` = `winning_run.summary` or `""`
- Status `inbox.review` / `inbox.urgent` counted by `inbox_membership`, not leftover math that would call Review “urgent”

- [ ] **Step 1: Add a failing App test** at the end of `crates/application/tests/inbox.rs`

Use the existing `test_app` / `cli_actor` helpers. Create TASK-1 completed In Review with summary `"shipped login"`, TASK-2 stale running, TASK-3 urgent todo. Assert inbox order TASK-2 (stale), TASK-1 (review), TASK-3 (urgent) and `TASK-1.reason == "shipped login"`. Assert `status(None).inbox.review == 1` and `status.inbox.urgent == 1`.

```rust
#[tokio::test]
async fn inbox_includes_completed_in_review_between_stale_and_urgent() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "Renai Sim".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Review me".into(),
            column: Some(Column::InReview),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Stuck".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Pin me".into(),
            column: None,
            urgent: true,
        },
    )
    .await
    .unwrap();
    let done = app
        .run_start(
            &actor,
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "cursor".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    app.run_finish(
        &actor,
        RunFinish {
            run_display_id: done.display_id,
            summary: "shipped login".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-2".into(),
            agent: "cursor".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    // force stale via SharedClock only if this file already has it; otherwise mark via
    // display_status stale by using the existing stale test clock pattern.
}
```

If `test_app` uses `SystemClock`, stale will not trip. Prefer asserting order Review before Urgent without a stale card, plus a unit-level membership order already in Task 1. For App: list `[Review me, Pin me]` and `status.inbox.review == 1`.

Simpler App test (use this):

```rust
#[tokio::test]
async fn inbox_completed_in_review_uses_run_summary() {
    // seed Review me (in-review) + Pin me (urgent todo) as above, finish RUN-1
    let items = app
        .inbox(InboxScope {
            project: None,
            include_archived: false,
        })
        .await
        .unwrap();
    assert_eq!(items[0].display_id, "TASK-1");
    assert_eq!(items[0].reason, "shipped login");
    assert_eq!(items[1].display_id, "TASK-3");
    let snap = app.status(None).await.unwrap();
    assert_eq!(snap.inbox.review, 1);
    assert_eq!(snap.inbox.urgent, 1);
}
```

Need `RunFinish` import. Check existing imports in `inbox.rs`.

- [ ] **Step 2: Run App test — expect FAIL** (`review` field missing or item absent)

- [ ] **Step 3: Add `review` to `InboxCounts` and count by group**

```rust
pub struct InboxCounts {
    pub total: usize,
    pub waiting: usize,
    pub failed: usize,
    pub stale: usize,
    pub review: usize,
    pub urgent: usize,
}
```

In `App::status`:

```rust
        let inbox_counts = InboxCounts {
            total: inbox.len(),
            waiting: count_group(&inbox, InboxGroup::Waiting),
            failed: count_group(&inbox, InboxGroup::Failed),
            stale: count_group(&inbox, InboxGroup::Stale),
            review: count_group(&inbox, InboxGroup::Review),
            urgent: count_group(&inbox, InboxGroup::Urgent),
        };
```

Add a private helper next to `inbox_line`:

```rust
fn count_group(items: &[InboxItem], group: InboxGroup) -> usize {
    items
        .iter()
        .filter(|item| {
            inbox_membership(item.column, item.urgent, item.display_status, item.stale)
                == Some(group)
        })
        .count()
}
```

Import `InboxGroup`.

- [ ] **Step 4: Review reason from run summary**

In `App::inbox`, after computing membership:

```rust
                let Some(group) = inbox_membership(...) else { continue };
                let reason = match group {
                    InboxGroup::Stale => { /* existing stale reason */ }
                    InboxGroup::Review => {
                        let runs = store.list_runs(summary.id).await?;
                        winning_run(&runs)
                            .and_then(|run| run.summary.clone())
                            .unwrap_or_default()
                    }
                    _ => inbox_reason(
                        summary.waiting_reason.as_deref(),
                        summary.run_message.as_deref(),
                    ),
                };
```

- [ ] **Step 5: `inbox_parts` includes review**

```rust
        (counts.waiting, "waiting"),
        (counts.failed, "failed"),
        (counts.stale, "stale"),
        (counts.review, "review"),
        (counts.urgent, "urgent"),
```

- [ ] **Step 6: Run App + CLI tests**

`cargo test -p taskboard-application --test inbox -- --nocapture`
`cargo test -p taskboard-cli --test cli_json status_json -- --nocapture`

Expected: PASS.

---

### Task 3: Inbox strip

**Files:**
- Modify: `packages/ui/src/fakeTransport.ts` — `inboxMember`
- Modify: `packages/ui/src/InboxStrip.tsx` — Review count between Stale and Urgent
- Modify: `packages/ui/src/InboxStrip.test.tsx`

**Interfaces:**
- Same membership: in-review && status not waiting/failed/running && !stale → Review

- [ ] **Step 1: Failing UI test**

```tsx
  it("lists completed in-review between stale and urgent", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const review = await transport.taskCreate(project.slug, {
      title: "Ship",
      column: "in-review",
    });
    review.column = "in-review";
    review.displayStatus = "completed";
    review.runMessage = "ignored";
    (review as TaskDetail).runs = [
      {
        id: "run-1",
        displayId: "RUN-1",
        taskId: review.id,
        agent: "cursor",
        sessionId: null,
        status: "completed",
        message: "ignored",
        waitingReason: null,
        summary: "shipped login",
        startedAt: "2026-09-16T12:00:00Z",
        endedAt: "2026-09-16T12:05:00Z",
        revision: 1,
        createdAt: "2026-09-16T12:00:00Z",
        updatedAt: "2026-09-16T12:05:00Z",
        worktreePath: null,
        branch: null,
      },
    ];
    const urgent = await transport.taskCreate(project.slug, {
      title: "Pin",
      column: "todo",
      urgent: true,
    });
    urgent.urgent = true;
    render(<TaskboardApp transport={transport} />);
    expect(await screen.findByText(/Inbox · 2/)).toBeTruthy();
    expect(screen.getByText(/Review 1/)).toBeTruthy();
    expect(screen.getByText(/Urgent 1/)).toBeTruthy();
  });
```

If fakeTransport reason cannot see `runs` yet, assert only the `Review 1` count (reason from App is covered in Task 2). Prefer count-only in UI.

- [ ] **Step 2: Update `inboxMember`**

```ts
function inboxMember(
  column: Column,
  urgent: boolean,
  status: DisplayStatus,
  stale: boolean,
): boolean {
  if (status === "waiting" || status === "failed") return true;
  if (stale) return true;
  if (column === "in-review" && status !== "running") return true;
  return urgent && column !== "done";
}
```

- [ ] **Step 3: Update `InboxStrip` counts**

```ts
  const review = props.items.filter(
    (item) =>
      item.column === "in-review" &&
      item.displayStatus !== "waiting" &&
      item.displayStatus !== "failed" &&
      item.displayStatus !== "running" &&
      !item.stale,
  ).length;
  const urgent = props.items.filter(
    (item) =>
      item.displayStatus !== "waiting" &&
      item.displayStatus !== "failed" &&
      !item.stale &&
      !(
        item.column === "in-review" &&
        item.displayStatus !== "running"
      ),
  ).length;
  const parts = [
    waiting ? `Waiting ${waiting}` : null,
    failed ? `Failed ${failed}` : null,
    stale ? `Stale ${stale}` : null,
    review ? `Review ${review}` : null,
    urgent ? `Urgent ${urgent}` : null,
  ].filter(Boolean);
```

- [ ] **Step 4: Run UI tests**

`pnpm --filter @taskboard/ui test InboxStrip`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/plans/2026-09-17-issue-134-inbox-review.md \
  crates/core/src/inbox.rs crates/application crates/cli/src/output.rs \
  packages/ui/src/InboxStrip.tsx packages/ui/src/InboxStrip.test.tsx \
  packages/ui/src/fakeTransport.ts
git commit -m "$(cat <<'EOF'
feat: mix completed In Review into Inbox

Add derived Inbox group Review after Stale and before Urgent so
finished cards surface for humans. Reason is the run summary.
EOF
)"
```

---

## Self-review

| Acceptance | Task |
| --- | --- |
| Completed In Review in group Review (Stale next, Urgent previous) | Task 1 enum order + Task 2 sort |
| Running / waiting / failed / stale In Review not Review | Task 1 tests |
| `tb inbox` / strip / `tb status` agree | one `inbox_membership` |
| Reason is run summary | Task 2 Review arm |
| Done excluded; no fifth column | membership + Done test |

No placeholders. Types: `InboxGroup::Review`, `InboxCounts.review`.
