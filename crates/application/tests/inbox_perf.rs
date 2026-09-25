mod common;

use std::time::Instant;

use chrono::{SecondsFormat, Utc};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use taskboard_application::InboxScope;
use uuid::Uuid;

use common::{seed_project, test_app};

async fn seed_tasks(db: &std::path::Path, project_id: Uuid, count: i64) {
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(db)
            .create_if_missing(false),
    )
    .await
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    for i in 1..=count {
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO tasks (
                id, display_id, project_id, title, column, urgent, note_markdown,
                worktree_path, branch, position, revision, created_at, updated_at, deleted_at
            ) VALUES (?, ?, ?, ?, 'todo', ?, '', NULL, NULL, ?, 1, ?, ?, NULL)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(format!("TASK-{i}"))
        .bind(project_id.as_bytes().as_slice())
        .bind(format!("task {i}"))
        .bind(if i == 1 { 1i64 } else { 0 })
        .bind(i - 1)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE counters SET value = ? WHERE name = 'task'")
        .bind(count)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    pool.close().await;
}

#[tokio::test]
async fn inbox_and_status_stay_under_two_seconds_for_1000_tasks() {
    let app = test_app().await;
    seed_project(&app, "Renai Sim").await;
    let project = app.project_list(false).await.unwrap().remove(0);
    seed_tasks(&app.path().join("taskboard.sqlite3"), project.id, 1000).await;

    let started = Instant::now();
    let inbox = app
        .inbox(InboxScope {
            project: None,
            include_archived: false,
        })
        .await
        .unwrap();
    let inbox_ms = started.elapsed().as_millis();

    let started = Instant::now();
    let status = app.status(None).await.unwrap();
    let status_ms = started.elapsed().as_millis();

    eprintln!("inbox {inbox_ms}ms status {status_ms}ms");
    assert_eq!(inbox.len(), 1, "only the urgent card belongs in the inbox");
    assert_eq!(inbox[0].display_id, "TASK-1");
    assert_eq!(status.inbox.urgent, 1);
    assert!(inbox_ms < 2000, "inbox took {inbox_ms}ms for 1000 tasks");
    assert!(status_ms < 2000, "status took {status_ms}ms for 1000 tasks");
}
