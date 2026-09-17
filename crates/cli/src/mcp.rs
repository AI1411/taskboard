use std::io::{self, BufRead, Write};
use std::str::FromStr;

use serde_json::{json, Value};
use taskboard_application::{
    ActivityQuery, Actor, App, CheckAdd, CommentAdd, InboxScope, LinkAdd, NextClaim, RunCancel,
    RunContinue, RunFail, RunFinish, RunListQuery, RunStart, RunWait, TaskCreate, TaskListQuery,
    TaskUpdate,
};
use taskboard_core::{CardDisplayStatus, Column, LinkKind};

use crate::output;

pub async fn serve(app: &App, actor: &Actor) -> Result<(), i32> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                eprintln!("error: {err}");
                return Err(1);
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(err) => {
                write_line(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": -32700, "message": err.to_string() },
                    }),
                )?;
                continue;
            }
        };
        if request.get("id").is_none() {
            continue;
        }
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(Value::Null);
        let result = match method {
            "initialize" => Ok(initialize()),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => call_tool(app, actor, params).await,
            other => Err(json!({
                "code": -32601,
                "message": format!("method not found: {other}"),
            })),
        };
        let response = match result {
            Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
            Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
        };
        write_line(&mut stdout, response)?;
    }
    Ok(())
}

fn write_line(stdout: &mut io::Stdout, value: Value) -> Result<(), i32> {
    writeln!(stdout, "{value}").map_err(|err| {
        eprintln!("error: {err}");
        1
    })?;
    stdout.flush().map_err(|err| {
        eprintln!("error: {err}");
        1
    })
}

fn initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "taskboard", "version": env!("CARGO_PKG_VERSION") },
    })
}

