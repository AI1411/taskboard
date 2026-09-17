use std::path::Path;

use serde::Serialize;
use taskboard_application::{AppError, BoardStatus, InboxCounts, OccupancyGroup, StatusLine};
use taskboard_core::{
    ActivityEntry, CardDisplayStatus, Check, Column, Comment, EntityType, InboxItem, Project, Run,
    TaskDetail, TaskSummary,
};

pub fn print_error(err: &AppError, json: bool) -> i32 {
    if json {
        println!("{}", error_value(err));
    } else {
        eprintln!("error: {err}");
    }
    match err {
        AppError::DatabaseBusy => 2,
        _ => 1,
    }
}

pub fn print_entity<T: Serialize>(json: bool, entity: &T, revision: i64, human: impl FnOnce()) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "entity": entity,
                "revision": revision,
            })
        );
    } else {
        human();
    }
}

pub fn print_entities<T: Serialize>(json: bool, entities: &[T], human: impl FnOnce()) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "entities": entities,
            })
        );
    } else {
        human();
    }
}

pub fn print_ok(json: bool, human: impl FnOnce()) {
    if json {
        println!("{}", serde_json::json!({ "ok": true }));
    } else {
        human();
    }
}

pub fn print_ok_path(json: bool, path: &Path, human: impl FnOnce()) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "path": path.to_string_lossy(),
            })
        );
    } else {
        human();
    }
}

pub fn print_trash(json: bool, projects: &[Project], tasks: &[TaskSummary], human: impl FnOnce()) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "entity": {
                    "projects": projects,
                    "tasks": tasks,
                },
            })
        );
    } else {
        human();
    }
}

pub fn created_project(project: &Project) {
    println!("Created project  {}  {}", project.slug, project.name);
}

pub fn created_task(task: &TaskDetail) {
    println!(
        "Created {}  {}  [{}]",
        task.display_id,
        task.title,
        task.column.as_str()
    );
}

pub fn print_status(snap: &BoardStatus) {
    println!(
        "inbox  {}  ({})",
        snap.inbox.total,
        inbox_parts(&snap.inbox)
    );
    print_status_lines(&snap.inbox_head);
    println!("open_runs  {}", snap.open_runs);
    print_status_lines(&snap.open_run_head);
    println!("stale  {}", snap.stale);
    print_status_lines(&snap.stale_head);
    println!("ready  {}", snap.ready);
    print_status_lines(&snap.ready_head);
    println!("in_review  {}", snap.in_review);
    print_status_lines(&snap.in_review_head);
    println!("blocked  {}", snap.blocked);
    print_status_lines(&snap.blocked_head);
}

fn inbox_parts(counts: &InboxCounts) -> String {
    let parts = [
        (counts.waiting, "waiting"),
        (counts.failed, "failed"),
        (counts.stale, "stale"),
        (counts.urgent, "urgent"),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, label)| format!("{label} {n}"))
    .collect::<Vec<_>>();
    if parts.is_empty() {
        "-".into()
    } else {
        parts.join(" · ")
    }
}

fn print_status_lines(lines: &[StatusLine]) {
    for line in lines {
        println!(
            "  {}  {}  {}  {}",
            line.display_id,
            line.status,
            line.agent.as_deref().unwrap_or("-"),
            line.detail
        );
    }
}

pub fn print_inbox(items: &[InboxItem]) {
    if items.is_empty() {
        println!("inbox is empty");
        return;
    }
    println!("ID  PROJECT  COLUMN  URGENT  RUN  TITLE  REASON");
    for item in items {
        println!(
            "{}  {}  {}  {}  {}  {}  {}",
            item.display_id,
            item.project_slug,
            item.column.as_str(),
            if item.urgent { "U" } else { "-" },
            run_badge(item.display_status),
            item.title,
            item.reason
        );
    }
}

pub fn print_occupancy(groups: &[OccupancyGroup]) {
    if groups.is_empty() {
        println!("No colliding worktrees");
        return;
    }
    for group in groups {
        println!("{}", group.worktree_path);
        for run in &group.runs {
            println!(
                "  {}  {}  {}  {}",
                run.run_display_id, run.task_display_id, run.status, run.agent
            );
        }
    }
}

pub fn print_run_list(runs: &[Run]) {
    println!("ID  AGENT  STATUS  SESSION");
    for run in runs {
        println!(
            "{}  {}  {}  {}",
            run.display_id,
            run.agent,
            run.status.as_str(),
            run.session_id.as_deref().unwrap_or("-")
        );
    }
}

pub fn print_run(run: &Run) {
    println!(
        "{}  {}  [{}]",
        run.display_id,
        run.agent,
        run.status.as_str()
    );
}

pub fn print_activity_list(rows: &[ActivityEntry]) {
    println!("SEQUENCE  TIME  ACTOR  OPERATION  TARGET");
    for row in rows {
        println!(
            "{}  {}  {}  {}  {}",
            row.sequence,
            row.created_at.to_rfc3339(),
            row.actor,
            row.operation,
            row.target
        );
    }
}

pub fn print_check_list(checks: &[Check]) {
    println!("ID  DONE  TEXT");
    for check in checks {
        println!(
            "{}  {}  {}",
            check.display_id,
            if check.done { "x" } else { "-" },
            check.text
        );
    }
}

pub fn print_comment_list(comments: &[Comment]) {
    println!("TIME  ACTOR  TEXT");
    for comment in comments {
        println!(
            "{}  {}  {}",
            comment.created_at.to_rfc3339(),
            comment.actor_label,
            comment.body
        );
    }
}

pub fn print_task_list(tasks: &[TaskSummary]) {
    println!("ID  COLUMN  URGENT  RUN  TITLE");
    for task in tasks {
        println!(
            "{}  {}  {}  {}  {}",
            task.display_id,
            task.column.as_str(),
            if task.urgent { "U" } else { "-" },
            run_badge(task.display_status),
            task.title
        );
    }
}

pub fn run_badge(status: CardDisplayStatus) -> &'static str {
    match status {
        CardDisplayStatus::Idle => "idle",
        CardDisplayStatus::Running => "running",
        CardDisplayStatus::Waiting => "waiting",
        CardDisplayStatus::Failed => "failed",
        CardDisplayStatus::Completed => "completed",
    }
}

pub fn column_label(column: Column) -> &'static str {
    column.as_str()
}

pub fn undo_human(entity_type: EntityType, entity: &serde_json::Value) -> String {
    match entity_type {
        EntityType::Project => format!(
            "Undid project  {}",
            entity
                .get("slug")
                .and_then(|v| v.as_str())
                .unwrap_or("project")
        ),
        EntityType::Task => format!(
            "Undid task  {}",
            entity
                .get("display_id")
                .and_then(|v| v.as_str())
                .unwrap_or("task")
        ),
        EntityType::Run => format!(
            "Undid run  {}",
            entity
                .get("display_id")
                .and_then(|v| v.as_str())
                .unwrap_or("run")
        ),
        EntityType::Link => format!(
            "Undid link  {}",
            entity.get("id").and_then(|v| v.as_str()).unwrap_or("link")
        ),
    }
}

pub fn error_value(err: &AppError) -> serde_json::Value {
    let (field, current) = match err {
        AppError::Validation { field, .. } => (Some(field.clone()), None),
        AppError::RevisionConflict { current } | AppError::UndoConflict { current } => {
            (None, Some(current.clone()))
        }
        _ => (None, None),
    };
    serde_json::json!({
        "ok": false,
        "error": {
            "code": err.code(),
            "message": err.to_string(),
            "field": field,
            "current": current,
        }
    })
}
