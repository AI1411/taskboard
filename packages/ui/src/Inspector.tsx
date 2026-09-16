import { useEffect, useRef, useState, type Ref } from "react";
import type { Column, TaskDetail } from "@taskboard/types";

import { COLUMNS } from "./columns";
import { ConfirmDialog } from "./ConfirmDialog";
import { openableHref } from "./openableHref";
import { relativeTime } from "./relativeTime";
import styles from "./Inspector.module.css";

const ACTIVITY_LABELS: Record<string, string> = {
  "task.create": "Created",
  "task.update": "Updated",
  "task.move": "Moved",
  "task.reorder": "Reordered",
  "task.delete": "Deleted",
  "task.urgent": "Urgent changed",
  "task.restore": "Restored",
  "run.start": "Run started",
  "run.finish": "Run finished",
  "run.fail": "Run failed",
  "run.wait": "Waiting",
  "run.update": "Run updated",
  "run.continue": "Run continued",
  "link.add": "Link added",
  "link.remove": "Link removed",
  undo: "Undo",
};

const RUN_STATUS: Record<string, string> = {
  running: "Running",
  waiting: "Waiting",
  failed: "Failed",
  completed: "Done",
};

function activityLabel(operation: string): string {
  if (ACTIVITY_LABELS[operation]) return ACTIVITY_LABELS[operation];
  const last = operation.split(".").pop() ?? operation;
  return last.replace(/[-_]/g, " ").replace(/\b\w/g, (ch) => ch.toUpperCase());
}

export function Inspector(props: {
  task: TaskDetail | null;
  open: boolean;
  titleRef?: Ref<HTMLInputElement>;
  trashed?: boolean;
  confirming?: boolean;
  onTitleCommit: (title: string) => void;
  onColumnChange: (column: Column) => void;
  onUrgentChange: (urgent: boolean) => void;
  onNoteChange: (markdown: string) => void;
  onDeleteRequest?: () => void;
  onDelete: () => void;
  onCancelDelete?: () => void;
  onRestore?: () => void;
  onLinkAdd?: (value: string) => void;
  onLinkRemove?: (linkId: string) => void;
  onCopyId?: (displayId: string) => void;
  onClose?: () => void;
}) {
  const [title, setTitle] = useState(props.task?.title ?? "");
  const [note, setNote] = useState(props.task?.noteMarkdown ?? "");
  const [linkValue, setLinkValue] = useState("");
  const lastId = useRef<string | null>(null);
  const panelRef = useRef<HTMLElement>(null);

  useEffect(() => {
    if (!props.task) {
      lastId.current = null;
      setTitle("");
      setNote("");
      setLinkValue("");
      return;
    }
    if (lastId.current !== props.task.displayId) {
      lastId.current = props.task.displayId;
      setTitle(props.task.title);
      setNote(props.task.noteMarkdown);
      setLinkValue("");
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

  useEffect(() => {
    if (!props.open || !props.task) return;
    panelRef.current?.focus();
  }, [props.open, props.task?.displayId]);

  if (!props.open || !props.task) return null;

  const runs = [...props.task.runs].sort((a, b) => (a.startedAt < b.startedAt ? 1 : -1));
  const activities = [...props.task.recentActivities].sort((a, b) => b.sequence - a.sequence);

  return (
    <aside
      ref={panelRef}
      className={styles.panel}
      aria-label="Task details"
      tabIndex={-1}
    >
        <label className={styles.field}>
          Title
          <input
            ref={props.titleRef}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onBlur={() => props.onTitleCommit(title)}
          />
        </label>
        <button
          type="button"
          className={styles.displayId}
          aria-label={`Copy ${props.task.displayId}`}
          onClick={() => props.onCopyId?.(props.task.displayId)}
        >
          {props.task.displayId}
        </button>
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
          <h3 className={styles.heading}>Checklist</h3>
          <ul className={styles.list}>
            {props.task.checks.map((check) => (
              <li key={check.id}>
                <label className={styles.switch}>
                  <input type="checkbox" checked={check.done} readOnly />
                  {check.text}
                </label>
              </li>
            ))}
          </ul>
        </div>
        <div>
          <h3 className={styles.heading}>Comments</h3>
          <ul className={styles.list}>
            {props.task.comments.map((comment) => (
              <li key={comment.id}>
                {comment.createdAt} · {comment.actorLabel} · {comment.body}
              </li>
            ))}
          </ul>
        </div>
        <div>
          <h3 className={styles.heading}>Links</h3>
          <ul className={styles.list}>
            {props.task.links.map((link) => {
              const href = openableHref(link.value);
              return (
                <li key={link.id} className={styles.linkRow}>
                  {href ? (
                    <a
                      className={styles.linkValue}
                      href={href}
                      target="_blank"
                      rel="noreferrer"
                    >
                      {link.value}
                    </a>
                  ) : (
                    <button
                      type="button"
                      className={styles.linkValue}
                      onClick={() => props.onCopyId?.(link.value)}
                    >
                      {link.value}
                    </button>
                  )}
                  <button
                    type="button"
                    className={styles.linkRemove}
                    aria-label={`Remove ${link.value}`}
                    onClick={() => props.onLinkRemove?.(link.id)}
                  >
                    Remove
                  </button>
                </li>
              );
            })}
          </ul>
          <div className={styles.linkAdd}>
            <input
              className={styles.linkInput}
              placeholder="https:// or /path"
              value={linkValue}
              onChange={(e) => setLinkValue(e.target.value)}
            />
            <button
              type="button"
              className={styles.linkButton}
              onClick={() => {
                const value = linkValue.trim();
                if (!value) return;
                props.onLinkAdd?.(value);
                setLinkValue("");
              }}
            >
              Add link
            </button>
          </div>
        </div>
        <div>
          <h3 className={styles.heading}>Runs</h3>
          <ul className={styles.list}>
            {runs.map((run) => (
              <li key={run.id} className={styles.historyItem}>
                <div>
                  {run.displayId} · {run.agent} ·{" "}
                  <span className={`${styles.runStatus} ${styles[run.status] ?? ""}`}>
                    {RUN_STATUS[run.status] ?? run.status}
                  </span>{" "}
                  · {relativeTime(run.endedAt || run.startedAt)}
                </div>
                {run.message ? <div className={styles.historyMeta}>{run.message}</div> : null}
                {run.waitingReason ? (
                  <div className={styles.historyMeta}>{run.waitingReason}</div>
                ) : null}
                {run.summary ? <div className={styles.historyMeta}>{run.summary}</div> : null}
              </li>
            ))}
          </ul>
        </div>
        <div>
          <h3 className={styles.heading}>Activity</h3>
          <ul className={styles.list}>
            {activities.map((item) => (
              <li key={item.id}>
                {activityLabel(item.operation)} · {item.actorLabel} ·{" "}
                {relativeTime(item.createdAt)}
              </li>
            ))}
          </ul>
        </div>
        {props.trashed ? (
          <button type="button" className={styles.delete} onClick={() => props.onRestore?.()}>
            Restore
          </button>
        ) : props.confirming ? (
          <ConfirmDialog
            message={`Delete ${props.task.title}?`}
            onConfirm={props.onDelete}
            onCancel={() => props.onCancelDelete?.()}
          />
        ) : (
          <button
            type="button"
            className={styles.delete}
            onClick={() => {
              if (props.onDeleteRequest) props.onDeleteRequest();
              else props.onDelete();
            }}
          >
            Delete
          </button>
        )}
    </aside>
  );
}
