use std::io::{self, BufRead, Write};
use std::str::FromStr;

use serde_json::{json, Value};
use taskboard_application::{
    ActivityQuery, Actor, App, CheckAdd, CommentAdd, InboxScope, LinkAdd, NextClaim,
    OccupancyQuery, ReviewAction, ReviewTask, RunCancel, RunContinue, RunFail, RunFinish,
    RunListQuery, RunStart, RunWait, TaskCreate, TaskListQuery, TaskSpawn, TaskUpdate,
};
use taskboard_core::{CardDisplayStatus, Column, LinkKind};

use crate::ops;
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
    })
}

fn initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "taskboard", "version": env!("CARGO_PKG_VERSION") }
    })
}

fn tools() -> Vec<Value> {
    ToolKind::ALL
        .iter()
        .copied()
        .map(|tool| tool_value(tool.name(), tool.description(), tool.schema()))
        .collect()
}

fn tool_value(name: &str, description: &str, input_schema: Value) -> Value {
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
    for tool in ToolKind::ALL {
        if tool.name() == name {
            return tool.run(app, actor, args).await;
        }
    }
    Err(taskboard_application::AppError::Validation {
        field: "name".into(),
        message: format!("unknown tool `{name}`"),
    })
}

/// Single registry: name, description, schema, and handler cannot drift.
#[derive(Clone, Copy)]
enum ToolKind {
    ProjectList,
    TaskList,
    TaskShow,
    RunList,
    RunShow,
    RunCurrent,
    RunStart,
    Next,
    RunContinue,
    Inbox,
    Status,
    Occupancy,
    TaskSpawn,
    Activity,
    CommentAdd,
    CommentList,
    TaskCreate,
    TaskMove,
    TaskUpdate,
    RunWait,
    RunFinish,
    RunFail,
    RunCancel,
    Review,
    CheckAdd,
    CheckToggle,
    CheckList,
    LinkAdd,
    Stale,
    ProjectDetect,
}

