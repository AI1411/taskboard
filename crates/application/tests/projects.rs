use std::ops::Deref;

use taskboard_application::{Actor, App, ProjectAdd, ProjectUpdate, SystemClock};
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
    let store = SqliteStore::new(pool);
    TestApp {
        app: App::new(store, SystemClock),
        _tmp: tmp,
    }
}

#[tokio::test]
async fn add_project_assigns_slug_and_revision_one() {
    let app = test_app().await;
    let p = app
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
    assert_eq!(p.slug, "renai-sim");
    assert_eq!(p.revision, 1);
}

#[tokio::test]
async fn duplicate_explicit_slug_fails() {
    let app = test_app().await;
    app.project_add(
        &cli_actor(),
        ProjectAdd {
            name: "A".into(),
            repo_path: None,
            slug: Some("renai-sim".into()),
        },
    )
    .await
    .unwrap();
    let err = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "B".into(),
                repo_path: None,
                slug: Some("renai-sim".into()),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "duplicate_slug");
}

#[tokio::test]
async fn second_same_name_gets_suffix() {
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
    let p2 = app
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
    assert_eq!(p2.slug, "renai-sim-2");
}

#[tokio::test]
async fn default_list_hides_archived_and_deleted() {
    let app = test_app().await;
    let a = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "A".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();
    let b = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "B".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();
    app.project_archive(&cli_actor(), &b.slug, true, None)
        .await
        .unwrap();
    app.project_delete(&cli_actor(), &a.slug, None)
        .await
        .unwrap();
    let live = app.project_list(false).await.unwrap();
    assert!(live.is_empty());
    let archived = app.project_list(true).await.unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].slug, b.slug);
}

#[tokio::test]
async fn blank_name_is_validation_error() {
    let app = test_app().await;
    let err = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "   ".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}

#[tokio::test]
async fn second_project_sort_order_follows_live_max() {
    let app = test_app().await;
    let a = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "A".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();
    let b = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "B".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(a.sort_order, 0);
    assert_eq!(b.sort_order, 1);
}

#[tokio::test]
async fn project_show_returns_live_and_hides_deleted() {
    let app = test_app().await;
    let p = app
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
    let shown = app.project_show(&p.slug).await.unwrap();
    assert_eq!(shown.id, p.id);
    app.project_delete(&cli_actor(), &p.slug, None)
        .await
        .unwrap();
    let err = app.project_show(&p.slug).await.unwrap_err();
    assert_eq!(err.code(), "not_found");
}

#[tokio::test]
async fn project_update_renames_without_changing_slug() {
    let app = test_app().await;
    let p = app
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
    let updated = app
        .project_update(
            &cli_actor(),
            ProjectUpdate {
                slug: p.slug.clone(),
                name: Some("Renai Simulation".into()),
                repo_path: None,
                new_slug: None,
                revision: Some(1),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.name, "Renai Simulation");
    assert_eq!(updated.slug, "renai-sim");
    assert_eq!(updated.revision, 2);
}

#[tokio::test]
async fn revision_mismatch_returns_conflict_with_current_entity() {
    let app = test_app().await;
    let p = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "A".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();
    let err = app
        .project_update(
            &cli_actor(),
            ProjectUpdate {
                slug: p.slug.clone(),
                name: Some("B".into()),
                repo_path: None,
                new_slug: None,
                revision: Some(99),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "revision_conflict");
    match err {
        taskboard_application::AppError::RevisionConflict { current } => {
            assert_eq!(current["slug"], "a");
            assert_eq!(current["revision"], 1);
        }
        other => panic!("expected revision_conflict, got {other:?}"),
    }
}

#[tokio::test]
async fn restore_fails_when_live_project_owns_slug() {
    let app = test_app().await;
    let a = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "A".into(),
                repo_path: None,
                slug: Some("renai-sim".into()),
            },
        )
        .await
        .unwrap();
    app.project_delete(&cli_actor(), &a.slug, None)
        .await
        .unwrap();
    app.project_add(
        &cli_actor(),
        ProjectAdd {
            name: "B".into(),
            repo_path: None,
            slug: Some("renai-sim".into()),
        },
    )
    .await
    .unwrap();
    let err = app
        .project_restore(&cli_actor(), "renai-sim")
        .await
        .unwrap_err();
    assert_eq!(err.code(), "duplicate_slug");
}

#[tokio::test]
async fn restore_brings_deleted_project_back() {
    let app = test_app().await;
    let p = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "A".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();
    app.project_delete(&cli_actor(), &p.slug, None)
        .await
        .unwrap();
    let restored = app.project_restore(&cli_actor(), &p.slug).await.unwrap();
    assert!(restored.deleted_at.is_none());
    assert_eq!(restored.slug, p.slug);
    assert_eq!(app.project_list(false).await.unwrap().len(), 1);
}

#[tokio::test]
async fn project_note_set_replaces_markdown() {
    let app = test_app().await;
    let p = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "A".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();
    let updated = app
        .project_note_set(&cli_actor(), &p.slug, "# hello".into(), None)
        .await
        .unwrap();
    assert_eq!(updated.note_markdown, "# hello");
    assert_eq!(updated.revision, 2);
    let shown = app.project_show(&p.slug).await.unwrap();
    assert_eq!(shown.note_markdown, "# hello");
}

#[tokio::test]
async fn project_reorder_rewrites_sort_order() {
    let app = test_app().await;
    let a = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "A".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();
    let b = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "B".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();
    let reordered = app
        .project_reorder(&cli_actor(), &[b.slug.clone(), a.slug.clone()])
        .await
        .unwrap();
    assert_eq!(reordered[0].slug, b.slug);
    assert_eq!(reordered[0].sort_order, 0);
    assert_eq!(reordered[1].slug, a.slug);
    assert_eq!(reordered[1].sort_order, 1);
    let listed = app.project_list(false).await.unwrap();
    assert_eq!(listed[0].slug, b.slug);
    assert_eq!(listed[1].slug, a.slug);
}

#[tokio::test]
async fn unarchive_returns_project_to_default_list() {
    let app = test_app().await;
    let p = app
        .project_add(
            &cli_actor(),
            ProjectAdd {
                name: "A".into(),
                repo_path: None,
                slug: None,
            },
        )
        .await
        .unwrap();
    app.project_archive(&cli_actor(), &p.slug, true, None)
        .await
        .unwrap();
    assert!(app.project_list(false).await.unwrap().is_empty());
    app.project_archive(&cli_actor(), &p.slug, false, None)
        .await
        .unwrap();
    let live = app.project_list(false).await.unwrap();
    assert_eq!(live.len(), 1);
    assert!(!live[0].archived);
}
