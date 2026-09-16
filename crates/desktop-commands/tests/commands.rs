use std::ops::Deref;

use taskboard_application::{App, AppError, SystemClock};
use taskboard_core::LinkKind;
use taskboard_desktop_commands::{
    check_add_inner, check_toggle_inner, comment_add_inner, link_add_inner, project_add_inner,
    sync_inner, task_create_inner, AppErrorDto,
};
use taskboard_store_sqlite::{open_db, SqliteStore};

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
    let comment = comment_add_inner(&app, "TASK-1".into(), "use TDD".into())
        .await
        .unwrap();
    assert_eq!(comment.body, "use TDD");
    assert_eq!(comment.actor_label, "local-ui");
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.note_markdown, "");
    assert_eq!(shown.comments[0].body, "use TDD");
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
