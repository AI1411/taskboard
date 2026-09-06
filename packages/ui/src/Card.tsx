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
} & HTMLAttributes<HTMLDivElement>;

export const Card = forwardRef<HTMLDivElement, CardProps>(function Card(
  { task, selected, grabbed, lifted, className, ...rest },
  ref,
) {
  const badge = BADGE_LABEL[task.displayStatus];
  const showMessage =
    (task.displayStatus === "running" || task.displayStatus === "waiting") && task.runMessage;
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
