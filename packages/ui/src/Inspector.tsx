import { useEffect, useRef, useState, type Ref } from "react";
import type { Column, TaskDetail } from "@taskboard/types";

import { COLUMNS } from "./columns";
import { EmptyState } from "./EmptyState";
import styles from "./Inspector.module.css";

const ACTIVITY_LABELS: Record<string, string> = {
  "task.create": "Created",
  "task.update": "Updated",
  "task.move": "Moved",
  "task.reorder": "Reordered",
  "task.delete": "Deleted",
  "task.urgent": "Urgent changed",
  "run.start": "Run started",
  "run.finish": "Run finished",
  "run.fail": "Run failed",
};

function activityLabel(operation: string): string {
  return ACTIVITY_LABELS[operation] ?? operation;
}

export function Inspector(props: {
  task: TaskDetail | null;
  open: boolean;
  titleRef?: Ref<HTMLInputElement>;
  onTitleCommit: (title: string) => void;
  onColumnChange: (column: Column) => void;
  onUrgentChange: (urgent: boolean) => void;
  onNoteChange: (markdown: string) => void;
  onDelete: () => void;
  onClose?: () => void;
}) {
  const [title, setTitle] = useState(props.task?.title ?? "");
  const [note, setNote] = useState(props.task?.noteMarkdown ?? "");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const lastId = useRef<string | null>(null);

  useEffect(() => {
    if (!props.task) {
      lastId.current = null;
      setTitle("");
      setNote("");
      return;
    }
    if (lastId.current !== props.task.displayId) {
      lastId.current = props.task.displayId;
      setTitle(props.task.title);
      setNote(props.task.noteMarkdown);
      setConfirmDelete(false);
    }
  }, [props.task]);

  useEffect(() => {
    if (!props.task) return;
    if (note === props.task.noteMarkdown) return;
    const handle = window.setTimeout(() => {
      props.onNoteChange(note);
    }, 400);
    return () => window.clearTimeout(handle);
  }, [note, props.task, props.onNoteChange]);

  if (!props.open) return null;

  if (!props.task) {
    return (
      <aside className={`${styles.panel} ${styles.hidden}`}>
        <div className={styles.empty}>
          <EmptyState>Select a card</EmptyState>
        </div>
      </aside>
    );
  }

  const runs = [...props.task.runs].sort((a, b) => (a.startedAt < b.startedAt ? 1 : -1));
  const activities = [...props.task.recentActivities].sort((a, b) => b.sequence - a.sequence);

  return (
    <>
      <button
        type="button"
        className={styles.backdrop}
        aria-label="Dismiss details"
        onClick={() => props.onClose?.()}
      />
      <aside className={styles.panel} role="dialog" aria-label="Task details">
        <label className={styles.field}>
          Title
          <input
            ref={props.titleRef}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onBlur={() => props.onTitleCommit(title)}
          />
        </label>
        <p className={styles.displayId}>{props.task.displayId}</p>
        <label className={styles.field}>
          Column
          <select
            value={props.task.column}
            onChange={(e) => props.onColumnChange(e.target.value as Column)}
          >
            {COLUMNS.map((col) => (
              <option key={col.id} value={col.id}>
                {col.label}
              </option>
            ))}
          </select>
        </label>
        <label className={styles.switch}>
          <input
            type="checkbox"
            checked={props.task.urgent}
            onChange={(e) => props.onUrgentChange(e.target.checked)}
          />
          Urgent
        </label>
        <label className={styles.field}>
          Note
          <textarea
            className={styles.note}
            value={note}
            onChange={(e) => setNote(e.target.value)}
          />
        </label>
        <div>
          <h3 className={styles.heading}>Links</h3>
          <ul className={styles.list}>
            {props.task.links.map((link) => (
              <li key={link.id}>{link.value}</li>
            ))}
          </ul>
        </div>
        <div>
          <h3 className={styles.heading}>Runs</h3>
          <ul className={styles.list}>
            {runs.map((run) => (
              <li key={run.id}>
                {run.displayId} {run.status}
                {run.message ? ` — ${run.message}` : ""}
              </li>
            ))}
          </ul>
        </div>
        <div>
          <h3 className={styles.heading}>Activity</h3>
          <ul className={styles.list}>
            {activities.map((item) => (
              <li key={item.id}>
                {activityLabel(item.operation)} · {item.actorLabel}
              </li>
            ))}
          </ul>
        </div>
        {confirmDelete ? (
          <div className={styles.deleteRow}>
            <button type="button" className={styles.delete} onClick={props.onDelete}>
              Move to Trash
            </button>
            <button
              type="button"
              className={styles.cancel}
              onClick={() => setConfirmDelete(false)}
            >
              Cancel
            </button>
          </div>
        ) : (
          <button type="button" className={styles.delete} onClick={() => setConfirmDelete(true)}>
            Delete
          </button>
        )}
      </aside>
    </>
  );
}
