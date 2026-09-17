use std::ops::Deref;

use taskboard_application::{
    Actor, App, CheckAdd, ProjectAdd, ReviewAction, ReviewTask, SystemClock, TaskCreate,
};
use taskboard_core::{ActorKind, Column};
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

async fn seeded_in_review() -> TestApp {
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
            title: "Ship login".into(),
            column: Some(Column::InReview),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app
}

#[tokio::test]
async fn review_approve_comments_and_moves_to_done() {
    let app = seeded_in_review().await;
    app.check_add(
        &cli_actor(),
        CheckAdd {
            task_display_id: "TASK-1".into(),
            text: "Write tests".into(),
        },
    )
    .await
    .unwrap();
    let shown = app
        .review(
            &cli_actor(),
            ReviewTask {
                task_display_id: "TASK-1".into(),
                action: ReviewAction::Approve,
                text: "lgtm".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(shown.column, Column::Done);
    assert_eq!(shown.comments[0].body, "lgtm");
    assert!(!shown.checks[0].done);
    assert!(shown.runs.is_empty());
}

#[tokio::test]
async fn review_changes_moves_to_in_progress_without_starting_a_run() {
    let app = seeded_in_review().await;
    let shown = app
        .review(
            &cli_actor(),
            ReviewTask {
                task_display_id: "TASK-1".into(),
                action: ReviewAction::Changes,
                text: "fix the copy".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(shown.column, Column::InProgress);
    assert_eq!(shown.comments[0].body, "fix the copy");
    assert!(shown.runs.is_empty());
}

#[tokio::test]
async fn review_todo_is_validation_error_and_leaves_no_comment() {
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
            title: "Idle".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    let err = app
        .review(
            &actor,
            ReviewTask {
                task_display_id: "TASK-1".into(),
                action: ReviewAction::Approve,
                text: "nope".into(),
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
    assert!(app.comment_list("TASK-1").await.unwrap().is_empty());
}
