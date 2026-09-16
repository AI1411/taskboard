import type { Project } from "@taskboard/types";
import { useEffect, useState, type Ref } from "react";

import { Composer } from "./Composer";
import { ConfirmDialog } from "./ConfirmDialog";
import { EmptyState } from "./EmptyState";
import styles from "./Sidebar.module.css";

export function Sidebar(props: {
  projects: Project[];
  selectedSlug: string | null;
  includeArchived: boolean;
  projectNote: string;
  inboxCounts?: Record<string, number>;
  onNewProject: () => void;
  onSelectProject: (slug: string) => void;
  onToggleArchived: () => void;
  onProjectNoteChange: (markdown: string) => void;
  onTrash?: () => void;
  composing?: boolean;
  composerRef?: Ref<HTMLInputElement>;
  onComposerSubmit?: (name: string) => void;
  onComposerCancel?: () => void;
  onRename?: (name: string) => void;
  onSetPath?: (path: string) => void;
  onArchive?: (archived: boolean) => void;
  onDeleteProject?: () => void;
  onReorder?: (slugs: string[]) => void;
}) {
  const selected = props.projects.find((p) => p.slug === props.selectedSlug);
  const [name, setName] = useState(selected?.name ?? "");
  const [menuOpen, setMenuOpen] = useState(false);
  const [editingPath, setEditingPath] = useState(false);
  const [pathValue, setPathValue] = useState(selected?.repoPath ?? "");
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  useEffect(() => {
    setName(selected?.name ?? "");
    setPathValue(selected?.repoPath ?? "");
    setMenuOpen(false);
    setEditingPath(false);
    setConfirmingDelete(false);
  }, [selected?.id, selected?.name, selected?.repoPath]);
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
        {props.composing ? (
          <Composer
            placeholder="Project name"
            inputRef={props.composerRef}
            onSubmit={(name) => props.onComposerSubmit?.(name)}
            onCancel={() => props.onComposerCancel?.()}
          />
        ) : null}
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
                aria-label={project.name}
                aria-current={project.slug === props.selectedSlug}
                onClick={() => props.onSelectProject(project.slug)}
              >
                {project.name}
                {(props.inboxCounts?.[project.slug] ?? 0) > 0 ? (
                  <span className={styles.inboxBadge} aria-hidden="true">
                    {props.inboxCounts?.[project.slug]}
                  </span>
                ) : null}
              </button>
            </li>
          ))}
        </ul>
      )}
      {selected ? (
        <>
          <div className={styles.selectedRow}>
            <input
              className={styles.selectedName}
              aria-label="Project name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              onBlur={() => {
                const next = name.trim();
                if (next && next !== selected.name) props.onRename?.(next);
              }}
            />
            <button
              type="button"
              className={styles.actions}
              aria-label="Project actions"
              aria-expanded={menuOpen}
              onClick={() => setMenuOpen((open) => !open)}
            >
              ⋯
            </button>
          </div>
          {menuOpen ? (
            <div className={styles.menu}>
              <button
                type="button"
                className={styles.menuItem}
                onClick={() => {
                  setEditingPath(true);
                  setMenuOpen(false);
                }}
              >
                Set repository path
              </button>
              <button
                type="button"
                className={styles.menuItem}
                onClick={() => {
                  props.onArchive?.(!selected.archived);
                  setMenuOpen(false);
                }}
              >
                {selected.archived ? "Unarchive" : "Archive"}
              </button>
              <button
                type="button"
                className={styles.menuItem}
                onClick={() => {
                  setConfirmingDelete(true);
                  setMenuOpen(false);
                }}
              >
                Delete project
              </button>
              <button
                type="button"
                className={styles.menuItem}
                onClick={() => {
                  const slugs = props.projects.map((p) => p.slug);
                  const i = slugs.indexOf(selected.slug);
                  if (i > 0) {
                    [slugs[i - 1], slugs[i]] = [slugs[i], slugs[i - 1]];
                    props.onReorder?.(slugs);
                  }
                  setMenuOpen(false);
                }}
              >
                Move up
              </button>
              <button
                type="button"
                className={styles.menuItem}
                onClick={() => {
                  const slugs = props.projects.map((p) => p.slug);
                  const i = slugs.indexOf(selected.slug);
                  if (i >= 0 && i < slugs.length - 1) {
                    [slugs[i], slugs[i + 1]] = [slugs[i + 1], slugs[i]];
                    props.onReorder?.(slugs);
                  }
                  setMenuOpen(false);
                }}
              >
                Move down
              </button>
            </div>
          ) : null}
          {editingPath ? (
            <input
              className={styles.pathInput}
              placeholder="Repository path"
              value={pathValue}
              onChange={(e) => setPathValue(e.target.value)}
              onBlur={() => {
                props.onSetPath?.(pathValue.trim());
                setEditingPath(false);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  props.onSetPath?.(pathValue.trim());
                  setEditingPath(false);
                }
              }}
            />
          ) : null}
          {confirmingDelete ? (
            <ConfirmDialog
              message={`Delete ${selected.name}?`}
              onConfirm={() => {
                setConfirmingDelete(false);
                props.onDeleteProject?.();
              }}
              onCancel={() => setConfirmingDelete(false)}
            />
          ) : null}
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
