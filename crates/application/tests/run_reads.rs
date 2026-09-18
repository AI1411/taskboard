#![allow(unused_imports)]
mod common;
use common::{cli_actor, test_app, TestApp};

use taskboard_application::{
    Actor, App, ProjectAdd, RunListQuery, RunStart, RunWait, SystemClock, TaskCreate,
};
use taskboard_core::{ActorKind, RunStatus};
use taskboard_store_sqlite::{open_db, SqliteStore};

async fn seeded_two_runs() -> TestApp {
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
            title: "One".into(),
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
            title: "Two".into(),
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
            session_id: Some("sess-a".into()),
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-2".into(),
            agent: "codex".into(),
            session_id: Some("sess-b".into()),
        },
    )
    .await
    .unwrap();
    app.run_wait(
        &actor,
        RunWait {
            run_display_id: "RUN-2".into(),
            reason: "need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app
}

#[tokio::test]
async fn run_list_open_returns_running_and_waiting() {
    let app = seeded_two_runs().await;
    let listed = app
        .run_list(RunListQuery {
            open: true,
            session_id: None,
            agent: None,
        })
        .await
        .unwrap();
    let ids: Vec<_> = listed.iter().map(|run| run.display_id.as_str()).collect();
    assert!(ids.contains(&"RUN-1"));
    assert!(ids.contains(&"RUN-2"));
    assert!(listed
        .iter()
        .all(|run| matches!(run.status, RunStatus::Running | RunStatus::Waiting)));
}

#[tokio::test]
async fn run_show_and_session_lookup() {
    let app = seeded_two_runs().await;
    let shown = app.run_show("RUN-1").await.unwrap();
    assert_eq!(shown.agent, "cursor");
    let by_session = app.run_by_session("sess-b").await.unwrap();
    assert_eq!(by_session.display_id, "RUN-2");
    let err = app.run_by_session("missing").await.unwrap_err();
    assert_eq!(err.code(), "not_found");
}

#[tokio::test]
async fn run_current_returns_winning_run() {
    let app = seeded_two_runs().await;
    let current = app.run_current("TASK-2").await.unwrap();
    assert_eq!(current.display_id, "RUN-2");
    assert_eq!(current.status, RunStatus::Waiting);
}

#[tokio::test]
async fn run_current_idle_task_is_not_found() {
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
            title: "Idle".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    let err = app.run_current("TASK-1").await.unwrap_err();
    assert_eq!(err.code(), "not_found");
}