fn tools() -> Vec<Value> {
    vec![
        tool(
            "project_list",
            "List live projects",
            json!({
                "type": "object",
                "properties": { "include_archived": { "type": "boolean" } }
            }),
        ),
        tool(
            "task_list",
            "List tasks in a project or across all live projects",
            json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "all": { "type": "boolean" },
                    "status": { "type": "string" },
                    "column": { "type": "string" },
                    "agent": { "type": "string" }
                }
            }),
        ),
        tool(
            "task_show",
            "Show one task by TASK-n",
            json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
        ),
        tool(
            "run_list",
            "List runs",
            json!({
                "type": "object",
                "properties": {
                    "open": { "type": "boolean" },
                    "session": { "type": "string" },
                    "agent": { "type": "string" }
                }
            }),
        ),
        tool(
            "run_show",
            "Show one run by RUN-n",
            json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
        ),
        tool(
            "run_current",
            "Show the winning run on a task",
            json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
        ),
        tool(
            "run_start",
            "Start a run on a task",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "agent": { "type": "string" },
                    "session": { "type": "string" },
                    "exclusive": { "type": "boolean" }
                },
                "required": ["display_id", "agent"]
            }),
        ),
        tool(
            "next",
            "Claim the first ready card and start a run",
            json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "agent": { "type": "string" },
                    "move": { "type": "boolean" }
                }
            }),
        ),
        tool(
            "run_continue",
            "Resume a waiting run as the same RUN-n",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "message": { "type": "string" },
                    "reply": { "type": "string" }
                },
                "required": ["display_id"]
            }),
        ),
        tool(
            "inbox",
            "List cards that need a person",
            json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "archived": { "type": "boolean" }
                }
            }),
        ),
        tool(
            "activity",
            "List board activity after a sequence",
            json!({
                "type": "object",
                "properties": {
                    "after": { "type": "integer" },
                    "project": { "type": "string" },
                    "task": { "type": "string" }
                }
            }),
        ),
        tool(
            "comment_add",
            "Append a comment without changing the note",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "text": { "type": "string" },
                    "continue": { "type": "boolean" }
                },
                "required": ["display_id", "text"]
            }),
        ),
        tool(
            "comment_list",
            "List comments on a task",
            json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
        ),
        tool(
            "task_create",
            "Create a task",
            json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "title": { "type": "string" },
                    "column": { "type": "string" },
                    "urgent": { "type": "boolean" }
                },
                "required": ["title"]
            }),
        ),
        tool(
            "task_move",
            "Move a task to a column",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "column": { "type": "string" }
                },
                "required": ["display_id", "column"]
            }),
        ),
        tool(
            "task_update",
            "Update a task title, worktree, or branch",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "title": { "type": "string" },
                    "worktree": { "type": "string" },
                    "branch": { "type": "string" }
                },
                "required": ["display_id"]
            }),
        ),
        tool(
            "run_wait",
            "Mark a run as waiting",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "reason": { "type": "string" }
                },
                "required": ["display_id", "reason"]
            }),
        ),
        tool(
            "run_finish",
            "Mark a run as completed",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "summary": { "type": "string" }
                },
                "required": ["display_id", "summary"]
            }),
        ),
        tool(
            "run_fail",
            "Mark a run as failed",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "summary": { "type": "string" }
                },
                "required": ["display_id", "summary"]
            }),
        ),
        tool(
            "run_cancel",
            "Cancel a running or waiting run",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "summary": { "type": "string" }
                },
                "required": ["display_id"]
            }),
        ),
        tool(
            "check_add",
            "Add a checklist item",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "text": { "type": "string" }
                },
                "required": ["display_id", "text"]
            }),
        ),
        tool(
            "check_toggle",
            "Toggle a checklist item",
            json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
        ),
        tool(
            "check_list",
            "List checklist items on a task",
            json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
        ),
        tool(
            "link_add",
            "Add a URL, path, or blocked-by link",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "url": { "type": "string" },
                    "path": { "type": "string" },
                    "blocked_by": { "type": "string" }
                },
                "required": ["display_id"]
            }),
        ),
        tool(
            "stale",
            "List stale running runs",
            json!({
                "type": "object",
                "properties": { "minutes": { "type": "integer" } }
            }),
        ),
        tool(
            "project_detect",
            "Resolve the current project from cwd or TASKBOARD_PROJECT",
            json!({
                "type": "object",
                "properties": {}
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

async fn call_tool(app: &App, actor: &Actor, params: Value) -> Result<Value, Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| json!({ "code": -32602, "message": "missing tool name" }))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    match dispatch_tool(app, actor, name, args).await {
        Ok(payload) => Ok(json!({
            "content": [{ "type": "text", "text": payload.to_string() }],
            "structuredContent": payload,
        })),
        Err(err) => Ok(json!({
            "content": [{ "type": "text", "text": output::error_value(&err).to_string() }],
            "isError": true,
            "structuredContent": output::error_value(&err),
        })),
    }
}

async fn dispatch_tool(
    app: &App,
    actor: &Actor,
    name: &str,
    args: Value,
) -> Result<Value, taskboard_application::AppError> {
    match name {
        "project_list" => {
            let include_archived = args
                .get("include_archived")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let projects = app.project_list(include_archived).await?;
            Ok(json!({ "ok": true, "entities": projects }))
        }
        "task_list" => {
            let all = args.get("all").and_then(Value::as_bool).unwrap_or(false);
            let project = if all {
                None
            } else {
                args.get("project")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            };
            let statuses = parse_statuses(args.get("status"))?;
            let column = parse_column(args.get("column"))?;
            let tasks = app
                .task_query(TaskListQuery {
                    project,
                    statuses,
                    column,
                    agent: string_arg(&args, "agent"),
                    blocked: args
                        .get("blocked")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    ready: args.get("ready").and_then(Value::as_bool).unwrap_or(false),
                })
                .await?;
            Ok(json!({ "ok": true, "entities": tasks }))
        }
        "task_show" => {
            let task = app.task_show(&require_string(&args, "display_id")?).await?;
            Ok(json!({ "ok": true, "entity": task, "revision": task.revision }))
        }
        "run_list" => {
            let runs = app
                .run_list(RunListQuery {
                    open: args.get("open").and_then(Value::as_bool).unwrap_or(false),
                    session_id: string_arg(&args, "session"),
                    agent: string_arg(&args, "agent"),
                })
                .await?;
            Ok(json!({ "ok": true, "entities": runs }))
        }
        "run_show" => {
            let run = app.run_show(&require_string(&args, "display_id")?).await?;
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "run_current" => {
            let run = app
                .run_current(&require_string(&args, "display_id")?)
                .await?;
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "run_start" => {
            let cmd = RunStart {
                task_display_id: require_string(&args, "display_id")?,
                agent: require_string(&args, "agent")?,
                session_id: string_arg(&args, "session"),
            };
            let exclusive = args
                .get("exclusive")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let run = if exclusive {
                app.run_start_exclusive(actor, cmd).await?
            } else {
                app.run_start(actor, cmd).await?
            };
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "next" => {
            let run = app
                .next(
                    actor,
                    NextClaim {
                        project: string_arg(&args, "project"),
                        agent: string_arg(&args, "agent").unwrap_or_else(|| "cursor".into()),
                        session_id: None,
                        move_to_in_progress: args
                            .get("move")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "run_continue" => {
            let run = app
                .run_continue(
                    actor,
                    RunContinue {
                        run_display_id: require_string(&args, "display_id")?,
                        message: string_arg(&args, "message"),
                        reply: string_arg(&args, "reply"),
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "inbox" => {
            let items = app
                .inbox(InboxScope {
                    project: string_arg(&args, "project"),
                    include_archived: args
                        .get("archived")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                })
                .await?;
            Ok(json!({ "ok": true, "entities": items }))
        }
        "activity" => {
            let rows = app
                .activity_list(ActivityQuery {
                    after: args.get("after").and_then(Value::as_i64).unwrap_or(0),
                    project: string_arg(&args, "project"),
                    task_display_id: string_arg(&args, "task"),
                })
                .await?;
            Ok(json!({ "ok": true, "entities": rows }))
        }
        "comment_add" => {
            let cmd = CommentAdd {
                task_display_id: require_string(&args, "display_id")?,
                body: require_string(&args, "text")?,
            };
            if args
                .get("continue")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                let result = app.comment_add_and_continue(actor, cmd).await?;
                Ok(json!({ "ok": true, "entity": result, "revision": result.run.revision }))
            } else {
                let comment = app.comment_add(actor, cmd).await?;
                Ok(json!({ "ok": true, "entity": comment, "revision": 0 }))
            }
        }
        "comment_list" => {
            let comments = app
                .comment_list(&require_string(&args, "display_id")?)
                .await?;
            Ok(json!({ "ok": true, "entities": comments }))
        }
        "task_create" => {
            let project = match string_arg(&args, "project") {
                Some(slug) => slug,
                None => detect_current_project(app).await?.slug,
            };
            let task = app
                .task_create(
                    actor,
                    TaskCreate {
                        project_slug: project,
                        title: require_string(&args, "title")?,
                        column: parse_column(args.get("column"))?,
                        urgent: args.get("urgent").and_then(Value::as_bool).unwrap_or(false),
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": task, "revision": task.revision }))
        }
        "task_move" => {
            let column = parse_column(args.get("column"))?.ok_or_else(|| {
                taskboard_application::AppError::Validation {
                    field: "column".into(),
                    message: "column is required".into(),
                }
            })?;
            let task = app
                .task_move(actor, &require_string(&args, "display_id")?, column, None)
                .await?;
            Ok(json!({ "ok": true, "entity": task, "revision": task.revision }))
        }
        "task_update" => {
            let task = app
                .task_update(
                    actor,
                    TaskUpdate {
                        display_id: require_string(&args, "display_id")?,
                        title: string_arg(&args, "title"),
                        worktree_path: optional_clearable(&args, "worktree"),
                        branch: optional_clearable(&args, "branch"),
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": task, "revision": task.revision }))
        }
        "run_wait" => {
            let run = app
                .run_wait(
                    actor,
                    RunWait {
                        run_display_id: require_string(&args, "display_id")?,
                        reason: require_string(&args, "reason")?,
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "run_finish" => {
            let run = app
                .run_finish(
                    actor,
                    RunFinish {
                        run_display_id: require_string(&args, "display_id")?,
                        summary: require_string(&args, "summary")?,
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "run_fail" => {
            let run = app
                .run_fail(
                    actor,
                    RunFail {
                        run_display_id: require_string(&args, "display_id")?,
                        summary: require_string(&args, "summary")?,
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "run_cancel" => {
            let run = app
                .run_cancel(
                    actor,
                    RunCancel {
                        run_display_id: require_string(&args, "display_id")?,
                        summary: string_arg(&args, "summary"),
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "check_add" => {
            let check = app
                .check_add(
                    actor,
                    CheckAdd {
                        task_display_id: require_string(&args, "display_id")?,
                        text: require_string(&args, "text")?,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": check, "revision": 0 }))
        }
        "check_toggle" => {
            let check = app
                .check_toggle(actor, &require_string(&args, "display_id")?)
                .await?;
            Ok(json!({ "ok": true, "entity": check, "revision": 0 }))
        }
        "check_list" => {
            let checks = app
                .check_list(&require_string(&args, "display_id")?)
                .await?;
            Ok(json!({ "ok": true, "entities": checks }))
        }
        "link_add" => {
            let (kind, value) = if let Some(url) = string_arg(&args, "url") {
                (LinkKind::Url, url)
            } else if let Some(path) = string_arg(&args, "path") {
                (LinkKind::Path, path)
            } else if let Some(blocked_by) = string_arg(&args, "blocked_by") {
                (LinkKind::BlockedBy, blocked_by)
            } else {
                return Err(taskboard_application::AppError::Validation {
                    field: "target".into(),
                    message: "url, path, or blocked_by is required".into(),
                });
            };
            let task = app
                .link_add(
                    actor,
                    LinkAdd {
                        task_display_id: require_string(&args, "display_id")?,
                        kind,
                        value,
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": task, "revision": task.revision }))
        }
        "stale" => {
            let minutes = args.get("minutes").and_then(Value::as_i64).unwrap_or(30);
            let runs = app.stale_list(minutes).await?;
            Ok(json!({ "ok": true, "entities": runs }))
        }
        "project_detect" => {
            let project = detect_current_project(app).await?;
            Ok(json!({ "ok": true, "entity": project, "revision": project.revision }))
        }
        other => Err(taskboard_application::AppError::Validation {
            field: "name".into(),
            message: format!("unknown tool `{other}`"),
        }),
    }
}

async fn detect_current_project(
    app: &App,
) -> Result<taskboard_core::Project, taskboard_application::AppError> {
    let cwd = std::env::current_dir()
        .map_err(|err| taskboard_application::AppError::Io(err.to_string()))?;
    let env_slug = std::env::var("TASKBOARD_PROJECT").ok();
    app.detect_project(&cwd, env_slug.as_deref().filter(|value| !value.is_empty()))
        .await
}

fn optional_clearable(args: &Value, key: &str) -> Option<Option<String>> {
    args.get(key).and_then(Value::as_str).map(|value| {
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    })
}

fn string_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn require_string(args: &Value, key: &str) -> Result<String, taskboard_application::AppError> {
    string_arg(args, key).ok_or_else(|| taskboard_application::AppError::Validation {
        field: key.to_string(),
        message: format!("{key} is required"),
    })
}

fn parse_column(value: Option<&Value>) -> Result<Option<Column>, taskboard_application::AppError> {
    let Some(raw) = value.and_then(Value::as_str) else {
        return Ok(None);
    };
    Column::from_str(raw)
        .map(Some)
        .map_err(|_| taskboard_application::AppError::Validation {
            field: "column".into(),
            message: format!("unknown column `{raw}`"),
        })
}

fn parse_statuses(
    value: Option<&Value>,
) -> Result<Vec<CardDisplayStatus>, taskboard_application::AppError> {
    let Some(raw) = value.and_then(Value::as_str) else {
        return Ok(Vec::new());
    };
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    raw.split(',')
        .map(|part| {
            CardDisplayStatus::from_str(part.trim()).map_err(|_| {
                taskboard_application::AppError::Validation {
                    field: "status".into(),
                    message: format!("unknown status `{part}`"),
                }
            })
        })
        .collect()
}
