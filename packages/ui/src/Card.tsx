import { forwardRef, type HTMLAttributes } from "react";
import type { TaskSummary } from "@taskboard/types";

import { BADGE_LABEL } from "./columns";
import styles from "./Card.module.css";

export type CardProps = {
  task: TaskSummary;
  selected: boolean;
  grabbed?: boolean;
} & HTMLAttributes<HTMLDivElement>;

export const Card = forwardRef<HTMLDivElement, CardProps>(function Card(
  { task, selected, grabbed, className, ...rest },
  ref,
) {
  const badge = BADGE_LABEL[task.displayStatus];
  const showMessage =
    (task.displayStatus === "running" || task.displayStatus === "waiting") && task.runMessage;
  const label = [task.title, task.displayId, task.urgent ? "Urgent" : null, badge]
    .filter(Boolean)
    .join(" ");

  return (
    <div
      ref={ref}
      className={[styles.card, className].filter(Boolean).join(" ")}
      {...rest}
      role="listitem"
      aria-label={label}
      aria-selected={selected}
      aria-grabbed={grabbed || undefined}
    >
      <span className={styles.title}>{task.title}</span>
      <span className={styles.displayId}>{task.displayId}</span>
      <span className={styles.meta}>
        {task.urgent ? <span className={styles.pip} aria-label="Urgent" /> : null}
        {badge ? (
          <span className={`${styles.badge} ${styles[task.displayStatus]}`}>{badge}</span>
        ) : null}
      </span>
      {showMessage ? <span className={styles.runMessage}>{task.runMessage}</span> : null}
    </div>
  );
});
