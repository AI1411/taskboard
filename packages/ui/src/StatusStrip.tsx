import type { BoardStatus, OccupancyGroup } from "@taskboard/types";

import styles from "./StatusStrip.module.css";

export function StatusStrip(props: {
  status: BoardStatus | null;
  occupancy: OccupancyGroup[];
  onSelect: (displayId: string) => void;
}) {
  if (!props.status) return null;
  const groups = [
    { label: "inbox", count: props.status.inbox.total, head: props.status.inboxHead },
    { label: "Open", count: props.status.openRuns, head: props.status.openRunHead },
    { label: "Stale", count: props.status.stale, head: props.status.staleHead },
    { label: "Ready", count: props.status.ready, head: props.status.readyHead },
    { label: "In review", count: props.status.inReview, head: props.status.inReviewHead },
    { label: "Blocked", count: props.status.blocked, head: props.status.blockedHead },
  ];
  const parts = [
    props.status.inbox.waiting ? `waiting ${props.status.inbox.waiting}` : null,
    props.status.inbox.failed ? `failed ${props.status.inbox.failed}` : null,
    props.status.inbox.stale ? `stale ${props.status.inbox.stale}` : null,
    props.status.inbox.review ? `review ${props.status.inbox.review}` : null,
    props.status.inbox.urgent ? `urgent ${props.status.inbox.urgent}` : null,
  ].filter(Boolean);

  return (
    <section className={styles.strip} aria-label="Status">
      {groups.map((group) => (
        <button
          key={group.label}
          type="button"
          className={styles.stat}
          disabled={group.count === 0}
          onClick={() => {
            const id = group.head[0]?.displayId;
            if (id) props.onSelect(id);
          }}
        >
          {group.label} {group.count}
        </button>
      ))}
      {parts.length > 0 ? <span className={styles.parts}>{parts.join(" · ")}</span> : null}
      {props.occupancy.map((group) => (
        <button
          key={group.worktreePath}
          type="button"
          className={styles.occupancy}
          aria-label="Occupancy"
          onClick={() => {
            const id = group.runs[0]?.taskDisplayId;
            if (id) props.onSelect(id);
          }}
        >
          {group.worktreePath} · {group.runs.map((run) => run.taskDisplayId).join(", ")}
        </button>
      ))}
    </section>
  );
}
