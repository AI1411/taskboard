use std::path::Path;

use serde::Serialize;
use taskboard_application::AppError;
use taskboard_core::{CardDisplayStatus, Column, EntityType, Project, TaskDetail, TaskSummary};

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

fn error_value(err: &AppError) -> serde_json::Value {
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
