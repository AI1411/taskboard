use std::ops::Deref;

use taskboard_application::{
    Actor, App, OccupancyQuery, ProjectAdd, RunFinish, RunStart, SystemClock, TaskCreate,
    TaskUpdate,
};
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

async fn seed_open_on(app: &App, title: &str, worktree: &str) -> String {
    let actor = cli_actor();
    let task = app
        .task_create(
            &actor,
            TaskCreate {
                project_slug: "renai-sim".into(),
                title: title.into(),
                column: None,
                urgent: false,
            },
        )
        .await
        .unwrap();
    app.task_update(
        &actor,
        TaskUpdate {
            display_id: task.display_id.clone(),
            title: None,
            worktree_path: Some(Some(worktree.into())),
            branch: None,
            revision: None,
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: task.display_id.clone(),
            agent: "cursor".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    task.display_id
}

#[tokio::test]
async fn occupancy_without_path_lists_only_collisions() {
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
    seed_open_on(&app, "A", "/tmp/shared").await;
    seed_open_on(&app, "B", "/tmp/shared").await;
    seed_open_on(&app, "C", "/tmp/alone").await;
    let groups = app.occupancy(OccupancyQuery { path: None }).await.unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].worktree_path, "/tmp/shared");
    assert_eq!(groups[0].runs.len(), 2);
}

#[tokio::test]
async fn occupancy_path_includes_singleton() {
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
    seed_open_on(&app, "Alone", "/tmp/alone").await;
    let none = app.occupancy(OccupancyQuery { path: None }).await.unwrap();
    assert!(none.is_empty());
    let groups = app
        .occupancy(OccupancyQuery {
            path: Some("/tmp/alone".into()),
        })
        .await
        .unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].runs[0].task_display_id, "TASK-1");
}

#[tokio::test]
async fn occupancy_ignores_completed_runs() {
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
    seed_open_on(&app, "A", "/tmp/shared").await;
    seed_open_on(&app, "B", "/tmp/shared").await;
    app.run_finish(
        &actor,
        RunFinish {
            run_display_id: "RUN-1".into(),
            summary: "done".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let groups = app.occupancy(OccupancyQuery { path: None }).await.unwrap();
    assert!(groups.is_empty());
}
