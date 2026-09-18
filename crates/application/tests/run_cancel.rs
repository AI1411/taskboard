#![allow(unused_imports)]
mod common;
use common::{cli_actor, test_app, TestApp};

use taskboard_application::{
    Actor, App, ProjectAdd, RunCancel, RunFail, RunFinish, RunStart, RunWait, SystemClock,
    TaskCreate,
};
use taskboard_core::{ActorKind, CardDisplayStatus, Column, RunStatus};
use taskboard_store_sqlite::{open_db, SqliteStore};

async fn seeded_running() -> TestApp {
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
            title: "Stuck".into(),
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
    app
}

#[tokio::test]
async fn cancel_running_ends_as_failed_with_default_summary() {
    let app = seeded_running().await;
    let run = app
        .run_cancel(
            &cli_actor(),
            RunCancel {
                run_display_id: "RUN-1".into(),
                summary: None,
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.summary.as_deref(), Some("canceled"));
    assert!(run.ended_at.is_some());
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.column, Column::Todo);
    assert_eq!(shown.display_status, CardDisplayStatus::Failed);
}

#[tokio::test]
async fn cancel_waiting_ends_as_failed() {
    let app = seeded_running().await;
    app.run_wait(
        &cli_actor(),
        RunWait {
            run_display_id: "RUN-1".into(),
            reason: "Need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let run = app
        .run_cancel(
            &cli_actor(),
            RunCancel {
                run_display_id: "RUN-1".into(),
                summary: Some("agent died".into()),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.summary.as_deref(), Some("agent died"));
}

#[tokio::test]
async fn cancel_completed_or_failed_is_validation_error() {
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
    let err = app
        .run_cancel(
            &cli_actor(),
            RunCancel {
                run_display_id: "RUN-1".into(),
                summary: None,
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");

    let app = seeded_running().await;
    app.run_fail(
        &cli_actor(),
        RunFail {
            run_display_id: "RUN-1".into(),
            summary: "boom".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let err = app
        .run_cancel(
            &cli_actor(),
            RunCancel {
                run_display_id: "RUN-1".into(),
                summary: None,
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}
