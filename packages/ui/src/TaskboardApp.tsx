import { useCallback, useRef, useState } from "react";
import type { Transport } from "@taskboard/client";

import { Board } from "./Board";
import { InboxStrip } from "./InboxStrip";
import { Inspector } from "./Inspector";
import { StatusStrip } from "./StatusStrip";
import { Sidebar } from "./Sidebar";
import { ShortcutLegend } from "./ShortcutLegend";
import { Toast } from "./Toast";
import { TrashPanel } from "./TrashPanel";
import { useBoardData } from "./hooks/useBoardData";
import { useBoardKeyboard } from "./hooks/useBoardKeyboard";
import { useLatestRef } from "./hooks/useLatestRef";
import { useRouteSync } from "./hooks/useRouteSync";
import { useSelection, type SelectionApi, type ToastState } from "./hooks/useSelection";
import styles from "./TaskboardApp.module.css";
import "./theme.css";

export type BoardRoute = {
  project?: string | null;
  task?: string | null;
};

export function TaskboardApp(props: {
  transport: Transport;
  sequence?: number;
  route?: BoardRoute;
  onRouteChange?: (route: { project: string | null; task: string | null }) => void;
}) {
  const { transport, sequence } = props;

  const [query, setQuery] = useState("");
  const [composingTask, setComposingTask] = useState(false);
  const [composingProject, setComposingProject] = useState(false);
  const [trashOpen, setTrashOpen] = useState(false);
  const [legendOpen, setLegendOpen] = useState(false);
  const [toast, setToast] = useState<ToastState>(null);

  const searchRef = useRef<HTMLInputElement>(null);
  const boardRef = useRef<HTMLElement>(null);
  const titleRef = useRef<HTMLInputElement>(null);
  const taskComposerRef = useRef<HTMLInputElement>(null);
  const projectComposerRef = useRef<HTMLInputElement>(null);
  const queryRef = useLatestRef(query);
  const trashOpenRef = useLatestRef(trashOpen);
  const legendOpenRef = useLatestRef(legendOpen);

  const selectionRef = useRef<SelectionApi>(null!);
  const getSelection = useCallback(() => selectionRef.current, []);

  const board = useBoardData({
    transport,
    sequence,
    setToast,
    setComposingTask,
    setComposingProject,
    getSelection,
  });

  const selection = useSelection({
    transport,
    setToast,
    boardRef,
    refreshTasks: board.refreshTasks,
    refreshTrash: board.refreshTrash,
    selectedProjectRef: board.selectedProjectRef,
    tasksRef: board.tasksRef,
    performUndo: board.performUndo,
  });
  selectionRef.current = selection;

  useRouteSync({
    transport,
    route: props.route,
    onRouteChange: props.onRouteChange,
    applyProject: board.applyProject,
    selectCard: selection.selectCard,
    selectedProject: board.selectedProject,
    selectedId: selection.selectedId,
    setProjects: board.setProjects,
    tasksRef: board.tasksRef,
  });

  useBoardKeyboard({
    searchRef,
    titleRef,
    queryRef,
    setQuery,
    legendOpenRef,
    setLegendOpen,
    confirmDeleteRef: selection.confirmDeleteRef,
    setConfirmDelete: selection.setConfirmDelete,
    trashOpenRef,
    setTrashOpen,
    selectedIdRef: selection.selectedIdRef,
    trashedSelectionRef: selection.trashedSelectionRef,
    inspectorOpenRef: selection.inspectorOpenRef,
    setInspectorOpen: selection.setInspectorOpen,
    selectedProjectRef: board.selectedProjectRef,
    currentColumnRef: selection.currentColumnRef,
    setCurrentColumn: selection.setCurrentColumn,
    tasksRef: board.tasksRef,
    closeInspector: selection.closeInspector,
    selectCard: selection.selectCard,
    toggleUrgent: selection.toggleUrgent,
    moveSelected: selection.moveSelected,
    performUndo: board.performUndo,
    switchProject: board.switchProject,
    setComposingProject,
    setComposingTask,
    setInboxExpanded: board.setInboxExpanded,
    setToast,
  });

  const dismissToast = useCallback(() => setToast(null), []);

  const copyId = useCallback(async (displayId: string) => {
    await navigator.clipboard.writeText(displayId);
    setToast({ message: "Copied" });
  }, []);

  return (
    <div className={styles.app}>
      <Sidebar
        projects={board.projects}
        selectedSlug={board.selectedProject?.slug ?? null}
        includeArchived={board.includeArchived}
        projectNote={board.projectNote}
        inboxCounts={board.inboxAll.reduce<Record<string, number>>((counts, item) => {
          counts[item.projectSlug] = (counts[item.projectSlug] ?? 0) + 1;
          return counts;
        }, {})}
        composing={composingProject}
        composerRef={projectComposerRef}
        onComposerSubmit={(name) => void board.addProject(name)}
        onComposerCancel={() => setComposingProject(false)}
        onNewProject={() => setComposingProject(true)}
        onSelectProject={(slug) => {
          const project = board.projectsRef.current.find((p) => p.slug === slug);
          if (project) void board.applyProject(project);
        }}
        onToggleArchived={() => void board.toggleArchived()}
        onProjectNoteChange={board.setProjectNote}
        onRename={(name) => {
          const current = board.selectedProjectRef.current;
          if (!current) return;
          void transport.projectUpdate(current.slug, { name }, current.revision).then((updated) => {
            board.selectedProjectRef.current = updated;
            board.setSelectedProject(updated);
            board.setProjects((prev) => prev.map((p) => (p.id === updated.id ? updated : p)));
          });
        }}
        onSetPath={(path) => {
          const current = board.selectedProjectRef.current;
          if (!current) return;
          void transport
            .projectUpdate(current.slug, { repoPath: path || null }, current.revision)
            .then((updated) => {
              board.selectedProjectRef.current = updated;
              board.setSelectedProject(updated);
              board.setProjects((prev) => prev.map((p) => (p.id === updated.id ? updated : p)));
            });
        }}
        onArchive={(archived) => {
          const current = board.selectedProjectRef.current;
          if (!current) return;
          void transport.projectArchive(current.slug, archived, current.revision).then(() => board.reloadBoard());
        }}
        onDeleteProject={() => {
          const current = board.selectedProjectRef.current;
          if (!current) return;
          void transport.projectDelete(current.slug, current.revision).then(() => {
            void board.refreshTrash();
            void board.reloadBoard();
          });
        }}
        onReorder={(slugs) => {
          void transport.projectReorder(slugs).then((list) => {
            board.setProjects(list);
            board.projectsRef.current = list;
          });
        }}
        onTrash={() => {
          void board.refreshTrash().then(() => setTrashOpen(true));
        }}
      />
      <main className={styles.main}>
        {board.selectedProject ? (
          <>
          <StatusStrip
            status={board.boardStatus}
            occupancy={board.occupancy}
            onSelect={(displayId) => void selection.selectCard(displayId)}
          />
          <InboxStrip
            items={board.inboxItems}
            expanded={board.inboxExpanded}
            scope={board.inboxScope}
            onToggle={() => board.setInboxExpanded((open) => !open)}
            onScope={(scope) => {
              board.inboxScopeRef.current = scope;
              board.setInboxScope(scope);
              void board.refreshInbox();
            }}
            onSelect={(item) => {
              void (async () => {
                const project = board.projectsRef.current.find((p) => p.slug === item.projectSlug);
                if (project && project.slug !== board.selectedProjectRef.current?.slug) {
                  await board.applyProject(project);
                }
                await selection.selectCard(item.displayId);
              })();
            }}
          />
          <Board
            tasks={board.tasks}
            selectedId={selection.selectedId}
            currentColumn={selection.currentColumn}
            query={query}
            searchRef={searchRef}
            boardRef={boardRef}
            onQueryChange={(value) => {
              setQuery(value);
              queryRef.current = value;
            }}
            onSelectCard={(id) => void selection.selectCard(id)}
            onMove={(id, column) => void board.onMove(id, column)}
            onReorder={(id, beforeId) => void board.onReorder(id, beforeId)}
            onBackgroundClick={selection.closeInspector}
            onCopyId={copyId}
            composing={composingTask}
            composerRef={taskComposerRef}
            onComposerSubmit={(title) => void board.createTask(title)}
            onComposerCancel={() => setComposingTask(false)}
            onNewTask={() => setComposingTask(true)}
          />
          </>
        ) : null}
      </main>
      {trashOpen ? (
        <TrashPanel
          trash={board.trash}
          onClose={() => setTrashOpen(false)}
          onRestoreTask={(displayId) => void selection.restoreTask(displayId)}
          onRestoreProject={(slug) => void board.restoreProject(slug)}
          onSelectTask={(displayId) => void selection.selectCard(displayId, true)}
        />
      ) : null}
      <Inspector
        task={selection.detail}
        open={selection.inspectorOpen}
        titleRef={titleRef}
        trashed={selection.trashedSelection}
        confirming={selection.confirmDelete}
        onTitleCommit={(title) => void selection.onTitleCommit(title)}
        onWorkspaceChange={(patch) => void selection.onWorkspaceChange(patch)}
        onSpawn={(title) => {
          void selection.mutateSelected(async (id) => {
            await transport.taskSpawn(id, [title]);
          }, { refreshBoard: true });
        }}
        onColumnChange={(column) => void selection.moveSelected(column)}
        onUrgentChange={(urgent) => {
          void selection.mutateSelected(async (id) => {
            await transport.taskUrgent(id, urgent, selection.detailRef.current?.revision);
          }, { refreshBoard: true });
        }}
        onNoteChange={selection.onNoteChange}
        onDeleteRequest={() => {
          selection.setConfirmDelete(true);
          selection.confirmDeleteRef.current = true;
        }}
        onDelete={() => void selection.onDelete()}
        onCancelDelete={() => {
          selection.setConfirmDelete(false);
          selection.confirmDeleteRef.current = false;
        }}
        onRestore={() => {
          const id = selection.selectedIdRef.current;
          if (id) void selection.restoreTask(id);
        }}
        onLinkAdd={(value) => {
          const kind = value.includes("://") ? "url" : "path";
          void selection.mutateSelected(async (id) => {
            await transport.linkAdd(id, { kind, value });
          });
        }}
        onLinkRemove={(linkId) => {
          void selection.mutateSelected(async () => {
            await transport.linkRemove(linkId);
          });
        }}
        onCommentAdd={(body, continueWaiting) => {
          void selection.mutateSelected(async (id) => {
            await transport.commentAdd(id, body, continueWaiting);
          });
        }}
        onCommentRemoveLatest={() => {
          void selection.mutateSelected(async (id) => {
            await transport.commentRemoveLatest(id);
          });
        }}
        onCheckAdd={(text) => {
          void selection.mutateSelected(async (id) => {
            await transport.checkAdd(id, text);
          }, { refreshBoard: true });
        }}
        onCheckToggle={(checkId) => {
          void selection.mutateSelected(async () => {
            await transport.checkToggle(checkId);
          }, { refreshBoard: true });
        }}
        onCheckRemove={(checkId) => {
          void selection.mutateSelected(async () => {
            await transport.checkRemove(checkId);
          }, { refreshBoard: true });
        }}
        onReview={(action, text) => {
          void selection.mutateSelected(async (id) => {
            await transport.review(id, { action, text });
          }, { refreshBoard: true });
        }}
        onRunCancel={(runDisplayId) => {
          void selection.mutateSelected(async () => {
            await transport.runPatch(runDisplayId, { op: "cancel" });
          }, { refreshBoard: true });
        }}
        onBlockedByAdd={(value) => {
          void selection.mutateSelected(async (id) => {
            await transport.linkAdd(id, { kind: "blocked_by", value });
          }, { refreshBoard: true });
        }}
        onCopyId={copyId}
        onClose={selection.closeInspector}
      />
      {legendOpen ? <ShortcutLegend onClose={() => setLegendOpen(false)} /> : null}
      {toast ? (
        <Toast
          message={toast.message}
          error={toast.error}
          action={toast.action}
          onDismiss={dismissToast}
        />
      ) : null}
    </div>
  );
}
