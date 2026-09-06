mod args;
mod data_dir;
mod output;

use std::path::PathBuf;

use chrono::Utc;
use clap::Parser;
use taskboard_application::{
    Actor, App, AppError, LinkAdd, ProjectAdd, ProjectUpdate, RunFail, RunFinish, RunStart,
    RunUpdate, RunWait, SystemClock, TaskCreate, TaskUpdate,
};
use taskboard_core::LinkKind;
use taskboard_store_sqlite::{open_db, SqliteStore};

use args::{
    BackupCommand, Cli, Command, LinkCommand, NoteCommand, ProjectCommand, ProjectNoteCommand,
    RunCommand, TaskCommand, TrashCommand,
};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(code) = run(cli).await {
        std::process::exit(code);
    }
}

async fn run(cli: Cli) -> Result<(), i32> {
    if matches!(cli.command, Command::Serve(_)) {
        eprintln!("error: tb serve is not available in this build");
        return Err(1);
    }

    let json = cli.json;
    let data_dir = data_dir::resolve(cli.data_dir.as_deref());
    let pool = open_db(&data_dir)
        .await
        .map_err(|err| output::print_error(&err, json))?;
    let store = SqliteStore::new(pool, &data_dir);
    let app = App::new(store, SystemClock);
    app.purge_expired_trash(Utc::now())
        .await
        .map_err(|err| output::print_error(&err, json))?;
    let actor = cli.actor();
    dispatch(&app, &actor, cli).await
}

async fn dispatch(app: &App, actor: &Actor, cli: Cli) -> Result<(), i32> {
    let json = cli.json;
    let revision = cli.revision;
    match cli.command {
        Command::Project(cmd) => project_cmd(app, actor, json, revision, cmd).await,
        Command::ProjectNote(cmd) => project_note_cmd(app, actor, json, revision, cmd).await,
        Command::Task(cmd) => task_cmd(app, actor, json, revision, cmd).await,
        Command::Note(cmd) => note_cmd(app, actor, json, revision, cmd).await,
        Command::Link(cmd) => link_cmd(app, actor, json, revision, cmd).await,
        Command::Run(cmd) => run_cmd(app, actor, json, revision, cmd).await,
        Command::Trash(TrashCommand::List) => {
            let trash = app
                .trash_list()
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_trash(json, &trash.projects, &trash.tasks, || {
                println!("PROJECTS");
                for project in &trash.projects {
                    println!("{}  {}", project.slug, project.name);
                }
                println!("TASKS");
                for task in &trash.tasks {
                    println!("{}  {}", task.display_id, task.title);
                }
            });
            Ok(())
        }
        Command::Undo => {
            let result = app
                .undo(actor)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            let revision = result
                .entity
                .get("revision")
                .and_then(|value| value.as_i64())
                .unwrap_or(0);
            let human = output::undo_human(result.entity_type, &result.entity);
            output::print_entity(json, &result.entity, revision, || println!("{human}"));
            Ok(())
        }
        Command::Backup(cmd) => backup_cmd(app, json, cmd).await,
        Command::Serve(_) => unreachable!("serve is handled before opening the database"),
    }
}

