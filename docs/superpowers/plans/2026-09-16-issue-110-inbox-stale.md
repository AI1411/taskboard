# Mix Stale into Inbox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put cards whose winning run is stale into Inbox group `Stale`, after Waiting / Failed and before Urgent, with the same members in `tb inbox` and the Inbox strip.

**Architecture:** No new table. Reuse `TaskSummary.stale` (winning run, 30-minute heartbeat from #82). Add `InboxGroup::Stale` and pass `stale` into `inbox_membership`. Inbox reason for those cards is `stale · last update {RFC3339}` from the winning run’s `updated_at`. Add `InboxItem.stale` so the strip can count Stale separately from Urgent. No OS notification, no auto-fail.

**Tech Stack:** Existing Rust crates, clap JSON (`--json` snake_case), React Inbox strip, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- No new table
- No OS notification
- No automatic `run fail` / `run cancel`
- Waiting and Failed still outrank Stale; Stale outranks Urgent
- Completed In Review stays out of Inbox unless urgent
- CLI `--json` stays snake_case; HTTP / desktop DTOs stay camelCase
- Default stale threshold stays 30 minutes

## File map

- Modify: `crates/core/src/inbox.rs` — `InboxGroup::Stale`, membership + reason
- Modify: `crates/core/src/models.rs` — `InboxItem.stale`
- Modify: `crates/application/src/app.rs` — inbox membership, reason, sort
- Modify: `crates/application/tests/inbox.rs` — stale member + group order
- Modify: `crates/api/src/dto.rs`, `crates/desktop-commands/src/dto.rs` — `stale` on InboxItemDto
- Modify: `packages/types/src/index.ts`
- Modify: `packages/ui/src/fakeTransport.ts`, `InboxStrip.tsx`, `InboxStrip.test.tsx`

User already chose sequential inline execution.

---

### Task 1: InboxGroup::Stale and App::inbox membership

**Files:**
- Modify: `crates/core/src/inbox.rs`
- Modify: `crates/core/src/models.rs`
- Modify: `crates/application/src/app.rs`
- Modify: `crates/application/tests/inbox.rs`
- Modify: `crates/api/src/dto.rs`
- Modify: `crates/desktop-commands/src/dto.rs`

**Interfaces:**
- Consumes: `TaskSummary.stale`, `is_stale` / winning run `updated_at`, existing `inbox_reason`
- Produces:
  - `enum InboxGroup { Waiting = 0, Failed = 1, Stale = 2, Urgent = 3 }`
  - `inbox_membership(column, urgent, status, stale: bool) -> Option<InboxGroup>`
  - Waiting / Failed ignore `stale` (those statuses are never stale today)
  - `_ if stale => Some(Stale)` before urgent
  - `inbox_stale_reason(updated_at: DateTime<Utc>) -> String` = `stale · last update {rfc3339 secs Z}`
  - `InboxItem.stale: bool`
  - `App::inbox` includes stale running cards; sort uses membership with `item.stale`
  - HTTP / desktop InboxItemDto gain `stale`

- [ ] **Step 1: Write the failing tests**

In `crates/core/src/inbox.rs` tests, update every `inbox_membership(...)` call to pass a fourth `false` argument, then add:

```rust
    #[test]
    fn stale_running_is_inbox_between_failed_and_urgent() {
        assert_eq!(
            inbox_membership(Column::InProgress, false, CardDisplayStatus::Running, true),
            Some(InboxGroup::Stale)
        );
        assert_eq!(
            inbox_membership(Column::InProgress, true, CardDisplayStatus::Running, true),
            Some(InboxGroup::Stale)
        );
        assert!(InboxGroup::Failed < InboxGroup::Stale);
        assert!(InboxGroup::Stale < InboxGroup::Urgent);
    }

    #[test]
    fn stale_reason_includes_last_update() {
        let at = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 9, 16, 12, 0, 0).unwrap();
        assert_eq!(
            inbox_stale_reason(at),
            "stale · last update 2026-09-16T12:00:00Z"
        );
    }
```

Existing `running_is_out_unless_urgent` stays, now called with `stale = false`.

In `crates/application/tests/inbox.rs`, add a `SharedClock` (copy the type from `stale.rs`) and:

```rust
#[tokio::test]
async fn inbox_includes_stale_between_failed_and_urgent() {
    let start = Utc.with_ymd_and_hms(2026, 9, 16, 12, 0, 0).unwrap();
    let clock = SharedClock(Arc::new(Mutex::new(start)));
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let store = SqliteStore::new(pool, tmp.path());
    let app = App::new(store, clock.clone());
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
            title: "Wait me".into(),
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
            title: "Fail me".into(),
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
    let wait = app
        .run_start(
            &actor,
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    app.run_wait(
        &actor,
        RunWait {
            run_display_id: wait.display_id,
            reason: "Need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let fail = app
        .run_start(
            &actor,
            RunStart {
                task_display_id: "TASK-2".into(),
                agent: "codex".into(),
                session_id: None,
            },
        )
        .await
    .unwrap();
    app.run_fail(
        &actor,
        RunFail {
            run_display_id: fail.display_id,
            summary: "boom".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-3".into(),
            agent: "cursor".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    *clock.0.lock().unwrap() += Duration::minutes(31);
    let items = app
        .inbox(InboxScope {
            project: None,
            include_archived: false,
        })
        .await
        .unwrap();
    let titles: Vec<_> = items.iter().map(|i| i.title.as_str()).collect();
    assert_eq!(titles, vec!["Wait me", "Fail me", "Stuck", "Pin me"]);
    let stuck = items.iter().find(|i| i.title == "Stuck").unwrap();
    assert!(stuck.stale);
    assert_eq!(stuck.reason, "stale · last update 2026-09-16T12:00:00Z");
}
```

Add the same `SharedClock` + `Clock` impl + imports (`Arc`, `Mutex`, `Duration`, `TimeZone`, `Utc`, `Clock`) used in `stale.rs`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-core stale_running_is_inbox -- --nocapture`

Expected: FAIL — `inbox_membership` takes 3 args / `InboxGroup::Stale` missing.

Run: `cargo test -p taskboard-application --test inbox inbox_includes_stale -- --nocapture`

Expected: FAIL — compile or Stuck not in inbox.

- [ ] **Step 3: Write minimal implementation**

`crates/core/src/inbox.rs`:

```rust
use chrono::{DateTime, SecondsFormat, Utc};

use crate::column::Column;
use crate::display_status::CardDisplayStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InboxGroup {
    Waiting = 0,
    Failed = 1,
    Stale = 2,
    Urgent = 3,
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
        _ if urgent && column != Column::Done => Some(InboxGroup::Urgent),
        _ => None,
    }
}

