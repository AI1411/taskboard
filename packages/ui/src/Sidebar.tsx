import type { Project } from "@taskboard/types";

import { EmptyState } from "./EmptyState";
import styles from "./Sidebar.module.css";

export function Sidebar(props: {
  projects: Project[];
  selectedSlug: string | null;
  includeArchived: boolean;
  projectNote: string;
  onNewProject: () => void;
  onSelectProject: (slug: string) => void;
  onToggleArchived: () => void;
  onProjectNoteChange: (markdown: string) => void;
  onTrash?: () => void;
}) {
  return (
    <aside className={styles.sidebar}>
      <div className={styles.masthead}>
        <h1 className={styles.wordmark}>Taskboard</h1>
        <div className={styles.header}>
          <span className={styles.brand}>Projects</span>
          <button type="button" className={styles.newProject} onClick={props.onNewProject}>
            New project
          </button>
        </div>
      </div>
      {props.projects.length === 0 ? (
        <EmptyState>Create a project to start a board</EmptyState>
      ) : (
        <ul className={styles.list}>
          {props.projects.map((project) => (
            <li key={project.id}>
              <button
                type="button"
                className={styles.project}
                aria-current={project.slug === props.selectedSlug}
                onClick={() => props.onSelectProject(project.slug)}
              >
                {project.name}
              </button>
            </li>
          ))}
        </ul>
      )}
      {props.selectedSlug ? (
        <>
          <p className={styles.selectedName}>
            {props.projects.find((p) => p.slug === props.selectedSlug)?.name}
          </p>
          <label className={styles.note}>
            Project note
            <textarea
              value={props.projectNote}
              onChange={(e) => props.onProjectNoteChange(e.target.value)}
            />
          </label>
        </>
      ) : null}
      <button
        type="button"
        className={styles.toggle}
        aria-pressed={props.includeArchived}
        onClick={props.onToggleArchived}
      >
        Archived projects
      </button>
      {props.onTrash ? (
        <button type="button" className={styles.trash} onClick={props.onTrash}>
          Trash
        </button>
      ) : null}
    </aside>
  );
}
