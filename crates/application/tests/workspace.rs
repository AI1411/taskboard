#![allow(unused_imports)]
mod common;
use common::{cli_actor, test_app, TestApp};

use taskboard_application::{
    Actor, App, ProjectAdd, RunStart, SystemClock, TaskCreate, TaskUpdate,
};
use taskboard_core::ActorKind;
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
async fn task_update_sets_and_clears_worktree_and_branch() {
    let app = seeded_task().await;
    let updated = app
        .task_update(
            &cli_actor(),
            TaskUpdate {
                display_id: "TASK-1".into(),
                title: None,
                worktree_path: Some(Some("/tmp/wt".into())),
                branch: Some(Some("cursor/foo-88ba".into())),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.worktree_path.as_deref(), Some("/tmp/wt"));
    assert_eq!(updated.branch.as_deref(), Some("cursor/foo-88ba"));
    let listed = app.task_list("renai-sim").await.unwrap();
    assert_eq!(listed[0].worktree_path.as_deref(), Some("/tmp/wt"));
    assert_eq!(listed[0].branch.as_deref(), Some("cursor/foo-88ba"));

    let cleared = app
        .task_update(
            &cli_actor(),
            TaskUpdate {
                display_id: "TASK-1".into(),
                title: None,
                worktree_path: Some(None),
                branch: Some(None),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(cleared.worktree_path, None);
    assert_eq!(cleared.branch, None);
}

#[tokio::test]
async fn run_current_includes_parent_task_workspace() {
    let app = seeded_task().await;
    app.task_update(
        &cli_actor(),
        TaskUpdate {
            display_id: "TASK-1".into(),
            title: None,
            worktree_path: Some(Some("/tmp/wt".into())),
            branch: Some(Some("cursor/foo-88ba".into())),
            revision: None,
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
    let current = app.run_current("TASK-1").await.unwrap();
    assert_eq!(current.worktree_path.as_deref(), Some("/tmp/wt"));
    assert_eq!(current.branch.as_deref(), Some("cursor/foo-88ba"));
}
