use std::ops::Deref;

use taskboard_application::{
    Actor, App, AppError, ProjectAdd, Store, SystemClock, TaskCreate, TaskUpdate,
};
use taskboard_core::{ActorKind, CardDisplayStatus, Column, Task};
use taskboard_store_sqlite::{open_db, SqliteStore};

struct TestApp {
    app: App,
    _tmp: tempfile::TempDir,
}

impl TestApp {
    async fn task_row(&self, display_id: &str) -> Task {
        let pool = open_db(self._tmp.path()).await.unwrap();
        let mut store = SqliteStore::new(pool);
        store
            .get_task_by_display_id(display_id, false)
            .await
            .unwrap()
            .unwrap()
    }
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
    let store = SqliteStore::new(pool);
    TestApp {
        app: App::new(store, SystemClock),
        _tmp: tmp,
    }
}

async fn seeded() -> TestApp {
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
    app
}

async fn create_task(app: &App, title: &str, column: Option<Column>, urgent: bool) {
    app.task_create(
        &cli_actor(),
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: title.into(),
            column,
            urgent,
        },
    )
    .await
    .unwrap();
}

async fn seeded_with_two_tasks() -> TestApp {
    let app = seeded().await;
    create_task(&app, "First", None, false).await;
    create_task(&app, "Second", None, false).await;
    app
}

async fn seeded_with_two_tasks_in_different_columns() -> TestApp {
    let app = seeded_with_two_tasks().await;
    app.task_move(&cli_actor(), "TASK-1", Column::InProgress, None)
        .await
        .unwrap();
    app
}

fn todo_ids(list: &[taskboard_core::TaskSummary]) -> Vec<&str> {
    list.iter()
        .filter(|t| t.column == Column::Todo)
        .map(|t| t.display_id.as_str())
        .collect()
}

