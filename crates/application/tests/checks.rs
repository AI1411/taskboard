#![allow(unused_imports)]
mod common;
use common::{cli_actor, test_app, TestApp};

use taskboard_application::{
    Actor, App, CheckAdd, ProjectAdd, RunFinish, RunStart, SystemClock, TaskCreate,
};
use taskboard_core::{ActorKind, Column};
use taskboard_store_sqlite::{open_db, SqliteStore};

async fn seeded_task() -> TestApp {
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
    app.task_create(
        &cli_actor(),
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
async fn check_add_list_toggle_does_not_change_note() {
    let app = seeded_task().await;
    app.task_note_set(&cli_actor(), "TASK-1", "# Spec".into(), None)
        .await
        .unwrap();
    let first = app
        .check_add(
            &cli_actor(),
            CheckAdd {
                task_display_id: "TASK-1".into(),
                text: "Add CLI JSON contract".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(first.display_id, "CHECK-1");
    assert!(!first.done);
    app.check_add(
        &cli_actor(),
        CheckAdd {
            task_display_id: "TASK-1".into(),
            text: "Write tests".into(),
        },
    )
    .await
    .unwrap();
    let toggled = app.check_toggle(&cli_actor(), "CHECK-1").await.unwrap();
    assert!(toggled.done);
    let listed = app.check_list("TASK-1").await.unwrap();
    assert_eq!(listed.len(), 2);
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.note_markdown, "# Spec");
    assert_eq!(shown.checks.len(), 2);
    let summary = app.task_list("renai-sim").await.unwrap();
    assert_eq!(summary[0].checklist_done, 1);
    assert_eq!(summary[0].checklist_total, 2);
}

#[tokio::test]
async fn finish_and_move_done_ignore_unchecked_items() {
    let app = seeded_task().await;
    app.check_add(
        &cli_actor(),
        CheckAdd {
            task_display_id: "TASK-1".into(),
            text: "left open".into(),
        },
    )
    .await
    .unwrap();
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
    app.run_finish(
        &cli_actor(),
        RunFinish {
            run_display_id: "RUN-1".into(),
            summary: "shipped anyway".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let moved = app
        .task_move(&cli_actor(), "TASK-1", Column::Done, None)
        .await
        .unwrap();
    assert_eq!(moved.column, Column::Done);
    assert!(!moved.checks[0].done);
}

#[tokio::test]
async fn blank_check_is_validation_error() {
    let app = seeded_task().await;
    let err = app
        .check_add(
            &cli_actor(),
            CheckAdd {
                task_display_id: "TASK-1".into(),
                text: "  ".into(),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}

#[tokio::test]
async fn check_add_and_toggle_record_activity_and_undo_toggle() {
    let app = seeded_task().await;
    app.check_add(
        &cli_actor(),
        CheckAdd {
            task_display_id: "TASK-1".into(),
            text: "Write tests".into(),
        },
    )
    .await
    .unwrap();
    app.check_toggle(&cli_actor(), "CHECK-1").await.unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert!(shown
        .recent_activities
        .iter()
        .any(|activity| activity.operation == "check.toggle"));
    assert!(shown.checks[0].done);
    app.undo(&cli_actor()).await.unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert!(!shown.checks[0].done);
}

#[tokio::test]
async fn check_remove_records_activity_and_undo_restores_it() {
    let app = seeded_task().await;
    app.check_add(
        &cli_actor(),
        CheckAdd {
            task_display_id: "TASK-1".into(),
            text: "Write tests".into(),
        },
    )
    .await
    .unwrap();
    let removed = app.check_remove(&cli_actor(), "CHECK-1").await.unwrap();
    assert_eq!(removed.display_id, "CHECK-1");
    let shown = app.task_show("TASK-1").await.unwrap();
    assert!(shown.checks.is_empty());
    assert!(shown
        .recent_activities
        .iter()
        .any(|activity| activity.operation == "check.remove"));
    app.undo(&cli_actor()).await.unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.checks.len(), 1);
    assert_eq!(shown.checks[0].text, "Write tests");
    assert!(!shown.checks[0].done);
}
