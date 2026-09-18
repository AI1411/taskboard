use chrono::{DateTime, Utc};
use uuid::Uuid;

use taskboard_core::{display_id, Check, DisplayKind, EntityType};

use crate::actor::Actor;
use crate::commands::CheckAdd;
use crate::error::AppError;
use crate::store::Store;

use super::helpers::*;

pub(super) async fn check_add_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: CheckAdd,
    now: DateTime<Utc>,
) -> Result<Check, AppError> {
    let text = require_non_blank("text", &cmd.text)?;
    let task = require_live_task(store, &cmd.task_display_id).await?;
    let existing = store.list_checks(task.id).await?;
    let sort_order = existing
        .iter()
        .map(|check| check.sort_order)
        .max()
        .map(|max| max + 1)
        .unwrap_or(0);
    let n = store.next_display_n("check").await?;
    let check = Check {
        id: Uuid::now_v7(),
        display_id: display_id(DisplayKind::Check, n),
        task_id: task.id,
        text,
        done: false,
        sort_order,
    };
    store.insert_check(&check).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Check,
            operation: "check.add",
            entity_id: check.id,
            previous_revision: None,
            before_json: None,
            after_json: Some(json_value(&check)?),
        },
    )
    .await?;
    Ok(check)
}

pub(super) async fn check_toggle_inner(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    now: DateTime<Utc>,
) -> Result<Check, AppError> {
    let mut check = store
        .get_check_by_display_id(display_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "check".into(),
            id: display_id.to_string(),
        })?;
    let before = check.clone();
    check.done = !check.done;
    store.update_check(&check).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Check,
            operation: "check.toggle",
            entity_id: check.id,
            previous_revision: None,
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&check)?),
        },
    )
    .await?;
    Ok(check)
}

pub(super) async fn check_remove_inner(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    now: DateTime<Utc>,
) -> Result<Check, AppError> {
    let check = store
        .get_check_by_display_id(display_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "check".into(),
            id: display_id.to_string(),
        })?;
    store.delete_check(check.id).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Check,
            operation: "check.remove",
            entity_id: check.id,
            previous_revision: None,
            before_json: Some(json_value(&check)?),
            after_json: None,
        },
    )
    .await?;
    Ok(check)
}
