use std::ops::Deref;

use taskboard_application::{App, AppError, SystemClock};
use taskboard_desktop_commands::{project_add_inner, sync_inner, AppErrorDto};
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
