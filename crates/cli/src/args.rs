use std::path::PathBuf;
use std::str::FromStr;

use clap::{Args, Parser, Subcommand};
use taskboard_application::Actor;
use taskboard_core::{ActorKind, Column};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "taskboard", version, about = "Local Taskboard CLI")]
pub struct Cli {
    /// Print snake_case JSON instead of human text.
    #[arg(long, global = true)]
    pub json: bool,

    /// Data directory (overrides TASKBOARD_DATA_DIR).
    #[arg(long, global = true, value_name = "PATH")]
    pub data_dir: Option<PathBuf>,

    /// Actor label recorded in activity (overrides TASKBOARD_ACTOR).
    #[arg(long = "actor", global = true, value_name = "LABEL")]
    pub actor_label: Option<String>,

    /// Expected revision for mutation commands.
    #[arg(long, global = true, value_name = "N")]
    pub revision: Option<i64>,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn actor(&self) -> Actor {
        let label = self
            .actor_label
            .clone()
            .or_else(|| {
                std::env::var("TASKBOARD_ACTOR")
                    .ok()
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or_else(|| "local-cli".to_string());
        Actor {
            kind: ActorKind::Cli,
            label,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Project commands
    #[command(subcommand)]
    Project(ProjectCommand),
    /// Replace a project note
    #[command(name = "project-note", subcommand)]
    ProjectNote(ProjectNoteCommand),
    /// Task commands
    #[command(subcommand)]
    Task(TaskCommand),
    /// Task note commands
    #[command(subcommand)]
    Note(NoteCommand),
    /// Task link commands
    #[command(subcommand)]
    Link(LinkCommand),
    /// Agent run commands
    #[command(subcommand)]
    Run(RunCommand),
    /// Card comment thread
    #[command(subcommand)]
    Comment(CommentCommand),
    /// Card definition-of-done checklist
    #[command(subcommand)]
    Check(CheckCommand),
    /// Soft-deleted entities
    #[command(subcommand)]
    Trash(TrashCommand),
    /// Cards that need a person
    Inbox {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        archived: bool,
    },
    /// Board-wide snapshot
    Status {
        #[arg(long)]
        project: Option<String>,
    },
    /// Claim the first ready card and start a run
    Next {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long = "move")]
        move_to: bool,
    },
    /// Approve or request changes on an In Review card
    #[command(group(
        clap::ArgGroup::new("verdict")
            .required(true)
            .args(["approve", "changes"])
    ))]
    Review {
        display_id: String,
        #[arg(long)]
        approve: bool,
        #[arg(long)]
        changes: bool,
        #[arg(long)]
        text: String,
    },
    /// Undo the latest undoable activity
    Undo,
    /// Group running and waiting runs by worktree path
    Occupancy {
        #[arg(long)]
        path: Option<String>,
    },
    /// List stale running runs
    Stale {
        #[arg(long, default_value_t = 30)]
        minutes: i64,
    },
    /// List board activity
    Activity {
        #[arg(long)]
        after: Option<i64>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        task: Option<String>,
    },
    /// Database backup
    #[command(subcommand)]
    Backup(BackupCommand),
    /// Start the local HTTP UI
    Serve(ServeArgs),
    /// Serve local MCP tools over stdio
    Mcp,
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommand {
    /// Create a project
    Add {
        #[arg(long)]
        name: String,
        #[arg(long = "path")]
        repo_path: Option<String>,
        #[arg(long)]
        slug: Option<String>,
    },
    /// List live projects
    List {
        /// Include archived projects
        #[arg(long)]
        archived: bool,
        /// Include archived projects
        #[arg(long)]
        all: bool,
    },
    /// Show one project
    Show { slug: String },
    /// Update a project
    Update {
        slug: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long = "path")]
        repo_path: Option<String>,
        #[arg(long = "slug")]
        new_slug: Option<String>,
    },
    /// Archive a project
    Archive { slug: String },
    /// Unarchive a project
    Unarchive { slug: String },
    /// Set live project order
    Reorder {
        #[arg(required = true, num_args = 1..)]
        slugs: Vec<String>,
    },
    /// Soft-delete a project
    Delete { slug: String },
    /// Restore a deleted project
    Restore { slug: String },
    /// Resolve the current project from cwd or TASKBOARD_PROJECT
    Detect,
}

