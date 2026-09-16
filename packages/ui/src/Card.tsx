import { forwardRef, type HTMLAttributes } from "react";
import type { TaskSummary } from "@taskboard/types";

import { BADGE_LABEL } from "./columns";
import styles from "./Card.module.css";

export type CardProps = {
  task: TaskSummary;
  selected: boolean;
  grabbed?: boolean;
  /** Visual pickup clone that follows the pointer. */
  lifted?: boolean;
  onCopyId?: (displayId: string) => void;
} & HTMLAttributes<HTMLDivElement>;

export const Card = forwardRef<HTMLDivElement, CardProps>(function Card(
  { task, selected, grabbed, lifted, className, onCopyId, ...rest },
  ref,
) {
  const badge = task.stale ? "Stale" : BADGE_LABEL[task.displayStatus];
  const faceMessage =
    task.runMessage ||
    ((task.displayStatus === "waiting" || task.displayStatus === "failed") && task.waitingReason) ||
    null;
  const showMessage =
    (task.displayStatus === "running" ||
      task.displayStatus === "waiting" ||
      task.displayStatus === "failed") &&
    faceMessage;
  const label = [task.title, task.displayId, task.urgent ? "Urgent" : null, badge]
    .filter(Boolean)
    .join(" ");
  const placeholder = Boolean(grabbed && !lifted);

  return (
    <div
      ref={ref}
      className={[
        styles.card,
        placeholder ? styles.placeholder : null,
        lifted ? styles.lifted : null,
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      data-placeholder={placeholder || undefined}
      data-lifted={lifted || undefined}
      {...rest}
      role="listitem"
      aria-label={label}
      aria-selected={selected}
      aria-grabbed={grabbed || undefined}
      aria-hidden={lifted || undefined}
    >
      <span className={styles.title}>{task.title}</span>
      <button
        type="button"
        className={styles.displayId}
        aria-label={`Copy ${task.displayId}`}
        onPointerDown={(event) => event.stopPropagation()}
        onClick={(event) => {
          event.stopPropagation();
          onCopyId?.(task.displayId);
        }}
      >
        {task.displayId}
      </button>
      <span className={styles.meta}>
        {task.urgent ? <span className={styles.pip} aria-label="Urgent" /> : null}
        {badge ? (
          <span
            className={`${styles.badge} ${task.stale ? styles.stale : styles[task.displayStatus]}`}
          >
            {badge}
          </span>
        ) : null}
      </span>
      {showMessage ? <span className={styles.runMessage}>{faceMessage}</span> : null}
      {task.blockedBy.length > 0 ? (
        <span className={styles.runMessage}>Blocked by {task.blockedBy.join(", ")}</span>
      ) : null}
      {task.blocks.length > 0 ? (
        <span className={styles.runMessage}>Blocks {task.blocks.join(", ")}</span>
      ) : null}
    </div>
  );
});
