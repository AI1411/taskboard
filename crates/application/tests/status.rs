use std::ops::Deref;

use taskboard_application::{
    Actor, App, LinkAdd, ProjectAdd, RunStart, RunWait, SystemClock, TaskCreate, TaskListQuery,
};
use taskboard_core::{ActorKind, Column, LinkKind};
use taskboard_store_sqlite::{open_db, SqliteStore};

struct TestApp {
    app: App,
    _tmp: tempfile::TempDir,
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

#[tokio::test]
async fn status_counts_inbox_open_ready_review_blocked() {
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
            title: "Ready me".into(),
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
            title: "Blocked me".into(),
            column: None,
            urgent: false,
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
    app.link_add(
        &actor,
        LinkAdd {
            task_display_id: "TASK-4".into(),
            kind: LinkKind::BlockedBy,
            value: "TASK-2".into(),
            revision: None,
        },
    )
    .await
    .unwrap();

    let ready = app
        .task_query(TaskListQuery {
            ready: true,
            ..TaskListQuery::default()
        })
        .await
        .unwrap();
    let snap = app.status(None).await.unwrap();
    assert_eq!(snap.inbox.total, 1);
    assert_eq!(snap.inbox.waiting, 1);
    assert_eq!(snap.open_runs, 1);
    assert_eq!(snap.ready, ready.len());
    assert_eq!(snap.in_review, 1);
    assert_eq!(snap.blocked, 1);
    assert_eq!(snap.inbox_head[0].display_id, "TASK-1");
    assert_eq!(snap.inbox_head[0].detail, "Need spec");
    assert_eq!(snap.open_run_head[0].display_id, "TASK-1");
    assert_eq!(snap.open_run_head[0].agent.as_deref(), Some("codex"));
    assert_eq!(
        snap.ready_head
            .iter()
            .map(|line| line.display_id.as_str())
            .collect::<Vec<_>>(),
        ready
            .iter()
            .take(3)
            .map(|task| task.display_id.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(snap.in_review_head[0].display_id, "TASK-3");
    assert_eq!(snap.blocked_head[0].display_id, "TASK-4");
}

#[tokio::test]
async fn status_unknown_project_is_not_found() {
    let app = test_app().await;
    let err = app.status(Some("nope".into())).await.unwrap_err();
    assert_eq!(err.code(), "not_found");
}
