#![allow(unused_imports)]
mod common;
use common::{cli_actor, test_app, TestApp};

use taskboard_application::{
    Actor, App, CommentAdd, ProjectAdd, RunStart, RunWait, SystemClock, TaskCreate,
};
use taskboard_core::{ActorKind, CardDisplayStatus};
use taskboard_store_sqlite::{open_db, SqliteStore};

async fn seeded_task() -> TestApp {
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
    app
}

#[tokio::test]
async fn comment_add_does_not_change_note() {
    let app = seeded_task().await;
    app.task_note_set(&cli_actor(), "TASK-1", "# Spec".into(), None)
        .await
        .unwrap();
    let comment = app
        .comment_add(
            &cli_actor(),
            CommentAdd {
                task_display_id: "TASK-1".into(),
                body: "please use TDD".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(comment.body, "please use TDD");
    assert_eq!(comment.actor_label, "local-cli");
    let listed = app.comment_list("TASK-1").await.unwrap();
    assert_eq!(listed.len(), 1);
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.note_markdown, "# Spec");
    assert_eq!(shown.comments.len(), 1);
    assert_eq!(shown.reply, None);
}

#[tokio::test]
async fn waiting_task_exposes_latest_comment_as_reply() {
    let app = seeded_task().await;
    app.run_start(
        &cli_actor(),
        RunStart {
            task_display_id: "TASK-1".into(),
            agent: "cursor".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    app.run_wait(
        &cli_actor(),
        RunWait {
            run_display_id: "RUN-1".into(),
            reason: "need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.comment_add(
        &cli_actor(),
        CommentAdd {
            task_display_id: "TASK-1".into(),
            body: "first".into(),
        },
    )
    .await
    .unwrap();
    app.comment_add(
        &cli_actor(),
        CommentAdd {
            task_display_id: "TASK-1".into(),
            body: "use TDD".into(),
        },
    )
    .await
    .unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.display_status, CardDisplayStatus::Waiting);
    assert_eq!(shown.reply.as_deref(), Some("use TDD"));
    let listed = app.task_list("renai-sim").await.unwrap();
    assert_eq!(listed[0].reply.as_deref(), Some("use TDD"));
}

#[tokio::test]
async fn blank_comment_is_validation_error() {
    let app = seeded_task().await;
    let err = app
        .comment_add(
            &cli_actor(),
            CommentAdd {
                task_display_id: "TASK-1".into(),
                body: "  ".into(),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}

#[tokio::test]
async fn comment_add_records_activity_and_undo_deletes_it() {
    let app = seeded_task().await;
    app.comment_add(
        &cli_actor(),
        CommentAdd {
            task_display_id: "TASK-1".into(),
            body: "use TDD".into(),
        },
    )
    .await
    .unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert!(shown
        .recent_activities
        .iter()
        .any(|activity| activity.operation == "comment.add"));
    app.undo(&cli_actor()).await.unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert!(shown.comments.is_empty());
}

#[tokio::test]
async fn comment_remove_latest_and_undo_restores_it() {
    let app = seeded_task().await;
    app.comment_add(
        &cli_actor(),
        CommentAdd {
            task_display_id: "TASK-1".into(),
            body: "first".into(),
        },
    )
    .await
    .unwrap();
    app.comment_add(
        &cli_actor(),
        CommentAdd {
            task_display_id: "TASK-1".into(),
            body: "latest".into(),
        },
    )
    .await
    .unwrap();
    let removed = app
        .comment_remove_latest(&cli_actor(), "TASK-1")
        .await
        .unwrap();
    assert_eq!(removed.body, "latest");
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.comments.len(), 1);
    assert_eq!(shown.comments[0].body, "first");
    assert!(shown
        .recent_activities
        .iter()
        .any(|activity| activity.operation == "comment.remove"));
    app.undo(&cli_actor()).await.unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.comments.len(), 2);
    assert_eq!(shown.comments[1].body, "latest");
}

#[tokio::test]
async fn comment_remove_latest_on_empty_thread_is_validation_error() {
    let app = seeded_task().await;
    let err = app
        .comment_remove_latest(&cli_actor(), "TASK-1")
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
    assert!(matches!(
        err,
        taskboard_application::AppError::Validation { field, .. } if field == "comment"
    ));
}
