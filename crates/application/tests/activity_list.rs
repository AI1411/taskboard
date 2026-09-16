use std::ops::Deref;

use taskboard_application::{ActivityQuery, Actor, App, ProjectAdd, SystemClock, TaskCreate};
use taskboard_core::ActorKind;
use taskboard_store_sqlite::{open_db, SqliteStore};

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

fn cli_actor() -> Actor {
    Actor {
        kind: ActorKind::Cli,
        label: "local-cli".into(),
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
async fn activity_list_returns_create_rows_in_sequence() {
    let app = seeded_task().await;
    let rows = app
        .activity_list(ActivityQuery {
            after: 0,
            project: None,
            task_display_id: None,
        })
        .await
        .unwrap();
    assert!(rows.len() >= 2);
    assert_eq!(rows[0].operation, "project.create");
    assert_eq!(rows[0].target, "renai-sim");
    assert_eq!(rows[1].operation, "task.create");
    assert_eq!(rows[1].target, "TASK-1");
    assert_eq!(rows[1].actor, "local-cli");
}

#[tokio::test]
async fn activity_list_filters_after_project_and_task() {
    let app = seeded_task().await;
    app.project_add(
        &cli_actor(),
        ProjectAdd {
            name: "Other".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    let after_create = app.activity_head().await.unwrap();
    app.task_note_set(&cli_actor(), "TASK-1", "# Spec".into(), None)
        .await
        .unwrap();
    let after = app
        .activity_list(ActivityQuery {
            after: after_create,
            project: None,
            task_display_id: None,
        })
        .await
        .unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].operation, "task.note.set");
    let by_task = app
        .activity_list(ActivityQuery {
            after: 0,
            project: None,
            task_display_id: Some("TASK-1".into()),
        })
        .await
        .unwrap();
    assert!(by_task.iter().all(|row| row.target == "TASK-1"));
    let by_project = app
        .activity_list(ActivityQuery {
            after: 0,
            project: Some("other".into()),
            task_display_id: None,
        })
        .await
        .unwrap();
    assert!(by_project.iter().all(|row| row.target == "other"));
}
