use std::ops::Deref;

use taskboard_application::{
    Actor, App, AppError, LinkAdd, ProjectAdd, RunFail, RunFinish, RunStart, RunUpdate, RunWait,
    Store, SystemClock, TaskCreate,
};
use taskboard_core::{ActorKind, CardDisplayStatus, Column, LinkKind, Run, RunStatus, Task};
use taskboard_store_sqlite::{open_db, SqliteStore};
use uuid::Uuid;

struct TestApp {
    app: App,
    _tmp: tempfile::TempDir,
}

impl TestApp {
    async fn task_row(&self, display_id: &str) -> Task {
        let pool = open_db(self._tmp.path()).await.unwrap();
        let mut store = SqliteStore::new(pool, self._tmp.path());
        store
            .get_task_by_display_id(display_id, false)
            .await
            .unwrap()
            .unwrap()
    }

    async fn latest_activity_for(&self, entity_id: Uuid) -> taskboard_core::Activity {
        let pool = open_db(self._tmp.path()).await.unwrap();
        let mut store = SqliteStore::new(pool, self._tmp.path());
        store.latest_activity_for(entity_id).await.unwrap().unwrap()
    }

    async fn get_run(&self, display_id: &str) -> Run {
        let pool = open_db(self._tmp.path()).await.unwrap();
        let mut store = SqliteStore::new(pool, self._tmp.path());
        store
            .get_run_by_display_id(display_id)
            .await
            .unwrap()
            .unwrap()
    }

    async fn get_link(&self, id: Uuid) -> Option<taskboard_core::Link> {
        let pool = open_db(self._tmp.path()).await.unwrap();
        let mut store = SqliteStore::new(pool, self._tmp.path());
        store.get_link(id).await.unwrap()
    }

    async fn soft_delete_task(&self, display_id: &str) {
        let pool = open_db(self._tmp.path()).await.unwrap();
        let mut store = SqliteStore::new(pool, self._tmp.path());
        let task = store
            .get_task_by_display_id(display_id, false)
            .await
            .unwrap()
            .unwrap();
        store
            .soft_delete_task(task.id, chrono::Utc::now())
            .await
            .unwrap();
    }
}

