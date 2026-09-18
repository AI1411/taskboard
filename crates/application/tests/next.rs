#![allow(unused_imports)]
mod common;
use common::{cli_actor, test_app, TestApp};

use taskboard_application::{Actor, App, NextClaim, ProjectAdd, RunStart, SystemClock, TaskCreate};
use taskboard_core::{ActorKind, Column, RunStatus};
use taskboard_store_sqlite::{open_db, SqliteStore};

async fn seeded_two_ready() -> TestApp {
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
            title: "First ready".into(),
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
            title: "Second ready".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    app
}

#[tokio::test]
async fn exclusive_start_conflicts_when_open_run_exists() {
    let app = seeded_two_ready().await;
    let actor = cli_actor();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-1".into(),
            agent: "codex".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    let err = app
        .run_start_exclusive(
            &actor,
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "cursor".into(),
                session_id: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "conflict");
}

#[tokio::test]
async fn flagless_start_still_allows_parallel_runs() {
    let app = seeded_two_ready().await;
    let actor = cli_actor();
    app.run_start(
        &actor,
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
            &actor,
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: Some("b".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(second.display_id, "RUN-2");
    assert_eq!(second.status, RunStatus::Running);
}

#[tokio::test]
async fn next_claims_first_ready_without_moving() {
    let app = seeded_two_ready().await;
    let run = app
        .next(
            &cli_actor(),
            NextClaim {
                project: Some("renai-sim".into()),
                agent: "cursor".into(),
                session_id: None,
                move_to_in_progress: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(run.display_id, "RUN-1");
    assert_eq!(run.agent, "cursor");
    assert_eq!(run.status, RunStatus::Running);
    let first = app.task_show("TASK-1").await.unwrap();
    assert_eq!(first.column, Column::Todo);
    assert_eq!(first.runs[0].display_id, "RUN-1");
}

#[tokio::test]
async fn next_skips_occupied_ready_and_optional_move() {
    let app = seeded_two_ready().await;
    let actor = cli_actor();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-1".into(),
            agent: "codex".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    let run = app
        .next(
            &actor,
            NextClaim {
                project: None,
                agent: "cursor".into(),
                session_id: None,
                move_to_in_progress: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(run.task_id, app.task_show("TASK-2").await.unwrap().id);
    let second = app.task_show("TASK-2").await.unwrap();
    assert_eq!(second.column, Column::InProgress);
}

#[tokio::test]
async fn next_with_no_claimable_ready_is_not_found() {
    let app = test_app().await;
    let err = app
        .next(
            &cli_actor(),
            NextClaim {
                project: None,
                agent: "cursor".into(),
                session_id: None,
                move_to_in_progress: false,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "not_found");
}
