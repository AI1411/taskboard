import type { Trash } from "@taskboard/types";

import { EmptyState } from "./EmptyState";
import styles from "./TrashPanel.module.css";

export function TrashPanel(props: {
  trash: Trash;
  onClose: () => void;
  onRestoreTask: (displayId: string) => void;
  onRestoreProject: (slug: string) => void;
  onSelectTask: (displayId: string) => void;
}) {
  const empty = props.trash.projects.length === 0 && props.trash.tasks.length === 0;

  return (
    <>
      <button type="button" className={styles.backdrop} aria-label="Close trash" onClick={props.onClose} />
      <aside className={styles.panel} role="dialog" aria-label="Trash">
        <div className={styles.header}>
          <h2 className={styles.title}>Trash</h2>
          <button type="button" className={styles.close} aria-label="Close trash" onClick={props.onClose}>
            Close
          </button>
        </div>
        {empty ? (
          <EmptyState>Trash is empty</EmptyState>
        ) : (
          <>
            {props.trash.projects.length > 0 ? (
              <section>
                <h3 className={styles.heading}>Projects</h3>
                <ul className={styles.list}>
                  {props.trash.projects.map((project) => (
                    <li key={project.id} className={styles.row}>
                      <span>{project.name}</span>
                      <button
                        type="button"
                        className={styles.restore}
                        aria-label={`Restore ${project.slug}`}
                        onClick={() => props.onRestoreProject(project.slug)}
                      >
                        Restore
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}
            {props.trash.tasks.length > 0 ? (
              <section>
                <h3 className={styles.heading}>Tasks</h3>
                <ul className={styles.list}>
                  {props.trash.tasks.map((task) => (
                    <li key={task.id} className={styles.row}>
                      <button
                        type="button"
                        className={styles.item}
                        onClick={() => props.onSelectTask(task.displayId)}
                      >
                        {task.title}
                      </button>
                      <button
                        type="button"
                        className={styles.restore}
                        aria-label={`Restore ${task.displayId}`}
                        onClick={() => props.onRestoreTask(task.displayId)}
                      >
                        Restore
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}
          </>
        )}
      </aside>
    </>
  );
}
