use std::ops::Deref;

use taskboard_application::{
    Actor, App, ProjectAdd, RunContinue, RunFail, RunStart, RunWait, SystemClock, TaskCreate,
};
use taskboard_core::{ActorKind, CardDisplayStatus, RunStatus};
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

async fn seeded_waiting() -> TestApp {
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
            title: "Fix login".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-1".into(),
            agent: "cursor".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    app.run_wait(
        &actor,
        RunWait {
            run_display_id: "RUN-1".into(),
            reason: "need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app
}

#[tokio::test]
async fn continue_moves_waiting_to_running_and_clears_reason() {
    let app = seeded_waiting().await;
    let continued = app
        .run_continue(
            &cli_actor(),
            RunContinue {
                run_display_id: "RUN-1".into(),
                message: Some("back to it".into()),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(continued.display_id, "RUN-1");
    assert_eq!(continued.status, RunStatus::Running);
    assert_eq!(continued.waiting_reason, None);
    assert!(continued.ended_at.is_none());
    assert_eq!(continued.message.as_deref(), Some("back to it"));
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.display_status, CardDisplayStatus::Running);
    assert_eq!(shown.column, taskboard_core::Column::Todo);
}

#[tokio::test]
async fn continue_without_message_keeps_existing_message() {
    let app = seeded_waiting().await;
    app.run_update(
        &cli_actor(),
        taskboard_application::RunUpdate {
            run_display_id: "RUN-1".into(),
            message: Some("paused".into()),
            revision: None,
        },
    )
    .await
    .unwrap();
    let continued = app
        .run_continue(
            &cli_actor(),
            RunContinue {
                run_display_id: "RUN-1".into(),
                message: None,
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(continued.message.as_deref(), Some("paused"));
}

#[tokio::test]
async fn continue_running_or_failed_is_validation_error() {
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
            title: "Fix login".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-1".into(),
            agent: "cursor".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    let running_err = app
        .run_continue(
            &actor,
            RunContinue {
                run_display_id: "RUN-1".into(),
                message: None,
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(running_err.code(), "validation_error");
    app.run_fail(
        &actor,
        RunFail {
            run_display_id: "RUN-1".into(),
            summary: "boom".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let failed_err = app
        .run_continue(
            &actor,
            RunContinue {
                run_display_id: "RUN-1".into(),
                message: None,
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(failed_err.code(), "validation_error");
    let restart = app
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
    assert_eq!(restart.display_id, "RUN-2");
}
