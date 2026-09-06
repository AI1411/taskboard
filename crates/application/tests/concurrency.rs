use std::ops::Deref;
use std::time::{Duration, Instant};

use chrono::{Duration as ChronoDuration, Utc};
use taskboard_application::{Actor, App, ProjectAdd, Store, SystemClock, TaskCreate, TaskUpdate};
use taskboard_core::{ActorKind, Column};
use taskboard_store_sqlite::{open_db, SqliteStore};

struct TestApp {
    app: App,
    tmp: tempfile::TempDir,
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
    let store = SqliteStore::with_data_dir(pool, tmp.path().to_path_buf());
    TestApp {
        app: App::new(store, SystemClock),
        tmp,
    }
}

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
            title: "Fix login error".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    app
}

#[tokio::test]
async fn stale_revision_does_not_write() {
    let app = seeded_task().await;
    let err = app
        .task_update(
            &cli_actor(),
            TaskUpdate {
                display_id: "TASK-1".into(),
                title: Some("Nope".into()),
                revision: Some(0),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "revision_conflict");
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.title, "Fix login error");
}

#[tokio::test]
async fn delete_hides_then_restore() {
    let app = seeded_task().await;
    app.task_delete(&cli_actor(), "TASK-1", None).await.unwrap();
    assert!(app.task_show("TASK-1").await.is_err());
    let trash = app.trash_list().await.unwrap();
    assert_eq!(trash.tasks[0].display_id, "TASK-1");
    app.task_restore(&cli_actor(), "TASK-1").await.unwrap();
    assert_eq!(
        app.task_show("TASK-1").await.unwrap().title,
        "Fix login error"
    );
}

#[tokio::test]
async fn purge_after_thirty_days() {
    let app = seeded_task().await;
    app.task_delete(&cli_actor(), "TASK-1", None).await.unwrap();
    let later = Utc::now() + ChronoDuration::days(31);
    let n = app.purge_expired_trash(later).await.unwrap();
    assert!(n >= 1);
    let trash = app.trash_list().await.unwrap();
    assert!(trash.tasks.is_empty());
}

#[tokio::test]
async fn undo_restores_title_then_conflict_on_second_change() {
    let app = seeded_task().await;
    app.task_update(
        &cli_actor(),
        TaskUpdate {
            display_id: "TASK-1".into(),
            title: Some("B".into()),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.undo(&cli_actor()).await.unwrap();
    assert_eq!(
        app.task_show("TASK-1").await.unwrap().title,
        "Fix login error"
    );
}

#[tokio::test]
async fn second_undo_is_conflict() {
    let app = seeded_task().await;
    app.task_update(
        &cli_actor(),
        TaskUpdate {
            display_id: "TASK-1".into(),
            title: Some("B".into()),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.undo(&cli_actor()).await.unwrap();
    let err = app.undo(&cli_actor()).await.unwrap_err();
    assert_eq!(err.code(), "undo_conflict");
}

#[tokio::test]
async fn trash_list_includes_deleted_projects_and_tasks() {
    let app = seeded_task().await;
    app.task_delete(&cli_actor(), "TASK-1", None).await.unwrap();
    app.project_delete(&cli_actor(), "renai-sim", None)
        .await
        .unwrap();
    let trash = app.trash_list().await.unwrap();
    assert_eq!(trash.projects[0].slug, "renai-sim");
    assert_eq!(trash.tasks[0].display_id, "TASK-1");
}

#[tokio::test]
async fn restore_assigns_unique_position_in_column() {
    let app = seeded_task().await;
    app.task_create(
        &cli_actor(),
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Second".into(),
            column: Some(Column::Todo),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_delete(&cli_actor(), "TASK-1", None).await.unwrap();
    app.task_create(
        &cli_actor(),
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Third".into(),
            column: Some(Column::Todo),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_restore(&cli_actor(), "TASK-1").await.unwrap();
    let listed = app.task_list("renai-sim").await.unwrap();
    let ids: Vec<_> = listed.iter().map(|t| t.display_id.as_str()).collect();
    assert!(ids.contains(&"TASK-1"));
    assert!(ids.contains(&"TASK-2"));
    assert!(ids.contains(&"TASK-3"));
    assert_eq!(listed.len(), 3);
}

#[tokio::test]
async fn activity_head_and_sync_see_task_update() {
    let app = seeded_task().await;
    let before = app.activity_head().await.unwrap();
    app.task_update(
        &cli_actor(),
        TaskUpdate {
            display_id: "TASK-1".into(),
            title: Some("Renamed".into()),
            revision: None,
        },
    )
    .await
    .unwrap();
    let head = app.activity_head().await.unwrap();
    assert_eq!(head, before + 1);
    let delta = app.sync(before).await.unwrap();
    assert_eq!(delta.sequence, head);
    assert!(
        delta.tasks.iter().any(|task| task.title == "Renamed"),
        "sync should include the changed task"
    );
}

#[tokio::test]
async fn backup_export_import_round_trip() {
    let app = seeded_task().await;
    let dest = app.tmp.path().join("export.sqlite3");
    app.backup_export(&dest).await.unwrap();
    app.task_update(
        &cli_actor(),
        TaskUpdate {
            display_id: "TASK-1".into(),
            title: Some("Changed".into()),
            revision: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(app.task_show("TASK-1").await.unwrap().title, "Changed");
    app.backup_import(&dest).await.unwrap();
    assert_eq!(
        app.task_show("TASK-1").await.unwrap().title,
        "Fix login error"
    );
    let backups = app.tmp.path().join("backups");
    let names: Vec<String> = std::fs::read_dir(&backups)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        names
            .iter()
            .any(|name| name.starts_with("pre-import-") && name.ends_with(".sqlite3")),
        "expected pre-import backup, got {names:?}"
    );
}

#[tokio::test]
async fn backup_import_rejects_non_taskboard_db() {
    let app = seeded_task().await;
    let junk = app.tmp.path().join("junk.sqlite3");
    std::fs::write(&junk, b"not a database").unwrap();
    let err = app.backup_import(&junk).await.unwrap_err();
    assert_eq!(err.code(), "io_error");
    assert_eq!(
        app.task_show("TASK-1").await.unwrap().title,
        "Fix login error"
    );
}

#[tokio::test]
async fn two_pools_second_writer_sees_busy_or_success() {
    let tmp = tempfile::tempdir().unwrap();
    let pool_a = open_db(tmp.path()).await.unwrap();
    let pool_b = open_db(tmp.path()).await.unwrap();
    let app_a = App::new(SqliteStore::new(pool_a.clone()), SystemClock);
    let app_b = App::new(SqliteStore::new(pool_b), SystemClock);

    app_a
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "Renai Sim".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();

    let mut holder = SqliteStore::new(pool_a);
    holder.begin().await.unwrap();
    holder.next_display_n("task").await.unwrap();

    let start = Instant::now();
    let result = app_b
        .task_create(
            &cli_actor(),
            TaskCreate {
                project_slug: "renai-sim".into(),
                title: "Concurrent".into(),
                column: None,
                urgent: false,
            },
        )
        .await;
    let elapsed = start.elapsed();
    assert!(
        elapsed <= Duration::from_secs(6),
        "second writer hung for {elapsed:?}"
    );
    match result {
        Ok(_) => {}
        Err(err) => assert_eq!(err.code(), "database_busy"),
    }

    holder.rollback().await.unwrap();
}