#[derive(Debug, Subcommand)]
pub enum ProjectNoteCommand {
    /// Set the project note from text or a file
    #[command(group(
        clap::ArgGroup::new("body")
            .required(true)
            .args(["text", "file"])
    ))]
    Set {
        slug: String,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum TaskCommand {
    /// Create a task
    Create {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        title: String,
        #[arg(long, value_parser = parse_column)]
        column: Option<Column>,
        #[arg(long)]
        urgent: bool,
    },
    /// Create child tasks and block the parent on them
    Spawn {
        display_id: String,
        #[arg(long = "title", required = true, num_args = 1..)]
        titles: Vec<String>,
    },
    /// List tasks in a project or across all live projects
    List {
        #[arg(long, conflicts_with = "all")]
        project: Option<String>,
        /// List every live project instead of detecting one
        #[arg(long)]
        all: bool,
        #[arg(long, value_delimiter = ',', value_parser = parse_status)]
        status: Vec<taskboard_core::CardDisplayStatus>,
        #[arg(long, value_parser = parse_column)]
        column: Option<Column>,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long, conflicts_with = "ready")]
        blocked: bool,
        #[arg(long, conflicts_with = "blocked")]
        ready: bool,
    },
    /// Show one task
    Show { display_id: String },
    /// Update a task title, worktree, or branch
    Update {
        display_id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(long)]
        branch: Option<String>,
    },
    /// Move a task to a column
    Move {
        display_id: String,
        #[arg(value_parser = parse_column)]
        column: Column,
    },
    /// Reorder a task in its column
    #[command(group(
        clap::ArgGroup::new("place")
            .required(true)
            .args(["before", "end"])
    ))]
    Prioritize {
        display_id: String,
        #[arg(long)]
        before: Option<String>,
        #[arg(long)]
        end: bool,
    },
    /// Set or clear the urgent flag
    Urgent { display_id: String, state: OnOff },
    /// Soft-delete a task
    Delete { display_id: String },
    /// Restore a deleted task
    Restore { display_id: String },
}

#[derive(Debug, Subcommand)]
pub enum NoteCommand {
    /// Append a paragraph to the task note
    #[command(group(
        clap::ArgGroup::new("body")
            .required(true)
            .args(["text", "file"])
    ))]
    Add {
        display_id: String,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Replace the task note
    #[command(group(
        clap::ArgGroup::new("body")
            .required(true)
            .args(["text", "file"])
    ))]
    Set {
        display_id: String,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum LinkCommand {
    /// Add a URL, path, or blocked-by link
    #[command(group(
        clap::ArgGroup::new("target")
            .required(true)
            .args(["url", "path", "blocked_by"])
    ))]
    Add {
        display_id: String,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long = "blocked-by")]
        blocked_by: Option<String>,
    },
    /// Remove a link by UUID
    Remove { link_id: Uuid },
}

#[derive(Debug, Subcommand)]
pub enum RunCommand {
    /// List runs
    List {
        /// Only running and waiting
        #[arg(long)]
        open: bool,
        #[arg(long = "session")]
        session_id: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    /// Show one run by RUN-n or session
    #[command(group(
        clap::ArgGroup::new("target")
            .required(true)
            .args(["run_id", "session_id"])
    ))]
    Show {
        run_id: Option<String>,
        #[arg(long = "session")]
        session_id: Option<String>,
    },
    /// Show the run that drives a card's display status
    Current { display_id: String },
    /// Start a run on a task
    Start {
        display_id: String,
        #[arg(long)]
        agent: String,
        #[arg(long = "session")]
        session_id: Option<String>,
        #[arg(long)]
        exclusive: bool,
    },
    /// Update a running or waiting run
    Update {
        run_id: String,
        #[arg(long)]
        message: Option<String>,
    },
    /// Mark a run as waiting
    Wait {
        run_id: String,
        #[arg(long)]
        reason: String,
    },
    /// Resume a waiting run
    Continue {
        run_id: String,
        #[arg(long)]
        message: Option<String>,
        #[arg(long)]
        reply: Option<String>,
    },
    /// Mark a run as failed
    Fail {
        run_id: String,
        #[arg(long)]
        summary: String,
    },
    /// Mark a run as completed
    Finish {
        run_id: String,
        #[arg(long)]
        summary: String,
    },
    /// Cancel a running or waiting run
    Cancel {
        run_id: String,
        #[arg(long)]
        summary: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum CommentCommand {
    /// Append a comment without changing the note
    Add {
        display_id: String,
        #[arg(long)]
        text: String,
        #[arg(long = "continue")]
        continue_waiting: bool,
    },
    /// List comments on a task
    List { display_id: String },
    /// Delete the latest comment on a task
    Remove { display_id: String },
}

#[derive(Debug, Subcommand)]
pub enum CheckCommand {
    /// Add a checklist item without changing the note
    Add {
        display_id: String,
        #[arg(long)]
        text: String,
    },
    /// Toggle a checklist item
    Toggle { display_id: String },
    /// List checklist items on a task
    List { display_id: String },
    /// Delete a checklist item
    Remove { display_id: String },
}

#[derive(Debug, Subcommand)]
pub enum TrashCommand {
    /// List deleted projects and tasks
    List,
}

#[derive(Debug, Subcommand)]
pub enum BackupCommand {
    /// Export a consistent copy of the database
    Export { file: PathBuf },
    /// Replace the database from a backup file
    Import { file: PathBuf },
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    #[arg(long)]
    pub port: Option<u16>,
    #[arg(long)]
    pub open: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum OnOff {
    On,
    Off,
}

impl OnOff {
    pub fn as_bool(self) -> bool {
        matches!(self, OnOff::On)
    }
}

impl FromStr for OnOff {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "on" => Ok(OnOff::On),
            "off" => Ok(OnOff::Off),
            other => Err(format!("expected on or off, got {other}")),
        }
    }
}

fn parse_column(s: &str) -> Result<Column, String> {
    s.parse::<Column>().map_err(|err| err.to_string())
}

fn parse_status(s: &str) -> Result<taskboard_core::CardDisplayStatus, String> {
    let part = s.trim();
    if part.is_empty() {
        return Err("status must not be empty".into());
    }
    part.parse::<taskboard_core::CardDisplayStatus>()
        .map_err(|err| err.to_string())
}