pub fn inbox_reason(waiting_reason: Option<&str>, run_message: Option<&str>) -> String {
    let waiting = waiting_reason.map(str::trim).filter(|s| !s.is_empty());
    let message = run_message.map(str::trim).filter(|s| !s.is_empty());
    waiting.or(message).unwrap_or("").to_string()
}

pub fn inbox_stale_reason(updated_at: DateTime<Utc>) -> String {
    format!(
        "stale · last update {}",
        updated_at.to_rfc3339_opts(SecondsFormat::Secs, true)
    )
}
```

Export `inbox_stale_reason` from `crates/core/src/lib.rs`.

Add `pub stale: bool` to `InboxItem` in `models.rs` (after `reason` or before `updated_at`).

In `App::inbox`, import `inbox_stale_reason`. Change the loop:

```rust
                let Some(_group) = inbox_membership(
                    summary.column,
                    summary.urgent,
                    summary.display_status,
                    summary.stale,
                ) else {
                    continue;
                };
                let reason = if summary.stale {
                    let runs = store.list_runs(summary.id).await?;
                    let last = winning_run(&runs)
                        .map(|run| run.updated_at)
                        .unwrap_or(updated_at);
                    inbox_stale_reason(last)
                } else {
                    inbox_reason(
                        summary.waiting_reason.as_deref(),
                        summary.run_message.as_deref(),
                    )
                };
                items.push(InboxItem {
                    // existing fields...
                    reason,
                    stale: summary.stale,
                    updated_at,
                });
```

Sort:

```rust
            let left_group = inbox_membership(
                left.column,
                left.urgent,
                left.display_status,
                left.stale,
            );
            let right_group = inbox_membership(
                right.column,
                right.urgent,
                right.display_status,
                right.stale,
            );
```

Add `stale` to both InboxItemDto structs and `From` impls.

Update every existing `inbox_membership(...)` test call to pass `false`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-core inbox -- --nocapture`

Run: `cargo test -p taskboard-application --test inbox -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/inbox.rs crates/core/src/lib.rs crates/core/src/models.rs crates/application/src/app.rs crates/application/tests/inbox.rs crates/api/src/dto.rs crates/desktop-commands/src/dto.rs
git commit -m "feat: include stale running cards in inbox"
```

