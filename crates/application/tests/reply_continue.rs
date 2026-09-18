#![allow(unused_imports)]
mod common;
use common::{cli_actor, test_app, TestApp};

use taskboard_application::{
    Actor, App, CommentAdd, ProjectAdd, RunContinue, RunStart, RunWait, SystemClock, TaskCreate,
};
use taskboard_core::{ActorKind, CardDisplayStatus, RunStatus};
use taskboard_store_sqlite::{open_db, SqliteStore};

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
async fn comment_add_and_continue_resumes_waiting_run() {
    let app = seeded_waiting().await;
    let result = app
        .comment_add_and_continue(
            &cli_actor(),
            CommentAdd {
                task_display_id: "TASK-1".into(),
                body: "here is spec".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(result.comment.body, "here is spec");
    assert_eq!(result.run.display_id, "RUN-1");
    assert_eq!(result.run.status, RunStatus::Running);
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.display_status, CardDisplayStatus::Running);
    assert_eq!(shown.comments[0].body, "here is spec");
}

#[tokio::test]
async fn comment_add_and_continue_on_idle_is_validation_error() {
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
    let err = app
        .comment_add_and_continue(
            &actor,
            CommentAdd {
                task_display_id: "TASK-1".into(),
                body: "nope".into(),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
    assert!(app.comment_list("TASK-1").await.unwrap().is_empty());
}

#[tokio::test]
async fn run_continue_reply_writes_comment() {
    let app = seeded_waiting().await;
    let run = app
        .run_continue(
            &cli_actor(),
            RunContinue {
                run_display_id: "RUN-1".into(),
                message: None,
                reply: Some("here is spec".into()),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(run.status, RunStatus::Running);
    let comments = app.comment_list("TASK-1").await.unwrap();
    assert_eq!(comments[0].body, "here is spec");
}
