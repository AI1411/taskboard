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
    /// Soft-deleted entities
    #[command(subcommand)]
    Trash(TrashCommand),
    /// Undo the latest undoable activity
    Undo,
    /// Database backup
    #[command(subcommand)]
    Backup(BackupCommand),
    /// Start the local HTTP UI
    Serve(ServeArgs),
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
        project: String,
        #[arg(long)]
        title: String,
        #[arg(long, value_parser = parse_column)]
        column: Option<Column>,
        #[arg(long)]
        urgent: bool,
    },
    /// List tasks in a project
    List {
        #[arg(long)]
        project: String,
        #[arg(long, value_parser = parse_column)]
        column: Option<Column>,
    },
    /// Show one task
    Show { display_id: String },
    /// Update a task title
    Update {
        display_id: String,
        #[arg(long)]
        title: Option<String>,
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
    /// Add a URL or path link
    #[command(group(
        clap::ArgGroup::new("target")
            .required(true)
            .args(["url", "path"])
    ))]
    Add {
        display_id: String,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// Remove a link by UUID
    Remove { link_id: Uuid },
}

#[derive(Debug, Subcommand)]
pub enum RunCommand {
    /// Start a run on a task
    Start {
        display_id: String,
        #[arg(long)]
        agent: String,
        #[arg(long = "session")]
        session_id: Option<String>,
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