#[tokio::test]
async fn create_task_gets_task_1_in_todo() {
    let app = seeded().await;
    let t = app
        .task_create(
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
    assert_eq!(t.display_id, "TASK-1");
    assert_eq!(t.column, Column::Todo);
    assert_eq!(t.display_status, CardDisplayStatus::Idle);
    assert!(t.links.is_empty());
    assert!(t.runs.is_empty());
    assert_eq!(t.revision, 1);
}

#[tokio::test]
async fn move_rewrites_positions_in_both_columns() {
    let app = seeded_with_two_tasks().await;
    app.task_move(&cli_actor(), "TASK-1", Column::InProgress, None)
        .await
        .unwrap();
    let listed = app.task_list("renai-sim").await.unwrap();
    assert_eq!(todo_ids(&listed), ["TASK-2"]);
    let remaining = app.task_show("TASK-2").await.unwrap();
    assert_eq!(remaining.column, Column::Todo);
    assert_eq!(remaining.display_id, "TASK-2");
    assert_eq!(app.task_row("TASK-2").await.position, 0);
    assert_eq!(app.task_row("TASK-1").await.position, 0);
    assert_eq!(app.task_row("TASK-1").await.column, Column::InProgress);
}

#[tokio::test]
async fn prioritize_across_columns_errors() {
    let app = seeded_with_two_tasks_in_different_columns().await;
    let err = app
        .task_reorder(&cli_actor(), "TASK-1", Some("TASK-2"), None)
        .await
        .unwrap_err();
    assert_eq!(err.code(), "different_column");
}

#[tokio::test]
async fn blank_title_is_validation_error() {
    let app = seeded().await;
    let err = app
        .task_create(
            &cli_actor(),
            TaskCreate {
                project_slug: "renai-sim".into(),
                title: "  ".into(),
                column: None,
                urgent: false,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
    match err {
        AppError::Validation { field, .. } => assert_eq!(field, "title"),
        other => panic!("expected validation_error, got {other:?}"),
    }
}

#[tokio::test]
async fn display_ids_never_reuse() {
    let app = seeded().await;
    create_task(&app, "A", None, false).await;
    create_task(&app, "B", None, false).await;
    let listed = app.task_list("renai-sim").await.unwrap();
    let ids: Vec<_> = listed.iter().map(|t| t.display_id.as_str()).collect();
    assert_eq!(ids, ["TASK-1", "TASK-2"]);
}

#[tokio::test]
async fn new_non_urgent_cards_go_at_end_of_non_urgent_group() {
    let app = seeded().await;
    create_task(&app, "Urgent first", None, true).await;
    create_task(&app, "Normal A", None, false).await;
    create_task(&app, "Normal B", None, false).await;
    let listed = app.task_list("renai-sim").await.unwrap();
    let ids: Vec<_> = listed.iter().map(|t| t.display_id.as_str()).collect();
    assert_eq!(ids, ["TASK-1", "TASK-2", "TASK-3"]);
    assert!(listed[0].urgent);
    assert!(!listed[1].urgent);
    assert!(!listed[2].urgent);
    assert_eq!(app.task_row("TASK-1").await.position, 0);
    assert_eq!(app.task_row("TASK-2").await.position, 1);
    assert_eq!(app.task_row("TASK-3").await.position, 2);
}

#[tokio::test]
async fn new_urgent_card_goes_at_end_of_urgent_group() {
    let app = seeded().await;
    create_task(&app, "Normal A", None, false).await;
    create_task(&app, "Urgent first", None, true).await;
    create_task(&app, "Urgent second", None, true).await;
    let listed = app.task_list("renai-sim").await.unwrap();
    let ids: Vec<_> = listed.iter().map(|t| t.display_id.as_str()).collect();
    assert_eq!(ids, ["TASK-2", "TASK-3", "TASK-1"]);
    assert_eq!(app.task_row("TASK-2").await.position, 0);
    assert_eq!(app.task_row("TASK-3").await.position, 1);
    assert_eq!(app.task_row("TASK-1").await.position, 2);
}

#[tokio::test]
async fn task_list_sorts_urgent_then_position_and_is_idle() {
    let app = seeded().await;
    create_task(&app, "Later non-urgent", None, false).await;
    create_task(&app, "Urgent", None, true).await;
    let listed = app.task_list("renai-sim").await.unwrap();
    assert_eq!(listed[0].display_id, "TASK-2");
    assert_eq!(listed[1].display_id, "TASK-1");
    assert!(listed
        .iter()
        .all(|t| t.display_status == CardDisplayStatus::Idle));
    assert!(listed.iter().all(|t| t.run_message.is_none()));
}

#[tokio::test]
async fn task_show_includes_empty_links_runs_and_create_activity() {
    let app = seeded().await;
    create_task(&app, "Fix login error", None, false).await;
    let shown = app.task_show("TASK-1").await.unwrap();
    assert!(shown.links.is_empty());
    assert!(shown.runs.is_empty());
    assert_eq!(shown.recent_activities.len(), 1);
    assert_eq!(shown.recent_activities[0].operation, "task.create");
    assert_eq!(shown.display_status, CardDisplayStatus::Idle);
}

#[tokio::test]
async fn create_on_deleted_project_is_not_found() {
    let app = seeded().await;
    app.project_delete(&cli_actor(), "renai-sim", None)
        .await
        .unwrap();
    let err = app
        .task_create(
            &cli_actor(),
            TaskCreate {
                project_slug: "renai-sim".into(),
                title: "Orphan".into(),
                column: None,
                urgent: false,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "not_found");
}

#[tokio::test]
async fn task_update_trims_title_and_checks_revision() {
    let app = seeded().await;
    create_task(&app, "Fix login error", None, false).await;
    let updated = app
        .task_update(
            &cli_actor(),
            TaskUpdate {
                display_id: "TASK-1".into(),
                title: Some("  Trimmed title  ".into()),
                revision: Some(1),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.title, "Trimmed title");
    assert_eq!(updated.revision, 2);
    assert_eq!(updated.recent_activities[0].operation, "task.update");

    let err = app
        .task_update(
            &cli_actor(),
            TaskUpdate {
                display_id: "TASK-1".into(),
                title: Some("Nope".into()),
                revision: Some(1),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "revision_conflict");
}

#[tokio::test]
async fn task_update_blank_title_is_validation_error() {
    let app = seeded().await;
    create_task(&app, "Keep me", None, false).await;
    let err = app
        .task_update(
            &cli_actor(),
            TaskUpdate {
                display_id: "TASK-1".into(),
                title: Some("   ".into()),
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
    match err {
        AppError::Validation { field, .. } => assert_eq!(field, "title"),
        other => panic!("expected validation_error, got {other:?}"),
    }
}

#[tokio::test]
async fn task_reorder_before_none_moves_to_end() {
    let app = seeded().await;
    create_task(&app, "A", None, false).await;
    create_task(&app, "B", None, false).await;
    create_task(&app, "C", None, false).await;
    app.task_reorder(&cli_actor(), "TASK-1", None, None)
        .await
        .unwrap();
    let listed = app.task_list("renai-sim").await.unwrap();
    let ids: Vec<_> = listed.iter().map(|t| t.display_id.as_str()).collect();
    assert_eq!(ids, ["TASK-2", "TASK-3", "TASK-1"]);
    assert_eq!(app.task_row("TASK-2").await.position, 0);
    assert_eq!(app.task_row("TASK-3").await.position, 1);
    assert_eq!(app.task_row("TASK-1").await.position, 2);
}

#[tokio::test]
async fn task_reorder_places_before_target() {
    let app = seeded().await;
    create_task(&app, "A", None, false).await;
    create_task(&app, "B", None, false).await;
    create_task(&app, "C", None, false).await;
    let reordered = app
        .task_reorder(&cli_actor(), "TASK-3", Some("TASK-1"), None)
        .await
        .unwrap();
    assert_eq!(reordered.recent_activities[0].operation, "task.reorder");
    let listed = app.task_list("renai-sim").await.unwrap();
    let ids: Vec<_> = listed.iter().map(|t| t.display_id.as_str()).collect();
    assert_eq!(ids, ["TASK-3", "TASK-1", "TASK-2"]);
}

#[tokio::test]
async fn task_urgent_on_moves_to_end_of_urgent_group() {
    let app = seeded().await;
    create_task(&app, "Urgent", None, true).await;
    create_task(&app, "Normal", None, false).await;
    let toggled = app
        .task_urgent(&cli_actor(), "TASK-2", true, None)
        .await
        .unwrap();
    assert!(toggled.urgent);
    assert_eq!(toggled.recent_activities[0].operation, "task.urgent");
    let listed = app.task_list("renai-sim").await.unwrap();
    let ids: Vec<_> = listed.iter().map(|t| t.display_id.as_str()).collect();
    assert_eq!(ids, ["TASK-1", "TASK-2"]);
    assert_eq!(app.task_row("TASK-1").await.position, 0);
    assert_eq!(app.task_row("TASK-2").await.position, 1);
}

#[tokio::test]
async fn task_urgent_off_moves_to_start_of_non_urgent_group() {
    let app = seeded().await;
    create_task(&app, "U1", None, true).await;
    create_task(&app, "U2", None, true).await;
    create_task(&app, "N1", None, false).await;
    app.task_urgent(&cli_actor(), "TASK-1", false, None)
        .await
        .unwrap();
    let listed = app.task_list("renai-sim").await.unwrap();
    let ids: Vec<_> = listed.iter().map(|t| t.display_id.as_str()).collect();
    assert_eq!(ids, ["TASK-2", "TASK-1", "TASK-3"]);
    assert!(!listed[1].urgent);
    assert_eq!(app.task_row("TASK-2").await.position, 0);
    assert_eq!(app.task_row("TASK-1").await.position, 1);
    assert_eq!(app.task_row("TASK-3").await.position, 2);
}

#[tokio::test]
async fn task_move_records_move_activity() {
    let app = seeded_with_two_tasks().await;
    let moved = app
        .task_move(&cli_actor(), "TASK-1", Column::Done, None)
        .await
        .unwrap();
    assert_eq!(moved.column, Column::Done);
    assert_eq!(moved.recent_activities[0].operation, "task.move");
}
