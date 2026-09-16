use std::ops::Deref;
use std::sync::{Arc, Mutex};

use chrono::{Duration, TimeZone, Utc};
use taskboard_application::{Actor, App, Clock, ProjectAdd, RunStart, SystemClock, TaskCreate};
use taskboard_core::{ActorKind, RunStatus};
use taskboard_store_sqlite::{open_db, SqliteStore};

struct TestApp {
    app: App,
    clock: SharedClock,
    _tmp: tempfile::TempDir,
}

impl Deref for TestApp {
    type Target = App;
    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

#[derive(Clone)]
struct SharedClock(Arc<Mutex<chrono::DateTime<Utc>>>);

impl Clock for SharedClock {
    fn now(&self) -> chrono::DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

fn cli_actor() -> Actor {
    Actor {
        kind: ActorKind::Cli,
        label: "local-cli".into(),
    }
}

async fn seeded_running() -> TestApp {
    let start = Utc.with_ymd_and_hms(2026, 9, 16, 12, 0, 0).unwrap();
    let clock = SharedClock(Arc::new(Mutex::new(start)));
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let store = SqliteStore::new(pool, tmp.path());
    let app = TestApp {
        app: App::new(store, clock.clone()),
        clock,
        _tmp: tmp,
    };
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
    app
}

#[tokio::test]
async fn stale_list_returns_old_running_runs() {
    let app = seeded_running().await;
    let fresh = app.stale_list(30).await.unwrap();
    assert!(fresh.is_empty());
    *app.clock.0.lock().unwrap() += Duration::minutes(31);
    let stale = app.stale_list(30).await.unwrap();
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].display_id, "RUN-1");
    assert_eq!(stale[0].status, RunStatus::Running);
    let listed = app.task_list("renai-sim").await.unwrap();
    assert!(listed[0].stale);
}

#[tokio::test]
async fn stale_minutes_must_be_positive() {
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let store = SqliteStore::new(pool, tmp.path());
    let app = App::new(store, SystemClock);
    let err = app.stale_list(0).await.unwrap_err();
    assert_eq!(err.code(), "validation_error");
}
