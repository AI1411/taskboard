#![allow(unused_imports)]
mod common;
use common::{cli_actor, test_app, TestApp};

use taskboard_application::{
    Actor, App, ProjectAdd, RunStart, RunWait, SystemClock, TaskCreate, TaskListQuery,
};
use taskboard_core::{ActorKind, CardDisplayStatus, Column};
use taskboard_store_sqlite::{open_db, SqliteStore};

#[tokio::test]
async fn task_query_filters_all_projects_by_status_column_and_agent() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "Alpha".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "Beta".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "alpha".into(),
            title: "Review cursor".into(),
            column: Some(Column::InReview),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "beta".into(),
            title: "Wait other".into(),
            column: Some(Column::InReview),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "alpha".into(),
            title: "Todo cursor".into(),
            column: Some(Column::Todo),
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
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-2".into(),
            agent: "codex".into(),
            session_id: None,
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

    let listed = app
        .task_query(TaskListQuery {
            project: None,
            statuses: vec![CardDisplayStatus::Running, CardDisplayStatus::Waiting],
            column: Some(Column::InReview),
            agent: Some("cursor".into()),
            blocked: false,
            ready: false,
        })
        .await
        .unwrap();
    let ids: Vec<_> = listed.iter().map(|task| task.display_id.as_str()).collect();
    assert_eq!(ids, ["TASK-1"]);
}

#[tokio::test]
async fn task_query_unknown_project_is_not_found() {
    let app = test_app().await;
    let err = app
        .task_query(TaskListQuery {
            project: Some("missing".into()),
            statuses: Vec::new(),
            column: None,
            agent: None,
            blocked: false,
            ready: false,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), "not_found");
}
