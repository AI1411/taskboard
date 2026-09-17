import type { InboxItem } from "@taskboard/types";

import { BADGE_LABEL } from "./columns";
import styles from "./InboxStrip.module.css";

export function InboxStrip(props: {
  items: InboxItem[];
  expanded: boolean;
  scope: "this" | "all";
  onToggle: () => void;
  onScope: (scope: "this" | "all") => void;
  onSelect: (item: InboxItem) => void;
}) {
  if (props.items.length === 0) return null;

  const waiting = props.items.filter((item) => item.displayStatus === "waiting").length;
  const failed = props.items.filter((item) => item.displayStatus === "failed").length;
  const stale = props.items.filter(
    (item) =>
      item.stale && item.displayStatus !== "waiting" && item.displayStatus !== "failed",
  ).length;
  const review = props.items.filter(
    (item) =>
      item.column === "in-review" &&
      item.displayStatus !== "waiting" &&
      item.displayStatus !== "failed" &&
      item.displayStatus !== "running" &&
      !item.stale,
  ).length;
  const urgent = props.items.filter(
    (item) =>
      item.displayStatus !== "waiting" &&
      item.displayStatus !== "failed" &&
      !item.stale &&
      !(item.column === "in-review" && item.displayStatus !== "running"),
  ).length;
  const parts = [
    waiting ? `Waiting ${waiting}` : null,
    failed ? `Failed ${failed}` : null,
    stale ? `Stale ${stale}` : null,
    review ? `Review ${review}` : null,
    urgent ? `Urgent ${urgent}` : null,
  ].filter(Boolean);

  return (
    <section className={styles.strip} aria-label="Inbox">
      <div className={styles.header}>
        <button type="button" className={styles.toggle} onClick={props.onToggle}>
          {waiting > 0 ? <span className={styles.pip} aria-hidden="true" /> : null}
          Inbox · {props.items.length}
          {parts.length > 0 ? (
            <span className={styles.counts}>{parts.join(" · ")}</span>
          ) : null}
        </button>
        <div className={styles.scopes}>
          <button
            type="button"
            className={styles.scope}
            aria-pressed={props.scope === "this"}
            onClick={() => props.onScope("this")}
          >
            This project
          </button>
          <button
            type="button"
            className={styles.scope}
            aria-pressed={props.scope === "all"}
            onClick={() => props.onScope("all")}
          >
            All projects
          </button>
        </div>
      </div>
      {props.expanded ? (
        <ul className={styles.list}>
          {props.items.map((item) => {
            const badge = BADGE_LABEL[item.displayStatus];
            const label = [item.projectName, item.displayId, item.title, badge, item.reason]
              .filter(Boolean)
              .join(" · ");
            return (
              <li key={item.id}>
                <button
                  type="button"
                  className={styles.row}
                  onClick={() => props.onSelect(item)}
                >
                  {label}
                </button>
              </li>
            );
          })}
        </ul>
      ) : null}
    </section>
  );
}
