use std::ops::Deref;

use taskboard_application::{
    Actor, App, ProjectAdd, SystemClock, TaskCreate, TaskSpawn, TaskUpdate,
};
use taskboard_core::{ActorKind, CardDisplayStatus, Column};
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

#[tokio::test]
async fn spawn_creates_todo_children_and_blocks_parent() {
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
            title: "Parent".into(),
            column: Some(Column::InProgress),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_update(
        &actor,
        TaskUpdate {
            display_id: "TASK-1".into(),
            title: None,
            worktree_path: Some(Some("/tmp/parent-wt".into())),
            branch: Some(Some("feat/parent".into())),
            revision: None,
        },
    )
    .await
    .unwrap();
    let children = app
        .task_spawn(
            &actor,
            TaskSpawn {
                parent_display_id: "TASK-1".into(),
                titles: vec!["API contract".into(), "UI slice".into()],
            },
        )
        .await
        .unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].display_id, "TASK-2");
    assert_eq!(children[1].display_id, "TASK-3");
    for child in &children {
        assert_eq!(child.column, Column::Todo);
        assert_eq!(child.display_status, CardDisplayStatus::Idle);
        assert_eq!(child.worktree_path, None);
        assert_eq!(child.branch, None);
        assert_eq!(child.project_id, children[0].project_id);
    }
    let parent = app.task_show("TASK-1").await.unwrap();
    assert!(parent
        .recent_activities
        .iter()
        .any(|activity| activity.operation == "task.spawn"));
    assert_eq!(parent.column, Column::InProgress);
    assert_eq!(parent.worktree_path.as_deref(), Some("/tmp/parent-wt"));
    let values: Vec<_> = parent
        .links
        .iter()
        .filter(|link| link.kind == taskboard_core::LinkKind::BlockedBy)
        .map(|link| link.value.as_str())
        .collect();
    assert!(values.contains(&"TASK-2"));
    assert!(values.contains(&"TASK-3"));
    let listed = app.task_list("renai-sim").await.unwrap();
    let parent_row = listed
        .iter()
        .find(|task| task.display_id == "TASK-1")
        .unwrap();
    assert!(parent_row.blocked_by.contains(&"TASK-2".to_string()));
    assert!(parent_row.blocked_by.contains(&"TASK-3".to_string()));
    let child_row = listed
        .iter()
        .find(|task| task.display_id == "TASK-2")
        .unwrap();
    assert!(child_row.blocks.contains(&"TASK-1".to_string()));
}

#[tokio::test]
async fn spawn_empty_titles_is_validation_error() {
    let app = test_app().await;
    let err = app
        .task_spawn(
            &cli_actor(),
            TaskSpawn {
                parent_display_id: "TASK-1".into(),
                titles: vec![],
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}

#[tokio::test]
async fn spawn_missing_parent_is_not_found() {
    let app = test_app().await;
    let err = app
        .task_spawn(
            &cli_actor(),
            TaskSpawn {
                parent_display_id: "TASK-9".into(),
                titles: vec!["Child".into()],
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "not_found");
}