async fn project_cmd(
    app: &App,
    actor: &Actor,
    json: bool,
    revision: Option<i64>,
    cmd: ProjectCommand,
) -> Result<(), i32> {
    match cmd {
        ProjectCommand::Add {
            name,
            repo_path,
            slug,
        } => {
            let project = app
                .project_add(
                    actor,
                    ProjectAdd {
                        name,
                        repo_path,
                        slug,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &project, project.revision, || {
                output::created_project(&project);
            });
        }
        ProjectCommand::List { archived, all } => {
            let projects = app
                .project_list(archived || all)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entities(json, &projects, || {
                for project in &projects {
                    if project.archived {
                        println!("{}  {}  [archived]", project.slug, project.name);
                    } else {
                        println!("{}  {}", project.slug, project.name);
                    }
                }
            });
        }
        ProjectCommand::Show { slug } => {
            let project = app
                .project_show(&slug)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &project, project.revision, || {
                println!("{}  {}", project.slug, project.name);
            });
        }
        ProjectCommand::Update {
            slug,
            name,
            repo_path,
            new_slug,
        } => {
            let project = app
                .project_update(
                    actor,
                    ProjectUpdate {
                        slug,
                        name,
                        repo_path,
                        new_slug,
                        revision,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &project, project.revision, || {
                println!("Updated project  {}  {}", project.slug, project.name);
            });
        }
        ProjectCommand::Archive { slug } => {
            let project = app
                .project_archive(actor, &slug, true, revision)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &project, project.revision, || {
                println!("Archived project  {}", project.slug);
            });
        }
        ProjectCommand::Unarchive { slug } => {
            let project = app
                .project_archive(actor, &slug, false, revision)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &project, project.revision, || {
                println!("Unarchived project  {}", project.slug);
            });
        }
        ProjectCommand::Reorder { slugs } => {
            let projects = app
                .project_reorder(actor, &slugs)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entities(json, &projects, || {
                let joined = slugs.join("  ");
                println!("Reordered projects  {joined}");
            });
        }
        ProjectCommand::Delete { slug } => {
            let project = app
                .project_delete(actor, &slug, revision)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &project, project.revision, || {
                println!("Deleted project  {}", project.slug);
            });
        }
        ProjectCommand::Restore { slug } => {
            let project = app
                .project_restore(actor, &slug)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &project, project.revision, || {
                println!("Restored project  {}", project.slug);
            });
        }
    }
    Ok(())
}

async fn project_note_cmd(
    app: &App,
    actor: &Actor,
    json: bool,
    revision: Option<i64>,
    cmd: ProjectNoteCommand,
) -> Result<(), i32> {
    let ProjectNoteCommand::Set { slug, text, file } = cmd;
    let markdown = load_body(text, file).map_err(|err| output::print_error(&err, json))?;
    let project = app
        .project_note_set(actor, &slug, markdown, revision)
        .await
        .map_err(|err| output::print_error(&err, json))?;
    output::print_entity(json, &project, project.revision, || {
        println!("Updated project note  {}", project.slug);
    });
    Ok(())
}

async fn task_cmd(
    app: &App,
    actor: &Actor,
    json: bool,
    revision: Option<i64>,
    cmd: TaskCommand,
) -> Result<(), i32> {
    match cmd {
        TaskCommand::Create {
            project,
            title,
            column,
            urgent,
        } => {
            let task = app
                .task_create(
                    actor,
                    TaskCreate {
                        project_slug: project,
                        title,
                        column,
                        urgent,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                output::created_task(&task);
            });
        }
        TaskCommand::List { project, column } => {
            let mut tasks = app
                .task_list(&project)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            if let Some(column) = column {
                tasks.retain(|task| task.column == column);
            }
            output::print_entities(json, &tasks, || output::print_task_list(&tasks));
        }
        TaskCommand::Show { display_id } => {
            let task = app
                .task_show(&display_id)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                println!(
                    "{}  {}  [{}]",
                    task.display_id,
                    task.title,
                    output::column_label(task.column)
                );
            });
        }
        TaskCommand::Update { display_id, title } => {
            let task = app
                .task_update(
                    actor,
                    TaskUpdate {
                        display_id,
                        title,
                        revision,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                println!("Updated {}  {}", task.display_id, task.title);
            });
        }
        TaskCommand::Move { display_id, column } => {
            let task = app
                .task_move(actor, &display_id, column, revision)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                println!(
                    "Moved {}  [{}]",
                    task.display_id,
                    output::column_label(task.column)
                );
            });
        }
        TaskCommand::Prioritize {
            display_id,
            before,
            end: _,
        } => {
            let task = app
                .task_reorder(actor, &display_id, before.as_deref(), revision)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                println!("Prioritized {}", task.display_id);
            });
        }
        TaskCommand::Urgent { display_id, state } => {
            let task = app
                .task_urgent(actor, &display_id, state.as_bool(), revision)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                println!(
                    "Urgent {}  {}",
                    task.display_id,
                    if task.urgent { "on" } else { "off" }
                );
            });
        }
        TaskCommand::Delete { display_id } => {
            let task = app
                .task_delete(actor, &display_id, revision)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                println!("Deleted {}", task.display_id);
            });
        }
        TaskCommand::Restore { display_id } => {
            let task = app
                .task_restore(actor, &display_id)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                println!("Restored {}", task.display_id);
            });
        }
    }
    Ok(())
}

