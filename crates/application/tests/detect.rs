#![allow(unused_imports)]
mod common;
use common::{cli_actor, test_app, TestApp};
use std::path::Path;

use taskboard_application::{Actor, App, ProjectAdd, SystemClock};
use taskboard_core::ActorKind;
use taskboard_store_sqlite::{open_db, SqliteStore};

#[tokio::test]
async fn detect_uses_env_slug() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "Renai Sim".into(),
            repo_path: Some("/tmp/renai".into()),
            slug: None,
        },
    )
    .await
    .unwrap();
    let found = app
        .detect_project(Path::new("/tmp/unrelated"), Some("renai-sim"))
        .await
        .unwrap();
    assert_eq!(found.slug, "renai-sim");
}

#[tokio::test]
async fn detect_unique_repo_ancestor() {
    let app = test_app().await;
    let actor = cli_actor();
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("renai");
    std::fs::create_dir_all(repo.join("crates")).unwrap();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "Renai Sim".into(),
            repo_path: Some(repo.to_string_lossy().into()),
            slug: None,
        },
    )
    .await
    .unwrap();
    let found = app
        .detect_project(&repo.join("crates"), None)
        .await
        .unwrap();
    assert_eq!(found.slug, "renai-sim");
}

#[tokio::test]
async fn detect_missing_is_project_required() {
    let app = test_app().await;
    let err = app
        .detect_project(Path::new("/tmp/nope"), None)
        .await
        .unwrap_err();
    assert_eq!(err.code(), "project_required");
}

#[tokio::test]
async fn detect_unknown_env_slug_is_not_found() {
    let app = test_app().await;
    let err = app
        .detect_project(Path::new("/tmp"), Some("missing"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), "not_found");
}

#[tokio::test]
async fn detect_longest_repo_path_wins() {
    let app = test_app().await;
    let actor = cli_actor();
    let tmp = tempfile::tempdir().unwrap();
    let parent = tmp.path().join("mono");
    let child = parent.join("svc");
    std::fs::create_dir_all(&child).unwrap();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "Mono".into(),
            repo_path: Some(parent.to_string_lossy().into()),
            slug: None,
        },
    )
    .await
    .unwrap();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "Svc".into(),
            repo_path: Some(child.to_string_lossy().into()),
            slug: None,
        },
    )
    .await
    .unwrap();
    let found = app.detect_project(&child, None).await.unwrap();
    assert_eq!(found.slug, "svc");
}