---

### Task 2: Inbox strip Stale group

**Files:**
- Modify: `packages/types/src/index.ts`
- Modify: `packages/ui/src/fakeTransport.ts`
- Modify: `packages/ui/src/InboxStrip.tsx`
- Modify: `packages/ui/src/InboxStrip.test.tsx`

**Interfaces:**
- Consumes: `InboxItem.stale`, `item.reason`
- Produces:
  - Header counts: `Waiting N · Failed N · Stale N · Urgent N` (omit zeros)
  - Stale is after Failed, before Urgent
  - Row still shows `item.reason` (includes `stale · last update …`)
  - `fakeTransport.inbox` includes `stale === true` members with that reason

- [ ] **Step 1: Write the failing strip test**

In `packages/ui/src/InboxStrip.test.tsx`:

```tsx
  it("lists stale running cards between failed and urgent", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const stale = await transport.taskCreate(project.slug, { title: "Stuck", column: "in-progress" });
    stale.displayStatus = "running";
    stale.stale = true;
    const urgent = await transport.taskCreate(project.slug, { title: "Pin", column: "todo", urgent: true });
    urgent.urgent = true;
    render(<TaskboardApp transport={transport} />);
    expect(await screen.findByText(/Inbox · 2/)).toBeTruthy();
    expect(screen.getByText(/Stale 1/)).toBeTruthy();
    expect(screen.getByText(/Urgent 1/)).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: /Inbox/ }));
    expect(await screen.findByRole("button", { name: /Stuck/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /stale · last update/ })).toBeTruthy();
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @taskboard/ui exec vitest run src/InboxStrip.test.tsx`

Expected: FAIL — Inbox hidden or no `Stale 1` (fakeTransport ignores `stale`).

- [ ] **Step 3: Write minimal implementation**

`packages/types/src/index.ts` InboxItem:

```ts
  reason: string;
  stale: boolean;
  updatedAt: string;
```

`packages/ui/src/fakeTransport.ts`:

```ts
function inboxMember(column: Column, urgent: boolean, status: DisplayStatus, stale: boolean): boolean {
  if (status === "waiting" || status === "failed") return true;
  if (stale) return true;
  return urgent && column !== "done";
}
```

In `inbox()`, pass `detail?.stale ?? task.stale` into `inboxMember`. Push:

```ts
          reason: stale
            ? "stale · last update 2026-09-16T12:00:00Z"
            : (waitingReason && waitingReason.trim()) || (runMessage && runMessage.trim()) || "",
          stale,
          updatedAt: project.updatedAt,
```

`InboxStrip.tsx` counts:

```ts
  const waiting = props.items.filter((item) => item.displayStatus === "waiting").length;
  const failed = props.items.filter((item) => item.displayStatus === "failed").length;
  const stale = props.items.filter((item) => item.stale && item.displayStatus !== "waiting" && item.displayStatus !== "failed").length;
  const urgent = props.items.filter(
    (item) =>
      item.displayStatus !== "waiting" &&
      item.displayStatus !== "failed" &&
      !item.stale,
  ).length;
  const parts = [
    waiting ? `Waiting ${waiting}` : null,
    failed ? `Failed ${failed}` : null,
    stale ? `Stale ${stale}` : null,
    urgent ? `Urgent ${urgent}` : null,
  ].filter(Boolean);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui exec vitest run src/InboxStrip.test.tsx`

Expected: PASS

Then: `pnpm --filter @taskboard/ui test`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/types/src/index.ts packages/ui/src/fakeTransport.ts packages/ui/src/InboxStrip.tsx packages/ui/src/InboxStrip.test.tsx
git commit -m "feat(ui): show Stale group in inbox strip"
```

---

## Self-review

**1. Spec coverage**
- Stale inbox group after Waiting/Failed, before Urgent → Task 1 enum + sort, Task 2 counts
- `tb inbox` same members → Task 1 `App::inbox` (CLI already prints `App::inbox`)
- Reason `stale · last update …` → Task 1 `inbox_stale_reason`
- No OS notification / auto-fail → no new code
- JSON snake_case `--json` → InboxItem serde unchanged except new `stale` field

**2. Placeholder scan:** none

**3. Type consistency:** `InboxItem.stale: bool` on core, both DTOs, and TS. `inbox_membership(..., stale)` used in membership, sort, and tests.
