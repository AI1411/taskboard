use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use taskboard_core::{
    inbox_membership, inbox_reason, inbox_stale_reason, InboxGroup, InboxItem, Run, RunStatus,
    TaskSummary,
};

use crate::commands::{InboxScope, OccupancyGroup, OccupancyQuery, OccupancyRun, StatusLine};
use crate::error::AppError;
use crate::store::Store;

use super::helpers;

pub(super) fn count_group(items: &[InboxItem], group: InboxGroup) -> usize {
    items
        .iter()
        .filter(|item| {
            inbox_membership(item.column, item.urgent, item.display_status, item.stale)
                == Some(group)
        })
        .count()
}

pub(super) fn inbox_line(item: &InboxItem) -> StatusLine {
    StatusLine {
        display_id: item.display_id.clone(),
        status: item.display_status.as_str().to_string(),
        agent: None,
        detail: if item.reason.is_empty() {
            item.title.clone()
        } else {
            item.reason.clone()
        },
    }
}

pub(super) fn task_line(task: &TaskSummary) -> StatusLine {
    let detail = task
        .waiting_reason
        .as_deref()
        .or(task.run_message.as_deref())
        .filter(|s| !s.is_empty())
        .unwrap_or(task.title.as_str())
        .to_string();
    StatusLine {
        display_id: task.display_id.clone(),
        status: task.display_status.as_str().to_string(),
        agent: None,
        detail,
    }
}

pub(super) fn run_line(run: &Run, tasks: &HashMap<Uuid, &TaskSummary>) -> StatusLine {
    let display_id = tasks
        .get(&run.task_id)
        .map(|task| task.display_id.clone())
        .unwrap_or_else(|| run.display_id.clone());
    let detail = run
        .waiting_reason
        .as_deref()
        .or(run.message.as_deref())
        .or(run.summary.as_deref())
        .unwrap_or("")
        .to_string();
    StatusLine {
        display_id,
        status: run.status.as_str().to_string(),
        agent: Some(run.agent.clone()),
        detail,
    }
}

pub(super) async fn occupancy_inner(
    store: &mut dyn Store,
    query: OccupancyQuery,
) -> Result<Vec<OccupancyGroup>, AppError> {
    let runs = store.list_all_runs().await?;
    let mut groups: std::collections::BTreeMap<String, Vec<OccupancyRun>> =
        std::collections::BTreeMap::new();
    for run in runs {
        if !matches!(run.status, RunStatus::Running | RunStatus::Waiting) {
            continue;
        }
        let Some(task) = store.get_task(run.task_id).await? else {
            continue;
        };
        let Some(path) = task.worktree_path.clone() else {
            continue;
        };
        if let Some(filter) = query.path.as_deref() {
            if path != filter {
                continue;
            }
        }
        groups.entry(path).or_default().push(OccupancyRun {
            run_display_id: run.display_id,
            task_display_id: task.display_id,
            status: run.status.as_str().to_string(),
            agent: run.agent,
        });
    }
    if query.path.is_none() {
        groups.retain(|_, runs| runs.len() >= 2);
    }
    Ok(groups
        .into_iter()
        .map(|(worktree_path, runs)| OccupancyGroup {
            worktree_path,
            runs,
        })
        .collect())
}

pub(super) async fn inbox_inner(
    store: &mut dyn Store,
    scope: InboxScope,
    now: DateTime<Utc>,
) -> Result<Vec<InboxItem>, AppError> {
    let projects = if let Some(slug) = scope.project.as_deref() {
        let project = store
            .get_project_by_slug(slug, false)
            .await?
            .ok_or_else(|| helpers::project_not_found(slug))?;
        if project.archived && !scope.include_archived {
            return Err(helpers::project_not_found(slug));
        }
        vec![project]
    } else {
        store.list_projects(scope.include_archived).await?
    };

    let mut runs_by_task = helpers::group_by_task(store.list_all_runs().await?, |run| run.task_id);
    for project in projects.iter().filter(|project| project.archived) {
        let runs = store.list_runs_for_project(project.id).await?;
        for run in runs {
            runs_by_task.entry(run.task_id).or_default().push(run);
        }
    }

    let mut items = Vec::new();
    for project in projects {
        let tasks = store.list_tasks(project.id).await?;
        for task in tasks {
            if task.deleted_at.is_some() {
                continue;
            }
            let runs = helpers::rows_for(&runs_by_task, task.id);
            let (display_status, run_message, waiting_reason) = helpers::display_from_runs(runs);
            let stale = helpers::winning_run(runs)
                .map(|run| helpers::is_stale(run, now, 30))
                .unwrap_or(false);
            let Some(group) = inbox_membership(task.column, task.urgent, display_status, stale)
            else {
                continue;
            };
            let reason = match group {
                InboxGroup::Stale => {
                    let last = helpers::winning_run(runs)
                        .map(|run| run.updated_at)
                        .unwrap_or(task.updated_at);
                    inbox_stale_reason(last)
                }
                InboxGroup::Review => helpers::winning_run(runs)
                    .and_then(|run| run.summary.clone())
                    .unwrap_or_default(),
                _ => inbox_reason(waiting_reason.as_deref(), run_message.as_deref()),
            };
            items.push(InboxItem {
                id: task.id,
                display_id: task.display_id,
                project_id: task.project_id,
                project_slug: project.slug.clone(),
                project_name: project.name.clone(),
                title: task.title,
                column: task.column,
                urgent: task.urgent,
                revision: task.revision,
                display_status,
                run_message,
                waiting_reason,
                reason,
                stale,
                updated_at: task.updated_at,
            });
        }
    }

    items.sort_by(|left, right| {
        let left_group =
            inbox_membership(left.column, left.urgent, left.display_status, left.stale);
        let right_group = inbox_membership(
            right.column,
            right.urgent,
            right.display_status,
            right.stale,
        );
        left_group
            .cmp(&right_group)
            .then_with(|| right.urgent.cmp(&left.urgent))
            .then_with(|| right.updated_at.cmp(&left.updated_at))
            .then_with(|| left.display_id.cmp(&right.display_id))
    });
    Ok(items)
}
