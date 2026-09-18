#![allow(dead_code)]

use std::ops::Deref;
use std::path::Path;

use taskboard_application::{Actor, App, ProjectAdd, SystemClock, TaskCreate};
use taskboard_core::{ActorKind, Column};
use taskboard_store_sqlite::{open_db, SqliteStore};

pub struct TestApp {
    pub app: App,
    _tmp: tempfile::TempDir,
}

impl TestApp {
    pub fn path(&self) -> &Path {
        self._tmp.path()
    }
}

impl Deref for TestApp {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

pub fn cli_actor() -> Actor {
    Actor {
        kind: ActorKind::Cli,
        label: "local-cli".into(),
    }
}

pub async fn test_app() -> TestApp {
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let store = SqliteStore::new(pool, tmp.path());
    TestApp {
        app: App::new(store, SystemClock),
        _tmp: tmp,
    }
}

pub async fn seed_project(app: &App, name: &str) -> String {
    let project = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: name.into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();
    project.slug
}

pub async fn create_task(
    app: &App,
    project_slug: &str,
    title: &str,
    column: Option<Column>,
    urgent: bool,
) {
    app.task_create(
        &cli_actor(),
        TaskCreate {
            project_slug: project_slug.into(),
            title: title.into(),
            column,
            urgent,
        },
    )
    .await
    .unwrap();
}