impl ToolKind {
    const ALL: &'static [ToolKind] = &[
        Self::ProjectList,
        Self::TaskList,
        Self::TaskShow,
        Self::RunList,
        Self::RunShow,
        Self::RunCurrent,
        Self::RunStart,
        Self::Next,
        Self::RunContinue,
        Self::Inbox,
        Self::Status,
        Self::Occupancy,
        Self::TaskSpawn,
        Self::Activity,
        Self::CommentAdd,
        Self::CommentList,
        Self::TaskCreate,
        Self::TaskMove,
        Self::TaskUpdate,
        Self::RunWait,
        Self::RunFinish,
        Self::RunFail,
        Self::RunCancel,
        Self::Review,
        Self::CheckAdd,
        Self::CheckToggle,
        Self::CheckList,
        Self::LinkAdd,
        Self::Stale,
        Self::ProjectDetect,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::ProjectList => "project_list",
            Self::TaskList => "task_list",
            Self::TaskShow => "task_show",
            Self::RunList => "run_list",
            Self::RunShow => "run_show",
            Self::RunCurrent => "run_current",
            Self::RunStart => "run_start",
            Self::Next => "next",
            Self::RunContinue => "run_continue",
            Self::Inbox => "inbox",
            Self::Status => "status",
            Self::Occupancy => "occupancy",
            Self::TaskSpawn => "task_spawn",
            Self::Activity => "activity",
            Self::CommentAdd => "comment_add",
            Self::CommentList => "comment_list",
            Self::TaskCreate => "task_create",
            Self::TaskMove => "task_move",
            Self::TaskUpdate => "task_update",
            Self::RunWait => "run_wait",
            Self::RunFinish => "run_finish",
            Self::RunFail => "run_fail",
            Self::RunCancel => "run_cancel",
            Self::Review => "review",
            Self::CheckAdd => "check_add",
            Self::CheckToggle => "check_toggle",
            Self::CheckList => "check_list",
            Self::LinkAdd => "link_add",
            Self::Stale => "stale",
            Self::ProjectDetect => "project_detect",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::ProjectList => "List live projects",
            Self::TaskList => "List tasks in a project or across all live projects",
            Self::TaskShow => "Show one task by TASK-n",
            Self::RunList => "List runs",
            Self::RunShow => "Show one run by RUN-n",
            Self::RunCurrent => "Show the winning run on a task",
            Self::RunStart => "Start a run on a task",
            Self::Next => "Claim the first ready card and start a run",
            Self::RunContinue => "Resume a waiting run as the same RUN-n",
            Self::Inbox => "List cards that need a person",
            Self::Status => "Board-wide snapshot of inbox, open runs, ready, review, and blocked",
            Self::Occupancy => "Group running and waiting runs by worktree path",
            Self::TaskSpawn => "Create child tasks and block the parent on them",
            Self::Activity => "List board activity after a sequence",
            Self::CommentAdd => "Append a comment without changing the note",
            Self::CommentList => "List comments on a task",
            Self::TaskCreate => "Create a task",
            Self::TaskMove => "Move a task to a column",
            Self::TaskUpdate => "Update a task title, worktree, or branch",
            Self::RunWait => "Mark a run as waiting",
            Self::RunFinish => "Mark a run as completed",
            Self::RunFail => "Mark a run as failed",
            Self::RunCancel => "Cancel a running or waiting run",
            Self::Review => "Approve or request changes on an In Review card",
            Self::CheckAdd => "Add a checklist item",
            Self::CheckToggle => "Toggle a checklist item",
            Self::CheckList => "List checklist items on a task",
            Self::LinkAdd => "Add a URL, path, or blocked-by link",
            Self::Stale => "List stale running runs",
            Self::ProjectDetect => "Resolve the current project from cwd or TASKBOARD_PROJECT",
        }
    }

    fn schema(self) -> Value {
        match self {
            Self::ProjectList => json!({
                "type": "object",
                "properties": { "include_archived": { "type": "boolean" } }
            }),
            Self::TaskList => json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "all": { "type": "boolean" },
                    "status": { "type": "string" },
                    "column": { "type": "string" },
                    "agent": { "type": "string" }
                }
            }),
            Self::TaskShow => json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
            Self::RunList => json!({
                "type": "object",
                "properties": {
                    "open": { "type": "boolean" },
                    "session": { "type": "string" },
                    "agent": { "type": "string" }
                }
            }),
            Self::RunShow => json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
            Self::RunCurrent => json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
            Self::RunStart => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "agent": { "type": "string" },
                    "session": { "type": "string" },
                    "exclusive": { "type": "boolean" }
                },
                "required": ["display_id", "agent"]
            }),
            Self::Next => json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "agent": { "type": "string" },
                    "move": { "type": "boolean" }
                }
            }),
            Self::RunContinue => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "message": { "type": "string" },
                    "reply": { "type": "string" }
                },
                "required": ["display_id"]
            }),
            Self::Inbox => json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "archived": { "type": "boolean" }
                }
            }),
            Self::Status => json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" }
                }
            }),
            Self::Occupancy => json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                }
            }),
            Self::TaskSpawn => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "titles": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["display_id", "titles"]
            }),
            Self::Activity => json!({
                "type": "object",
                "properties": {
                    "after": { "type": "integer" },
                    "project": { "type": "string" },
                    "task": { "type": "string" }
                }
            }),
            Self::CommentAdd => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "text": { "type": "string" },
                    "continue": { "type": "boolean" }
                },
                "required": ["display_id", "text"]
            }),
            Self::CommentList => json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
            Self::TaskCreate => json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "title": { "type": "string" },
                    "column": { "type": "string" },
                    "urgent": { "type": "boolean" }
                },
                "required": ["title"]
            }),
            Self::TaskMove => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "column": { "type": "string" }
                },
                "required": ["display_id", "column"]
            }),
            Self::TaskUpdate => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "title": { "type": "string" },
                    "worktree": { "type": "string" },
                    "branch": { "type": "string" }
                },
                "required": ["display_id"]
            }),
            Self::RunWait => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "reason": { "type": "string" }
                },
                "required": ["display_id", "reason"]
            }),
            Self::RunFinish => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "summary": { "type": "string" }
                },
                "required": ["display_id", "summary"]
            }),
            Self::RunFail => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "summary": { "type": "string" }
                },
                "required": ["display_id", "summary"]
            }),
            Self::RunCancel => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "summary": { "type": "string" }
                },
                "required": ["display_id"]
            }),
            Self::Review => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "action": { "type": "string" },
                    "text": { "type": "string" }
                },
                "required": ["display_id", "action", "text"]
            }),
            Self::CheckAdd => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "text": { "type": "string" }
                },
                "required": ["display_id", "text"]
            }),
            Self::CheckToggle => json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
            Self::CheckList => json!({
                "type": "object",
                "properties": { "display_id": { "type": "string" } },
                "required": ["display_id"]
            }),
            Self::LinkAdd => json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "url": { "type": "string" },
                    "path": { "type": "string" },
                    "blocked_by": { "type": "string" }
                },
                "required": ["display_id"]
            }),
            Self::Stale => json!({
                "type": "object",
                "properties": { "minutes": { "type": "integer" } }
            }),
            Self::ProjectDetect => json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    async fn run(
        self,
        app: &App,
        actor: &Actor,
        args: Value,
    ) -> Result<Value, taskboard_application::AppError> {
        match self {
            Self::ProjectList => {
                let include_archived = args
                    .get("include_archived")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let projects = ops::project_list(app, include_archived).await?;
                Ok(ops::entities_ok(&projects))
            }
            Self::TaskList => {
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
                let tasks = ops::task_list(
                    app,
                    TaskListQuery {
                        project,
                        statuses,
                        column,
                        agent: string_arg(&args, "agent"),
                        blocked: args
                            .get("blocked")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        ready: args.get("ready").and_then(Value::as_bool).unwrap_or(false),
                    },
                )
                .await?;
                Ok(ops::entities_ok(&tasks))
            }
            Self::TaskShow => {
                let task = ops::task_show(app, &require_string(&args, "display_id")?).await?;
                Ok(ops::entity_ok(&task, task.revision))
            }
            Self::RunList => {
                let runs = ops::run_list(
                    app,
                    RunListQuery {
                        open: args.get("open").and_then(Value::as_bool).unwrap_or(false),
                        session_id: string_arg(&args, "session"),
                        agent: string_arg(&args, "agent"),
                    },
                )
                .await?;
                Ok(ops::entities_ok(&runs))
            }
            Self::RunShow => {
                let run = ops::run_show(app, &require_string(&args, "display_id")?).await?;
                Ok(ops::entity_ok(&run, run.revision))
            }
            Self::RunCurrent => {
                let run = ops::run_current(app, &require_string(&args, "display_id")?).await?;
                Ok(ops::entity_ok(&run, run.revision))
            }
            Self::RunStart => {
                let cmd = RunStart {
                    task_display_id: require_string(&args, "display_id")?,
                    agent: require_string(&args, "agent")?,
                    session_id: string_arg(&args, "session"),
                };
                let exclusive = args
                    .get("exclusive")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let run = ops::run_start(app, actor, cmd, exclusive).await?;
                Ok(ops::entity_ok(&run, run.revision))
            }
            Self::Next => {
                let run = ops::next(
                    app,
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
                Ok(ops::entity_ok(&run, run.revision))
            }
            Self::RunContinue => {
                let run = ops::run_continue(
                    app,
                    actor,
                    RunContinue {
                        run_display_id: require_string(&args, "display_id")?,
                        message: string_arg(&args, "message"),
                        reply: string_arg(&args, "reply"),
                        revision: None,
                    },
                )
                .await?;
                Ok(ops::entity_ok(&run, run.revision))
            }
            Self::Inbox => {
                let items = ops::inbox(
                    app,
                    InboxScope {
                        project: string_arg(&args, "project"),
                        include_archived: args
                            .get("archived")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    },
                )
                .await?;
                Ok(ops::entities_ok(&items))
            }
            Self::Activity => {
                let rows = ops::activity(
                    app,
                    ActivityQuery {
                        after: args.get("after").and_then(Value::as_i64).unwrap_or(0),
                        project: string_arg(&args, "project"),
                        task_display_id: string_arg(&args, "task"),
                    },
                )
                .await?;
                Ok(ops::entities_ok(&rows))
            }
            Self::CommentAdd => {
                let cmd = CommentAdd {
                    task_display_id: require_string(&args, "display_id")?,
                    body: require_string(&args, "text")?,
                };
                if args
                    .get("continue")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    let result = ops::comment_add_and_continue(app, actor, cmd).await?;
                    Ok(ops::entity_ok(&result, result.run.revision))
                } else {
                    let comment = ops::comment_add(app, actor, cmd).await?;
                    Ok(ops::entity_ok(&comment, 0))
                }
            }
            Self::CommentList => {
                let comments =
                    ops::comment_list(app, &require_string(&args, "display_id")?).await?;
                Ok(ops::entities_ok(&comments))
            }
            Self::TaskCreate => {
                let project = match string_arg(&args, "project") {
                    Some(slug) => slug,
                    None => ops::detect_project(app).await?.slug,
                };
                let task = ops::task_create(
                    app,
                    actor,
                    TaskCreate {
                        project_slug: project,
                        title: require_string(&args, "title")?,
                        column: parse_column(args.get("column"))?,
                        urgent: args.get("urgent").and_then(Value::as_bool).unwrap_or(false),
                    },
                )
                .await?;
                Ok(ops::entity_ok(&task, task.revision))
            }
            Self::TaskMove => {
                let column = parse_column(args.get("column"))?.ok_or_else(|| {
                    taskboard_application::AppError::Validation {
                        field: "column".into(),
                        message: "column is required".into(),
                    }
                })?;
                let task = ops::task_move(
                    app,
                    actor,
                    &require_string(&args, "display_id")?,
                    column,
                    None,
                )
                .await?;
                Ok(ops::entity_ok(&task, task.revision))
            }
            Self::TaskUpdate => {
                let task = ops::task_update(
                    app,
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
                Ok(ops::entity_ok(&task, task.revision))
            }
            Self::RunWait => {
                let run = ops::run_wait(
                    app,
                    actor,
                    RunWait {
                        run_display_id: require_string(&args, "display_id")?,
                        reason: require_string(&args, "reason")?,
                        revision: None,
                    },
                )
                .await?;
                Ok(ops::entity_ok(&run, run.revision))
            }
            Self::RunFinish => {
                let run = ops::run_finish(
                    app,
                    actor,
                    RunFinish {
                        run_display_id: require_string(&args, "display_id")?,
                        summary: require_string(&args, "summary")?,
                        revision: None,
                    },
                )
                .await?;
                Ok(ops::entity_ok(&run, run.revision))
            }
            Self::RunFail => {
                let run = ops::run_fail(
                    app,
                    actor,
                    RunFail {
                        run_display_id: require_string(&args, "display_id")?,
                        summary: require_string(&args, "summary")?,
                        revision: None,
                    },
                )
                .await?;
                Ok(ops::entity_ok(&run, run.revision))
            }
            Self::RunCancel => {
                let run = ops::run_cancel(
                    app,
                    actor,
                    RunCancel {
                        run_display_id: require_string(&args, "display_id")?,
                        summary: string_arg(&args, "summary"),
                        revision: None,
                    },
                )
                .await?;
                Ok(ops::entity_ok(&run, run.revision))
            }
            Self::Review => {
                let action = match require_string(&args, "action")?.as_str() {
                    "approve" => ReviewAction::Approve,
                    "changes" => ReviewAction::Changes,
                    other => {
                        return Err(taskboard_application::AppError::Validation {
                            field: "action".into(),
                            message: format!("unknown action `{other}`"),
                        });
                    }
                };
                let task = ops::review(
                    app,
                    actor,
                    ReviewTask {
                        task_display_id: require_string(&args, "display_id")?,
                        action,
                        text: require_string(&args, "text")?,
                        revision: None,
                    },
                )
                .await?;
                Ok(ops::entity_ok(&task, task.revision))
            }
            Self::CheckAdd => {
                let check = ops::check_add(
                    app,
                    actor,
                    CheckAdd {
                        task_display_id: require_string(&args, "display_id")?,
                        text: require_string(&args, "text")?,
                    },
                )
                .await?;
                Ok(ops::entity_ok(&check, 0))
            }
            Self::CheckToggle => {
                let check =
                    ops::check_toggle(app, actor, &require_string(&args, "display_id")?).await?;
                Ok(ops::entity_ok(&check, 0))
            }
            Self::CheckList => {
                let checks = ops::check_list(app, &require_string(&args, "display_id")?).await?;
                Ok(ops::entities_ok(&checks))
            }
            Self::LinkAdd => {
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
                let task = ops::link_add(
                    app,
                    actor,
                    LinkAdd {
                        task_display_id: require_string(&args, "display_id")?,
                        kind,
                        value,
                        revision: None,
                    },
                )
                .await?;
                Ok(ops::entity_ok(&task, task.revision))
            }
            Self::Stale => {
                let minutes = args.get("minutes").and_then(Value::as_i64).unwrap_or(30);
                let runs = ops::stale(app, minutes).await?;
                Ok(ops::entities_ok(&runs))
            }
            Self::ProjectDetect => {
                let project = ops::detect_project(app).await?;
                Ok(ops::entity_ok(&project, project.revision))
            }
            Self::Status => {
                let snap = ops::status(app, string_arg(&args, "project")).await?;
                Ok(ops::entity_ok(&snap, 0))
            }
            Self::Occupancy => {
                let groups = ops::occupancy(
                    app,
                    OccupancyQuery {
                        path: string_arg(&args, "path"),
                    },
                )
                .await?;
                Ok(ops::entities_ok(&groups))
            }
            Self::TaskSpawn => {
                let titles = args
                    .get("titles")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .filter(|title| !title.is_empty())
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let children = ops::task_spawn(
                    app,
                    actor,
                    TaskSpawn {
                        parent_display_id: require_string(&args, "display_id")?,
                        titles,
                    },
                )
                .await?;
                Ok(ops::entities_ok(&children))
            }
        }
    }
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
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn require_string(args: &Value, key: &str) -> Result<String, taskboard_application::AppError> {
    string_arg(args, key).ok_or_else(|| taskboard_application::AppError::Validation {
        field: key.into(),
        message: format!("{key} is required"),
    })
}

fn parse_column(value: Option<&Value>) -> Result<Option<Column>, taskboard_application::AppError> {
    match value.and_then(Value::as_str) {
        None | Some("") => Ok(None),
        Some(raw) => Column::from_str(raw).map(Some).map_err(|_| {
            taskboard_application::AppError::Validation {
                field: "column".into(),
                message: format!("unknown column `{raw}`"),
            }
        }),
    }
}

fn parse_statuses(
    value: Option<&Value>,
) -> Result<Vec<CardDisplayStatus>, taskboard_application::AppError> {
    match value.and_then(Value::as_str) {
        None | Some("") => Ok(Vec::new()),
        Some(raw) => raw
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(|part| {
                CardDisplayStatus::from_str(part).map_err(|_| {
                    taskboard_application::AppError::Validation {
                        field: "status".into(),
                        message: format!("unknown status `{part}`"),
                    }
                })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_kind_names_are_unique() {
        let mut names = std::collections::BTreeSet::new();
        for tool in ToolKind::ALL {
            assert!(names.insert(tool.name()), "duplicate tool {}", tool.name());
        }
        assert!(names.contains("task_show"));
        assert!(names.contains("project_list"));
        assert_eq!(names.len(), ToolKind::ALL.len());
    }
}