async fn note_cmd(
    app: &App,
    actor: &Actor,
    json: bool,
    revision: Option<i64>,
    cmd: NoteCommand,
) -> Result<(), i32> {
    match cmd {
        NoteCommand::Add {
            display_id,
            text,
            file,
        } => {
            let paragraph = load_body(text, file).map_err(|err| output::print_error(&err, json))?;
            let task = app
                .task_note_add(actor, &display_id, &paragraph, revision)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                println!("Added note to {}", task.display_id);
            });
        }
        NoteCommand::Set {
            display_id,
            text,
            file,
        } => {
            let markdown = load_body(text, file).map_err(|err| output::print_error(&err, json))?;
            let task = app
                .task_note_set(actor, &display_id, markdown, revision)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                println!("Set note on {}", task.display_id);
            });
        }
    }
    Ok(())
}

async fn link_cmd(
    app: &App,
    actor: &Actor,
    json: bool,
    revision: Option<i64>,
    cmd: LinkCommand,
) -> Result<(), i32> {
    match cmd {
        LinkCommand::Add {
            display_id,
            url,
            path,
        } => {
            let (kind, value) = if let Some(url) = url {
                (LinkKind::Url, url)
            } else {
                (LinkKind::Path, path.expect("clap group requires --path"))
            };
            let task = app
                .link_add(
                    actor,
                    LinkAdd {
                        task_display_id: display_id,
                        kind,
                        value,
                        revision,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                println!("Added link to {}", task.display_id);
            });
        }
        LinkCommand::Remove { link_id } => {
            let task = app
                .link_remove(actor, link_id, revision)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &task, task.revision, || {
                println!("Removed link {}", link_id);
            });
        }
    }
    Ok(())
}

async fn run_cmd(
    app: &App,
    actor: &Actor,
    json: bool,
    revision: Option<i64>,
    cmd: RunCommand,
) -> Result<(), i32> {
    match cmd {
        RunCommand::Start {
            display_id,
            agent,
            session_id,
        } => {
            let run = app
                .run_start(
                    actor,
                    RunStart {
                        task_display_id: display_id,
                        agent,
                        session_id,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &run, run.revision, || {
                println!(
                    "Started {}  {}  [{}]",
                    run.display_id,
                    run.agent,
                    run.status.as_str()
                );
            });
        }
        RunCommand::Update { run_id, message } => {
            let run = app
                .run_update(
                    actor,
                    RunUpdate {
                        run_display_id: run_id,
                        message,
                        revision,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &run, run.revision, || {
                println!("Updated {}", run.display_id);
            });
        }
        RunCommand::Wait { run_id, reason } => {
            let run = app
                .run_wait(
                    actor,
                    RunWait {
                        run_display_id: run_id,
                        reason,
                        revision,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &run, run.revision, || {
                println!("Waiting {}", run.display_id);
            });
        }
        RunCommand::Fail { run_id, summary } => {
            let run = app
                .run_fail(
                    actor,
                    RunFail {
                        run_display_id: run_id,
                        summary,
                        revision,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &run, run.revision, || {
                println!("Failed {}", run.display_id);
            });
        }
        RunCommand::Finish { run_id, summary } => {
            let run = app
                .run_finish(
                    actor,
                    RunFinish {
                        run_display_id: run_id,
                        summary,
                        revision,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &run, run.revision, || {
                println!("Finished {}", run.display_id);
            });
        }
    }
    Ok(())
}

async fn backup_cmd(app: &App, json: bool, cmd: BackupCommand) -> Result<(), i32> {
    match cmd {
        BackupCommand::Export { file } => {
            app.backup_export(&file)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_ok_path(json, &file, || {
                println!("Exported backup to  {}", file.display());
            });
        }
        BackupCommand::Import { file } => {
            app.backup_import(&file)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_ok(json, || println!("Imported backup"));
        }
    }
    Ok(())
}

fn load_body(text: Option<String>, file: Option<PathBuf>) -> Result<String, AppError> {
    if let Some(text) = text {
        return Ok(text);
    }
    let path = file.ok_or_else(|| AppError::Validation {
        field: "text".into(),
        message: "either --text or --file is required".into(),
    })?;
    std::fs::read_to_string(&path).map_err(|err| AppError::Io(err.to_string()))
}
