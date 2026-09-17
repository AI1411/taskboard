use std::ops::Deref;

use taskboard_application::{App, AppError, SystemClock};
use taskboard_core::LinkKind;
use taskboard_desktop_commands::{
    check_add_inner, check_remove_inner, check_toggle_inner, comment_add_inner,
    comment_remove_latest_inner, link_add_inner, occupancy_inner, project_add_inner,
    run_start_inner, status_inner, sync_inner, task_create_inner, task_show_inner,
    task_spawn_inner, task_update_inner, AppErrorDto, TaskPatchArgs,
};
use taskboard_store_sqlite::{open_db, SqliteStore};

#[test]
fn task_patch_args_rejects_unknown_fields() {
    let err = serde_json::from_value::<TaskPatchArgs>(serde_json::json!({
        "display_id": "TASK-1",
        "nope": true
    }))
    .unwrap_err();
    assert!(
        err.to_string().contains("nope"),
        "unknown field should be named: {err}"
    );
}

#[test]
fn app_error_dto_serializes_code_message_field_current() {
    let err = AppErrorDto::from(AppError::Validation {
        field: "title".into(),
        message: "must not be empty".into(),
    });
    let value = serde_json::to_value(&err).unwrap();
    assert_eq!(value["code"], "validation_error");
    assert_eq!(value["message"], "must not be empty");
    assert_eq!(value["field"], "title");
    assert_eq!(value["current"], serde_json::Value::Null);
}

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
async fn project_add_command_creates_slug() {
    let app = test_app().await;
    let p = project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    assert_eq!(p.slug, "renai-sim");
}

#[tokio::test]
async fn sync_sees_cli_equivalent_write_on_same_app() {
    let app = test_app().await;
    project_add_inner(&app, "A".into(), None, None)
        .await
        .unwrap();
    let delta = sync_inner(&app, 0).await.unwrap();
    assert!(delta.sequence >= 1);
    assert_eq!(delta.projects[0].slug, "a");
}

#[tokio::test]
async fn comment_add_command_uses_desktop_actor_and_keeps_note() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    let task = task_create_inner(&app, "renai-sim".into(), "Talk".into(), None, None)
        .await
        .unwrap();
    assert_eq!(task.note_markdown, "");
    let comment = comment_add_inner(&app, "TASK-1".into(), "use TDD".into(), false)
        .await
        .unwrap();
    assert_eq!(comment.body, "use TDD");
    assert_eq!(comment.actor_label, "local-ui");
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.note_markdown, "");
    assert_eq!(shown.comments[0].body, "use TDD");
    let removed = comment_remove_latest_inner(&app, "TASK-1".into())
        .await
        .unwrap();
    assert_eq!(removed.body, "use TDD");
    assert!(app.comment_list("TASK-1").await.unwrap().is_empty());
}

#[tokio::test]
async fn check_add_and_toggle_commands() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    task_create_inner(&app, "renai-sim".into(), "DoD".into(), None, None)
        .await
        .unwrap();
    let check = check_add_inner(&app, "TASK-1".into(), "Write tests".into())
        .await
        .unwrap();
    assert_eq!(check.display_id, "CHECK-1");
    assert!(!check.done);
    let toggled = check_toggle_inner(&app, "CHECK-1".into()).await.unwrap();
    assert!(toggled.done);
    let removed = check_remove_inner(&app, "CHECK-1".into()).await.unwrap();
    assert_eq!(removed.display_id, "CHECK-1");
    assert!(app.check_list("TASK-1").await.unwrap().is_empty());
}

#[tokio::test]
async fn link_add_blocked_by_cycle_is_validation_error() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    task_create_inner(&app, "renai-sim".into(), "A".into(), None, None)
        .await
        .unwrap();
    task_create_inner(&app, "renai-sim".into(), "B".into(), None, None)
        .await
        .unwrap();
    link_add_inner(
        &app,
        "TASK-2".into(),
        LinkKind::BlockedBy,
        "TASK-1".into(),
        None,
    )
    .await
    .unwrap();
    let err = link_add_inner(
        &app,
        "TASK-1".into(),
        LinkKind::BlockedBy,
        "TASK-2".into(),
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(err.code, "validation_error");
    assert_eq!(err.field.as_deref(), Some("blocked_by"));
}

#[tokio::test]
async fn task_update_sets_and_clears_worktree_and_branch() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    task_create_inner(&app, "renai-sim".into(), "Fix login".into(), None, None)
        .await
        .unwrap();

    let set = task_update_inner(
        &app,
        TaskPatchArgs {
            display_id: "TASK-1".into(),
            title: None,
            note_markdown: None,
            urgent: None,
            column: None,
            before_display_id: None,
            worktree_path: Some("/tmp/wt".into()),
            branch: Some("cursor/foo-88ba".into()),
            revision: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(set.worktree_path.as_deref(), Some("/tmp/wt"));
    assert_eq!(set.branch.as_deref(), Some("cursor/foo-88ba"));

    let cleared = task_update_inner(
        &app,
        TaskPatchArgs {
            display_id: "TASK-1".into(),
            title: None,
            note_markdown: None,
            urgent: None,
            column: None,
            before_display_id: None,
            worktree_path: Some("".into()),
            branch: Some("".into()),
            revision: Some(set.revision),
        },
    )
    .await
    .unwrap();
    assert_eq!(cleared.worktree_path, None);
    assert_eq!(cleared.branch, None);
}

#[tokio::test]
async fn status_command_counts_ready_card() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    task_create_inner(&app, "renai-sim".into(), "Fix login".into(), None, None)
        .await
        .unwrap();
    let snap = status_inner(&app, None).await.unwrap();
    assert_eq!(snap.ready, 1);
    assert_eq!(snap.ready_head[0].display_id, "TASK-1");
}

#[tokio::test]
async fn occupancy_command_lists_collision() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    task_create_inner(&app, "renai-sim".into(), "A".into(), None, None)
        .await
        .unwrap();
    task_create_inner(&app, "renai-sim".into(), "B".into(), None, None)
        .await
        .unwrap();
    for id in ["TASK-1", "TASK-2"] {
        task_update_inner(
            &app,
            TaskPatchArgs {
                display_id: id.into(),
                title: None,
                note_markdown: None,
                urgent: None,
                column: None,
                before_display_id: None,
                worktree_path: Some("/tmp/shared".into()),
                branch: None,
                revision: None,
            },
        )
        .await
        .unwrap();
        run_start_inner(&app, id.into(), "cursor".into(), None)
            .await
            .unwrap();
    }
    let groups = occupancy_inner(&app, None).await.unwrap();
    assert_eq!(groups[0].worktree_path, "/tmp/shared");
    assert_eq!(groups[0].runs.len(), 2);
}

#[tokio::test]
async fn spawn_command_creates_child_without_moving_parent() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    let parent = task_create_inner(&app, "renai-sim".into(), "Parent".into(), None, None)
        .await
        .unwrap();
    let kids = task_spawn_inner(&app, "TASK-1".into(), vec!["Child".into()])
        .await
        .unwrap();
    assert_eq!(kids[0].title, "Child");
    assert_eq!(kids[0].column, taskboard_core::Column::Todo);
    let shown = task_show_inner(&app, "TASK-1".into()).await.unwrap();
    assert_eq!(shown.column, parent.column);
    let values: Vec<_> = shown
        .links
        .iter()
        .filter(|link| link.kind == LinkKind::BlockedBy)
        .map(|link| link.value.as_str())
        .collect();
    assert_eq!(values, ["TASK-2"]);
}
