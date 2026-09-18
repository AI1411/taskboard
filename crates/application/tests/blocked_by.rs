#![allow(unused_imports)]
mod common;
use common::{cli_actor, test_app, TestApp};

use taskboard_application::{
    Actor, App, LinkAdd, ProjectAdd, SystemClock, TaskCreate, TaskListQuery,
};
use taskboard_core::{ActorKind, Column, LinkKind};
use taskboard_store_sqlite::{open_db, SqliteStore};

async fn seeded_two_tasks() -> TestApp {
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
            title: "Blocker".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Blocked".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    app
}

#[tokio::test]
async fn blocked_by_stores_directed_link_and_does_not_move() {
    let app = seeded_two_tasks().await;
    let detail = app
        .link_add(
            &cli_actor(),
            LinkAdd {
                task_display_id: "TASK-2".into(),
                kind: LinkKind::BlockedBy,
                value: "TASK-1".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(detail.column, Column::Todo);
    assert_eq!(detail.links[0].kind, LinkKind::BlockedBy);
    assert_eq!(detail.links[0].value, "TASK-1");
    let listed = app.task_list("renai-sim").await.unwrap();
    let one = listed.iter().find(|t| t.display_id == "TASK-1").unwrap();
    let two = listed.iter().find(|t| t.display_id == "TASK-2").unwrap();
    assert_eq!(two.blocked_by, vec!["TASK-1".to_string()]);
    assert_eq!(one.blocks, vec!["TASK-2".to_string()]);
    assert!(one.blocked_by.is_empty());
}

#[tokio::test]
async fn blocked_by_cycle_is_validation_error() {
    let app = seeded_two_tasks().await;
    app.link_add(
        &cli_actor(),
        LinkAdd {
            task_display_id: "TASK-2".into(),
            kind: LinkKind::BlockedBy,
            value: "TASK-1".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let err = app
        .link_add(
            &cli_actor(),
            LinkAdd {
                task_display_id: "TASK-1".into(),
                kind: LinkKind::BlockedBy,
                value: "TASK-2".into(),
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}

#[tokio::test]
async fn task_query_blocked_and_ready() {
    let app = seeded_two_tasks().await;
    app.link_add(
        &cli_actor(),
        LinkAdd {
            task_display_id: "TASK-2".into(),
            kind: LinkKind::BlockedBy,
            value: "TASK-1".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let blocked = app
        .task_query(TaskListQuery {
            project: Some("renai-sim".into()),
            blocked: true,
            ..TaskListQuery::default()
        })
        .await
        .unwrap();
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0].display_id, "TASK-2");
    let ready = app
        .task_query(TaskListQuery {
            project: Some("renai-sim".into()),
            ready: true,
            ..TaskListQuery::default()
        })
        .await
        .unwrap();
    assert!(ready.iter().any(|t| t.display_id == "TASK-1"));
    assert!(ready.iter().all(|t| t.display_id != "TASK-2"));
}

#[tokio::test]
async fn removing_blocked_by_clears_block_without_moving() {
    let app = seeded_two_tasks().await;
    let added = app
        .link_add(
            &cli_actor(),
            LinkAdd {
                task_display_id: "TASK-2".into(),
                kind: LinkKind::BlockedBy,
                value: "TASK-1".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    let link_id = added.links[0].id;
    let removed = app.link_remove(&cli_actor(), link_id, None).await.unwrap();
    assert!(removed.links.is_empty());
    assert_eq!(removed.column, Column::Todo);
}