impl Deref for TestApp {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

fn cli_actor() -> Actor {
    Actor {
        kind: ActorKind::Cli,
        label: "local-cli".into(),
    }
}

async fn test_app() -> TestApp {
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let store = SqliteStore::new(pool, tmp.path());
    TestApp {
        app: App::new(store, SystemClock),
        _tmp: tmp,
    }
}

async fn seeded() -> TestApp {
    let app = test_app().await;
    app.project_add(
        &cli_actor(),
        ProjectAdd {
            name: "Renai Sim".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app
}

async fn seeded_task() -> TestApp {
    let app = seeded().await;
    app.task_create(
        &cli_actor(),
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Fix login error".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    app
}

async fn seeded_running() -> TestApp {
    let app = seeded_task().await;
    app.run_start(
        &cli_actor(),
        RunStart {
            task_display_id: "TASK-1".into(),
            agent: "codex".into(),
            session_id: Some("abc123".into()),
        },
    )
    .await
    .unwrap();
    app
}

#[tokio::test]
async fn note_add_appends_paragraph() {
    let app = seeded_task().await;
    app.task_note_set(&cli_actor(), "TASK-1", "First".into(), None)
        .await
        .unwrap();
    let t = app
        .task_note_add(&cli_actor(), "TASK-1", "Second", None)
        .await
        .unwrap();
    assert_eq!(t.note_markdown, "First\n\nSecond");
}

#[tokio::test]
async fn run_start_returns_run_1_running() {
    let app = seeded_task().await;
    let r = app
        .run_start(
            &cli_actor(),
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: Some("abc123".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(r.display_id, "RUN-1");
    assert_eq!(r.status, RunStatus::Running);
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.display_status, CardDisplayStatus::Running);
    assert_eq!(shown.column, Column::Todo);
}

#[tokio::test]
async fn finish_does_not_move_column() {
    let app = seeded_running().await;
    app.run_finish(
        &cli_actor(),
        RunFinish {
            run_display_id: "RUN-1".into(),
            summary: "done".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.column, Column::Todo);
    assert_eq!(shown.display_status, CardDisplayStatus::Completed);
}

#[tokio::test]
async fn wait_requires_reason() {
    let app = seeded_running().await;
    let err = app
        .run_wait(
            &cli_actor(),
            RunWait {
                run_display_id: "RUN-1".into(),
                reason: " ".into(),
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}

#[tokio::test]
async fn note_add_on_empty_note_is_just_the_paragraph() {
    let app = seeded_task().await;
    let t = app
        .task_note_add(&cli_actor(), "TASK-1", "Only paragraph", None)
        .await
        .unwrap();
    assert_eq!(t.note_markdown, "Only paragraph");
}

#[tokio::test]
async fn note_add_empty_paragraph_is_validation_error() {
    let app = seeded_task().await;
    let err = app
        .task_note_add(&cli_actor(), "TASK-1", "   ", None)
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}

#[tokio::test]
async fn note_set_keeps_malformed_markdown() {
    let app = seeded_task().await;
    let markdown = "[unclosed](http://example.com".to_string();
    let t = app
        .task_note_set(&cli_actor(), "TASK-1", markdown.clone(), None)
        .await
        .unwrap();
    assert_eq!(t.note_markdown, markdown);
    assert_eq!(t.revision, 2);
    assert_eq!(t.recent_activities[0].operation, "task.note.set");
}

#[tokio::test]
async fn note_add_records_task_note_set_activity() {
    let app = seeded_task().await;
    let t = app
        .task_note_add(&cli_actor(), "TASK-1", "Check sessions", None)
        .await
        .unwrap();
    assert_eq!(t.recent_activities[0].operation, "task.note.set");
}

#[tokio::test]
async fn link_add_url_and_path_sort_order_per_task() {
    let app = seeded_task().await;
    let first = app
        .link_add(
            &cli_actor(),
            LinkAdd {
                task_display_id: "TASK-1".into(),
                kind: LinkKind::Url,
                value: "https://example.com".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(first.links.len(), 1);
    assert_eq!(first.links[0].kind, LinkKind::Url);
    assert_eq!(first.links[0].value, "https://example.com");
    assert_eq!(first.links[0].sort_order, 0);
    assert_eq!(
        app.latest_activity_for(first.links[0].id).await.operation,
        "link.add"
    );

    let second = app
        .link_add(
            &cli_actor(),
            LinkAdd {
                task_display_id: "TASK-1".into(),
                kind: LinkKind::Path,
                value: "/tmp/does-not-need-to-exist.log".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(second.links.len(), 2);
    assert_eq!(second.links[0].sort_order, 0);
    assert_eq!(second.links[1].kind, LinkKind::Path);
    assert_eq!(second.links[1].value, "/tmp/does-not-need-to-exist.log");
    assert_eq!(second.links[1].sort_order, 1);
}

#[tokio::test]
async fn link_add_invalid_url_is_validation_error() {
    let app = seeded_task().await;
    let err = app
        .link_add(
            &cli_actor(),
            LinkAdd {
                task_display_id: "TASK-1".into(),
                kind: LinkKind::Url,
                value: "ftp://x".into(),
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
    match err {
        AppError::Validation { field, .. } => assert_eq!(field, "value"),
        other => panic!("expected validation_error, got {other:?}"),
    }
}

#[tokio::test]
async fn link_remove_deletes_by_id() {
    let app = seeded_task().await;
    let added = app
        .link_add(
            &cli_actor(),
            LinkAdd {
                task_display_id: "TASK-1".into(),
                kind: LinkKind::Url,
                value: "https://example.com".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    let link_id = added.links[0].id;
    let removed = app.link_remove(&cli_actor(), link_id, None).await.unwrap();
    assert!(removed.links.is_empty());
    assert_eq!(
        app.latest_activity_for(link_id).await.operation,
        "link.remove"
    );
    assert!(app.get_link(link_id).await.is_none());
}

#[tokio::test]
async fn link_remove_missing_is_not_found() {
    let app = seeded_task().await;
    let err = app
        .link_remove(&cli_actor(), Uuid::nil(), None)
        .await
        .unwrap_err();
    assert_eq!(err.code(), "not_found");
}

#[tokio::test]
async fn run_start_on_deleted_task_is_not_found() {
    let app = seeded_task().await;
    app.soft_delete_task("TASK-1").await;
    let err = app
        .run_start(
            &cli_actor(),
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "not_found");
}

#[tokio::test]
async fn run_start_invalid_agent_is_validation_error() {
    let app = seeded_task().await;
    let err = app
        .run_start(
            &cli_actor(),
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "Claude".into(),
                session_id: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
    match err {
        AppError::Validation { field, .. } => assert_eq!(field, "agent"),
        other => panic!("expected validation_error, got {other:?}"),
    }
}

#[tokio::test]
async fn concurrent_runs_are_allowed() {
    let app = seeded_task().await;
    let first = app
        .run_start(
            &cli_actor(),
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: Some("a".into()),
            },
        )
        .await
        .unwrap();
    let second = app
        .run_start(
            &cli_actor(),
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: Some("b".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(first.display_id, "RUN-1");
    assert_eq!(second.display_id, "RUN-2");
    assert_eq!(first.status, RunStatus::Running);
    assert_eq!(second.status, RunStatus::Running);
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.runs.len(), 2);
    assert_eq!(shown.display_status, CardDisplayStatus::Running);
    assert_eq!(shown.column, Column::Todo);
}

#[tokio::test]
async fn run_update_sets_message_and_rejects_over_500() {
    let app = seeded_running().await;
    let updated = app
        .run_update(
            &cli_actor(),
            RunUpdate {
                run_display_id: "RUN-1".into(),
                message: Some("compiling".into()),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.message.as_deref(), Some("compiling"));
    assert_eq!(
        app.latest_activity_for(updated.id).await.operation,
        "run.update"
    );

    let err = app
        .run_update(
            &cli_actor(),
            RunUpdate {
                run_display_id: "RUN-1".into(),
                message: Some("x".repeat(501)),
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}

#[tokio::test]
async fn run_wait_sets_waiting_reason() {
    let app = seeded_running().await;
    let waiting = app
        .run_wait(
            &cli_actor(),
            RunWait {
                run_display_id: "RUN-1".into(),
                reason: "need review".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(waiting.status, RunStatus::Waiting);
    assert_eq!(waiting.waiting_reason.as_deref(), Some("need review"));
    assert!(waiting.ended_at.is_none());
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.display_status, CardDisplayStatus::Waiting);
    assert_eq!(shown.column, Column::Todo);
    assert_eq!(
        app.latest_activity_for(waiting.id).await.operation,
        "run.wait"
    );
}

#[tokio::test]
async fn run_fail_sets_ended_at_and_does_not_move_column() {
    let app = seeded_running().await;
    let failed = app
        .run_fail(
            &cli_actor(),
            RunFail {
                run_display_id: "RUN-1".into(),
                summary: "boom".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(failed.status, RunStatus::Failed);
    assert_eq!(failed.summary.as_deref(), Some("boom"));
    assert!(failed.ended_at.is_some());
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.column, Column::Todo);
    assert_eq!(shown.display_status, CardDisplayStatus::Failed);
    assert_eq!(app.task_row("TASK-1").await.column, Column::Todo);
    assert_eq!(
        app.latest_activity_for(failed.id).await.operation,
        "run.fail"
    );
}

#[tokio::test]
async fn run_fail_and_finish_require_summary() {
    let app = seeded_running().await;
    let fail_err = app
        .run_fail(
            &cli_actor(),
            RunFail {
                run_display_id: "RUN-1".into(),
                summary: "  ".into(),
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(fail_err.code(), "validation_error");

    let finish_err = app
        .run_finish(
            &cli_actor(),
            RunFinish {
                run_display_id: "RUN-1".into(),
                summary: "".into(),
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(finish_err.code(), "validation_error");
}

#[tokio::test]
async fn finish_records_run_finish_and_ended_at() {
    let app = seeded_running().await;
    let finished = app
        .run_finish(
            &cli_actor(),
            RunFinish {
                run_display_id: "RUN-1".into(),
                summary: "done".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(finished.status, RunStatus::Completed);
    assert!(finished.ended_at.is_some());
    let stored = app.get_run("RUN-1").await;
    assert!(stored.ended_at.is_some());
    assert_eq!(stored.status, RunStatus::Completed);
    assert_eq!(
        app.latest_activity_for(finished.id).await.operation,
        "run.finish"
    );
    assert_eq!(
        app.latest_activity_for(finished.id).await.entity_type,
        taskboard_core::EntityType::Run
    );
}

#[tokio::test]
async fn run_message_matches_display_status_run() {
    let app = seeded_task().await;
    let older = app
        .run_start(
            &cli_actor(),
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    app.run_update(
        &cli_actor(),
        RunUpdate {
            run_display_id: older.display_id.clone(),
            message: Some("please review".into()),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.run_wait(
        &cli_actor(),
        RunWait {
            run_display_id: "RUN-1".into(),
            reason: "review".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let newer = app
        .run_start(
            &cli_actor(),
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    app.run_update(
        &cli_actor(),
        RunUpdate {
            run_display_id: newer.display_id.clone(),
            message: Some("finished work".into()),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.run_finish(
        &cli_actor(),
        RunFinish {
            run_display_id: "RUN-2".into(),
            summary: "done".into(),
            revision: None,
        },
    )
    .await
    .unwrap();

    let listed = app.task_list("renai-sim").await.unwrap();
    assert_eq!(listed[0].display_status, CardDisplayStatus::Waiting);
    assert_eq!(listed[0].run_message.as_deref(), Some("please review"));
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.display_status, CardDisplayStatus::Waiting);
    assert_eq!(shown.run_message.as_deref(), Some("please review"));
}

#[tokio::test]
async fn run_start_records_run_start_activity() {
    let app = seeded_task().await;
    let r = app
        .run_start(
            &cli_actor(),
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: Some("abc123".into()),
            },
        )
        .await
        .unwrap();
    let activity = app.latest_activity_for(r.id).await;
    assert_eq!(activity.operation, "run.start");
    assert_eq!(r.agent, "codex");
    assert_eq!(r.session_id.as_deref(), Some("abc123"));
    assert_eq!(r.revision, 1);
}
