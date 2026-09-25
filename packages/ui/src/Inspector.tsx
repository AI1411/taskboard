import { useEffect, useRef, useState, type Ref } from "react";
import type { Column, ReviewAction, TaskDetail } from "@taskboard/types";

import { COLUMNS } from "./columns";
import { ConfirmDialog } from "./ConfirmDialog";
import { editDraft, followServer, freshDraft, keepMine, shouldCommit, type FieldDraft } from "./fieldDraft";
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
  "comment.add": "Commented",
  "check.add": "Check added",
  "check.toggle": "Check toggled",
  "check.remove": "Check removed",
  "comment.remove": "Comment removed",
  "task.review": "Review",
  "task.spawn": "Spawned",
  "run.cancel": "Run canceled",
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
  onCommentAdd?: (body: string, continueWaiting?: boolean) => void;
  onCommentRemoveLatest?: () => void;
  onCheckAdd?: (text: string) => void;
  onCheckToggle?: (displayId: string) => void;
  onCheckRemove?: (displayId: string) => void;
  onBlockedByAdd?: (displayId: string) => void;
  onRunCancel?: (runDisplayId: string) => void;
  onReview?: (action: ReviewAction, text: string) => void;
  onCopyId?: (displayId: string) => void;
  onClose?: () => void;
  onWorkspaceChange?: (patch: { worktreePath?: string; branch?: string }) => void;
  onSpawn?: (title: string) => void;
}) {
  const [title, setTitle] = useState<FieldDraft>(() => freshDraft(props.task?.title ?? ""));
  const [note, setNote] = useState<FieldDraft>(() => freshDraft(props.task?.noteMarkdown ?? ""));
  const [worktree, setWorktree] = useState<FieldDraft>(() =>
    freshDraft(props.task?.worktreePath ?? ""),
  );
  const [branch, setBranch] = useState<FieldDraft>(() => freshDraft(props.task?.branch ?? ""));
  const [spawnValue, setSpawnValue] = useState("");
  const [linkValue, setLinkValue] = useState("");
  const [commentValue, setCommentValue] = useState("");
  const [continueWaiting, setContinueWaiting] = useState(false);
  const [checkValue, setCheckValue] = useState("");
  const [blockedByValue, setBlockedByValue] = useState("");
  const [reviewValue, setReviewValue] = useState("");
  const lastId = useRef<string | null>(null);
  const panelRef = useRef<HTMLElement>(null);

  useEffect(() => {
    if (!props.task) {
      lastId.current = null;
      setTitle(freshDraft(""));
      setNote(freshDraft(""));
      setWorktree(freshDraft(""));
      setBranch(freshDraft(""));
      setSpawnValue("");
      setLinkValue("");
      setCommentValue("");
      setCheckValue("");
      setBlockedByValue("");
      setReviewValue("");
      return;
    }
    const switched = lastId.current !== props.task.displayId;
    lastId.current = props.task.displayId;
    if (switched) {
      setTitle(freshDraft(props.task.title));
      setNote(freshDraft(props.task.noteMarkdown));
      setWorktree(freshDraft(props.task.worktreePath ?? ""));
      setBranch(freshDraft(props.task.branch ?? ""));
      setSpawnValue("");
      setLinkValue("");
      setCommentValue("");
      setCheckValue("");
      setBlockedByValue("");
      setReviewValue("");
      return;
    }
    const task = props.task;
    setTitle((draft) => followServer(draft, task.title));
    setNote((draft) => followServer(draft, task.noteMarkdown));
    setWorktree((draft) => followServer(draft, task.worktreePath ?? ""));
    setBranch((draft) => followServer(draft, task.branch ?? ""));
  }, [props.task]);

  useEffect(() => {
    if (!shouldCommit(note)) return;
    const handle = window.setTimeout(() => {
      props.onNoteChange(note.value);
    }, 400);
    return () => window.clearTimeout(handle);
  }, [note, props.onNoteChange]);

  useEffect(() => {
    if (!props.open || !props.task) return;
    panelRef.current?.focus();
  }, [props.open, props.task?.displayId]);

  if (!props.open || !props.task) return null;
  const task = props.task;

  const runs = [...task.runs].sort((a, b) => (a.startedAt < b.startedAt ? 1 : -1));
  const activities = [...props.task.recentActivities].sort((a, b) => b.sequence - a.sequence);
  const conflict = title.conflict || note.conflict || worktree.conflict || branch.conflict;

  function acceptMine() {
    if (!props.task) return;
    if (title.conflict) {
      const next = keepMine(title, props.task.title);
      setTitle(next);
      if (shouldCommit(next)) props.onTitleCommit(next.value);
    }
    if (note.conflict) {
      const next = keepMine(note, props.task.noteMarkdown);
      setNote(next);
      if (shouldCommit(next)) props.onNoteChange(next.value);
    }
    if (worktree.conflict) {
      const next = keepMine(worktree, props.task.worktreePath ?? "");
      setWorktree(next);
      if (shouldCommit(next)) props.onWorkspaceChange?.({ worktreePath: next.value });
    }
    if (branch.conflict) {
      const next = keepMine(branch, props.task.branch ?? "");
      setBranch(next);
      if (shouldCommit(next)) props.onWorkspaceChange?.({ branch: next.value });
    }
  }

  function loadServer() {
    if (!props.task) return;
    if (title.conflict) setTitle(freshDraft(props.task.title));
    if (note.conflict) setNote(freshDraft(props.task.noteMarkdown));
    if (worktree.conflict) setWorktree(freshDraft(props.task.worktreePath ?? ""));
    if (branch.conflict) setBranch(freshDraft(props.task.branch ?? ""));
  }

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
            value={title.value}
            onChange={(e) => setTitle((draft) => editDraft(draft, e.target.value))}
            onBlur={() => {
              if (shouldCommit(title)) props.onTitleCommit(title.value);
            }}
          />
        </label>
        <button
          type="button"
          className={styles.displayId}
          aria-label={`Copy ${task.displayId}`}
          onClick={() => props.onCopyId?.(task.displayId)}
        >
          {props.task.displayId}
        </button>
        <label className={styles.field}>
          Worktree
          <input
            value={worktree.value}
            onChange={(e) => setWorktree((draft) => editDraft(draft, e.target.value))}
            onBlur={() => {
              if (shouldCommit(worktree)) props.onWorkspaceChange?.({ worktreePath: worktree.value });
            }}
          />
        </label>
        <label className={styles.field}>
          Branch
          <input
            value={branch.value}
            onChange={(e) => setBranch((draft) => editDraft(draft, e.target.value))}
            onBlur={() => {
              if (shouldCommit(branch)) props.onWorkspaceChange?.({ branch: branch.value });
            }}
          />
        </label>
        <div className={styles.linkAdd}>
          <input
            className={styles.linkInput}
            placeholder="Spawn title"
            value={spawnValue}
            onChange={(e) => setSpawnValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== "Enter") return;
              e.preventDefault();
              const title = spawnValue.trim();
              if (!title) return;
              props.onSpawn?.(title);
              setSpawnValue("");
            }}
          />
          <button
            type="button"
            className={styles.linkButton}
            onClick={() => {
              const title = spawnValue.trim();
              if (!title) return;
              props.onSpawn?.(title);
              setSpawnValue("");
            }}
          >
            Spawn
          </button>
        </div>
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
        {conflict ? (
          <div className={styles.conflict} role="status">
            Updated elsewhere
            <button type="button" className={styles.linkButton} onClick={acceptMine}>
              Keep mine
            </button>
            <button type="button" className={styles.linkButton} onClick={loadServer}>
              Load server
            </button>
          </div>
        ) : null}
        <label className={styles.field}>
          Note
          <textarea
            className={styles.note}
            value={note.value}
            onChange={(e) => setNote((draft) => editDraft(draft, e.target.value))}
          />
        </label>
        <div>
          <h3 className={styles.heading}>Checklist</h3>
          <ul className={styles.list}>
            {props.task.checks.map((check) => (
              <li key={check.id} className={styles.linkRow}>
                <label className={styles.switch}>
                  <input
                    type="checkbox"
                    checked={check.done}
                    onChange={() => props.onCheckToggle?.(check.displayId)}
                  />
                  {check.text}
                </label>
                <button
                  type="button"
                  className={styles.linkRemove}
                  aria-label={`Remove ${check.displayId}`}
                  onClick={() => props.onCheckRemove?.(check.displayId)}
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
          <div className={styles.linkAdd}>
            <input
              className={styles.linkInput}
              placeholder="Add a check"
              value={checkValue}
              onChange={(e) => setCheckValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key !== "Enter") return;
                e.preventDefault();
                const text = checkValue.trim();
                if (!text) return;
                props.onCheckAdd?.(text);
                setCheckValue("");
              }}
            />
            <button
              type="button"
              className={styles.linkButton}
              onClick={() => {
                const text = checkValue.trim();
                if (!text) return;
                props.onCheckAdd?.(text);
                setCheckValue("");
              }}
            >
              Add check
            </button>
          </div>
        </div>
        <div>
          <h3 className={styles.heading}>Comments</h3>
          <ul className={styles.list}>
            {task.comments.map((comment, index) => (
              <li key={comment.id} className={styles.linkRow}>
                <span>
                  {relativeTime(comment.createdAt)} · {comment.actorLabel} · {comment.body}
                </span>
                {index === task.comments.length - 1 ? (
                  <button
                    type="button"
                    className={styles.linkRemove}
                    aria-label="Remove latest comment"
                    onClick={() => props.onCommentRemoveLatest?.()}
                  >
                    Remove
                  </button>
                ) : null}
              </li>
            ))}
          </ul>
          <input
            className={styles.linkInput}
            placeholder="Add a comment"
            value={commentValue}
            onChange={(e) => setCommentValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== "Enter") return;
              e.preventDefault();
              const body = commentValue.trim();
              if (!body) return;
              props.onCommentAdd?.(body, continueWaiting);
              setCommentValue("");
            }}
          />
          <label className={styles.field}>
            <input
              type="checkbox"
              checked={continueWaiting}
              onChange={(e) => setContinueWaiting(e.target.checked)}
            />
            If Waiting, Continue
          </label>
        </div>
        {props.task.column === "in-review" ? (
          <div>
            <h3 className={styles.heading}>Review</h3>
            <input
              className={styles.linkInput}
              placeholder="Review comment"
              value={reviewValue}
              onChange={(e) => setReviewValue(e.target.value)}
            />
            <div className={styles.linkAdd}>
              <button
                type="button"
                className={styles.linkButton}
                onClick={() => {
                  const text = reviewValue.trim();
                  if (!text) return;
                  props.onReview?.("approve", text);
                  setReviewValue("");
                }}
              >
                Approve
              </button>
              <button
                type="button"
                className={styles.linkButton}
                onClick={() => {
                  const text = reviewValue.trim();
                  if (!text) return;
                  props.onReview?.("changes", text);
                  setReviewValue("");
                }}
              >
                Request changes
              </button>
            </div>
          </div>
        ) : null}
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
          <div className={styles.linkAdd}>
            <input
              className={styles.linkInput}
              placeholder="TASK-n"
              value={blockedByValue}
              onChange={(e) => setBlockedByValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key !== "Enter") return;
                e.preventDefault();
                const value = blockedByValue.trim();
                if (!value) return;
                props.onBlockedByAdd?.(value);
                setBlockedByValue("");
              }}
            />
            <button
              type="button"
              className={styles.linkButton}
              onClick={() => {
                const value = blockedByValue.trim();
                if (!value) return;
                props.onBlockedByAdd?.(value);
                setBlockedByValue("");
              }}
            >
              Add blocked-by
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
                {run.status === "running" || run.status === "waiting" ? (
                  <button
                    type="button"
                    className={styles.linkButton}
                    aria-label={`Cancel ${run.displayId}`}
                    onClick={() => props.onRunCancel?.(run.displayId)}
                  >
                    Cancel
                  </button>
                ) : null}
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
