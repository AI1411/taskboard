use std::ops::Deref;
use std::sync::{Arc, Mutex};

use chrono::{Duration, TimeZone, Utc};
use taskboard_application::{
    Actor, App, Clock, InboxScope, ProjectAdd, RunFail, RunFinish, RunStart, RunWait, SystemClock,
    TaskCreate,
};
use taskboard_core::{ActorKind, Column};
use taskboard_store_sqlite::{open_db, SqliteStore};

#[derive(Clone)]
struct SharedClock(Arc<Mutex<chrono::DateTime<Utc>>>);

impl Clock for SharedClock {
    fn now(&self) -> chrono::DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

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
async fn inbox_includes_waiting_failed_and_urgent_not_done() {
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
            title: "Wait me".into(),
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
            title: "Fail me".into(),
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
            title: "Pin me".into(),
            column: None,
            urgent: true,
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
    let wait = app
        .run_start(
            &actor,
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    app.run_wait(
        &actor,
        RunWait {
            run_display_id: wait.display_id,
            reason: "Need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let fail = app
        .run_start(
            &actor,
            RunStart {
                task_display_id: "TASK-2".into(),
                agent: "codex".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    app.run_fail(
        &actor,
        RunFail {
            run_display_id: fail.display_id,
            summary: "boom".into(),
            revision: None,
        },
    )
    .await
    .unwrap();

    let items = app
        .inbox(InboxScope {
            project: None,
            include_archived: false,
        })
        .await
        .unwrap();
    let titles: Vec<_> = items.iter().map(|i| i.title.as_str()).collect();
    assert_eq!(titles, vec!["Wait me", "Fail me", "Pin me"]);
    assert_eq!(items[0].reason, "Need spec");
    assert_eq!(items[0].project_slug, "renai-sim");
    assert_eq!(items[0].waiting_reason.as_deref(), Some("Need spec"));
}

#[tokio::test]
async fn inbox_drops_urgent_done_and_archived_by_default() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "A".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "a".into(),
            title: "Done pin".into(),
            column: Some(Column::Done),
            urgent: true,
        },
    )
    .await
    .unwrap();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "B".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "b".into(),
            title: "Archived wait".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    let run = app
        .run_start(
            &actor,
            RunStart {
                task_display_id: "TASK-2".into(),
                agent: "codex".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    app.run_wait(
        &actor,
        RunWait {
            run_display_id: run.display_id,
            reason: "x".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.project_archive(&actor, "b", true, None).await.unwrap();

    let live = app
        .inbox(InboxScope {
            project: None,
            include_archived: false,
        })
        .await
        .unwrap();
    assert!(live.is_empty());
    let with_arch = app
        .inbox(InboxScope {
            project: None,
            include_archived: true,
        })
        .await
        .unwrap();
    assert_eq!(
        with_arch
            .iter()
            .map(|i| i.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Archived wait"]
    );
}

#[tokio::test]
async fn inbox_project_scope_and_sort() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "A".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "B".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "a".into(),
            title: "A wait".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "b".into(),
            title: "B wait".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    for id in ["TASK-1", "TASK-2"] {
        let run = app
            .run_start(
                &actor,
                RunStart {
                    task_display_id: id.into(),
                    agent: "codex".into(),
                    session_id: None,
                },
            )
            .await
            .unwrap();
        app.run_wait(
            &actor,
            RunWait {
                run_display_id: run.display_id,
                reason: id.into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    }
    let only_a = app
        .inbox(InboxScope {
            project: Some("a".into()),
            include_archived: false,
        })
        .await
        .unwrap();
    assert_eq!(only_a.len(), 1);
    assert_eq!(only_a[0].title, "A wait");
    let both = app
        .inbox(InboxScope {
            project: None,
            include_archived: false,
        })
        .await
        .unwrap();
    assert_eq!(both.len(), 2);
}

#[tokio::test]
async fn inbox_unknown_project_is_not_found() {
    let app = test_app().await;
    let err = app
        .inbox(InboxScope {
            project: Some("nope".into()),
            include_archived: false,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), "not_found");
}

#[tokio::test]
async fn inbox_includes_stale_between_failed_and_urgent() {
    let start = Utc.with_ymd_and_hms(2026, 9, 16, 12, 0, 0).unwrap();
    let clock = SharedClock(Arc::new(Mutex::new(start)));
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let store = SqliteStore::new(pool, tmp.path());
    let app = App::new(store, clock.clone());
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
            title: "Wait me".into(),
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
            title: "Fail me".into(),
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
            title: "Stuck".into(),
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
            title: "Pin me".into(),
            column: None,
            urgent: true,
        },
    )
    .await
    .unwrap();
    let wait = app
        .run_start(
            &actor,
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    app.run_wait(
        &actor,
        RunWait {
            run_display_id: wait.display_id,
            reason: "Need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let fail = app
        .run_start(
            &actor,
            RunStart {
                task_display_id: "TASK-2".into(),
                agent: "codex".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    app.run_fail(
        &actor,
        RunFail {
            run_display_id: fail.display_id,
            summary: "boom".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-3".into(),
            agent: "cursor".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    *clock.0.lock().unwrap() += Duration::minutes(31);
    let items = app
        .inbox(InboxScope {
            project: None,
            include_archived: false,
        })
        .await
        .unwrap();
    let titles: Vec<_> = items.iter().map(|i| i.title.as_str()).collect();
    assert_eq!(titles, vec!["Wait me", "Fail me", "Stuck", "Pin me"]);
    let stuck = items.iter().find(|i| i.title == "Stuck").unwrap();
    assert!(stuck.stale);
    assert_eq!(stuck.reason, "stale · last update 2026-09-16T12:00:00Z");
}

#[tokio::test]
async fn inbox_completed_in_review_uses_run_summary() {
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
            title: "Review me".into(),
            column: Some(Column::InReview),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Pin me".into(),
            column: None,
            urgent: true,
        },
    )
    .await
    .unwrap();
    let done = app
        .run_start(
            &actor,
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "cursor".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    app.run_finish(
        &actor,
        RunFinish {
            run_display_id: done.display_id,
            summary: "shipped login".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let items = app
        .inbox(InboxScope {
            project: None,
            include_archived: false,
        })
        .await
        .unwrap();
    assert_eq!(items[0].display_id, "TASK-1");
    assert_eq!(items[0].reason, "shipped login");
    assert_eq!(items[1].display_id, "TASK-2");
    let snap = app.status(None).await.unwrap();
    assert_eq!(snap.inbox.review, 1);
    assert_eq!(snap.inbox.urgent, 1);
}
