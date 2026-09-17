import { useCallback, useEffect, useRef, useState } from "react";
import type { Transport } from "@taskboard/client";
import type {
  BoardStatus,
  Column,
  InboxItem,
  OccupancyGroup,
  Project,
  TaskDetail,
  TaskSummary,
  Trash,
} from "@taskboard/types";

import { Board } from "./Board";
import { InboxStrip } from "./InboxStrip";
import { Inspector } from "./Inspector";
import { StatusStrip } from "./StatusStrip";
import { Sidebar } from "./Sidebar";
import { ShortcutLegend } from "./ShortcutLegend";
import { Toast } from "./Toast";
import { TrashPanel } from "./TrashPanel";
import { COLUMNS, COLUMN_IDS, neighborColumn } from "./columns";
import { errorCode, errorField, errorMessage, isNotFound } from "./errors";
import { taskMatchesQuery } from "./searchMatch";
import styles from "./TaskboardApp.module.css";
import "./theme.css";

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable;
}

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
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [tasks, setTasks] = useState<TaskSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<TaskDetail | null>(null);
  const [currentColumn, setCurrentColumn] = useState<Column>("todo");
  const [query, setQuery] = useState("");
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [includeArchived, setIncludeArchived] = useState(false);
  const [projectNote, setProjectNote] = useState("");
  const [inboxItems, setInboxItems] = useState<InboxItem[]>([]);
  const [inboxAll, setInboxAll] = useState<InboxItem[]>([]);
  const [inboxExpanded, setInboxExpanded] = useState(false);
  const [inboxScope, setInboxScope] = useState<"this" | "all">("this");
  const [boardStatus, setBoardStatus] = useState<BoardStatus | null>(null);
  const [occupancy, setOccupancy] = useState<OccupancyGroup[]>([]);
  const [composingTask, setComposingTask] = useState(false);
  const [composingProject, setComposingProject] = useState(false);
  const [trashOpen, setTrashOpen] = useState(false);
  const [trash, setTrash] = useState<Trash>({ projects: [], tasks: [] });
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [trashedSelection, setTrashedSelection] = useState(false);
  const [legendOpen, setLegendOpen] = useState(false);
  const [toast, setToast] = useState<{
    message: string;
    error?: boolean;
    action?: { label: string; onClick: () => void };
  } | null>(null);

  const searchRef = useRef<HTMLInputElement>(null);
  const boardRef = useRef<HTMLElement>(null);
  const titleRef = useRef<HTMLInputElement>(null);
  const taskComposerRef = useRef<HTMLInputElement>(null);
  const projectComposerRef = useRef<HTMLInputElement>(null);
  const selectedProjectRef = useRef<Project | null>(null);
  const selectedIdRef = useRef<string | null>(null);
  const tasksRef = useRef<TaskSummary[]>([]);
  const currentColumnRef = useRef<Column>("todo");
  const queryRef = useRef("");
  const inspectorOpenRef = useRef(false);
  const projectsRef = useRef<Project[]>([]);
  const detailRef = useRef<TaskDetail | null>(null);
  const includeArchivedRef = useRef(false);
  const inboxScopeRef = useRef<"this" | "all">("this");
  const confirmDeleteRef = useRef(false);
  const trashOpenRef = useRef(false);
  const trashedSelectionRef = useRef(false);
  const legendOpenRef = useRef(false);

  selectedProjectRef.current = selectedProject;
  selectedIdRef.current = selectedId;
  tasksRef.current = tasks;
  currentColumnRef.current = currentColumn;
  queryRef.current = query;
  inspectorOpenRef.current = inspectorOpen;
  projectsRef.current = projects;
  includeArchivedRef.current = includeArchived;
  inboxScopeRef.current = inboxScope;
  confirmDeleteRef.current = confirmDelete;
  trashOpenRef.current = trashOpen;
  trashedSelectionRef.current = trashedSelection;
  legendOpenRef.current = legendOpen;

  const applyDetail = (updated: TaskDetail | null) => {
    detailRef.current = updated;
    setDetail(updated);
  };

  const closeInspector = useCallback(() => {
    setSelectedId(null);
    selectedIdRef.current = null;
    applyDetail(null);
    setConfirmDelete(false);
    confirmDeleteRef.current = false;
    setTrashedSelection(false);
    setInspectorOpen(false);
    inspectorOpenRef.current = false;
    boardRef.current?.focus();
  }, []);

  const refreshInbox = useCallback(async () => {
    const all = await transport.inbox();
    setInboxAll(all);
    const slug = selectedProjectRef.current?.slug;
    if (inboxScopeRef.current === "this" && slug) {
      setInboxItems(await transport.inbox({ project: slug }));
    } else {
      setInboxItems(all);
    }
    const project = inboxScopeRef.current === "this" ? slug : undefined;
    setBoardStatus(await transport.status(project));
    setOccupancy(await transport.occupancy());
  }, [transport]);

  const refreshTasks = useCallback(
    async (slug: string) => {
      const list = await transport.taskList(slug);
      setTasks(list);
      tasksRef.current = list;
      await refreshInbox();
      return list;
    },
    [transport, refreshInbox],
  );

  const applyProject = useCallback(
    async (project: Project) => {
      selectedProjectRef.current = project;
      setSelectedProject(project);
      setProjectNote(project.noteMarkdown);
      setSelectedId(null);
      selectedIdRef.current = null;
      applyDetail(null);
      setInspectorOpen(false);
      inspectorOpenRef.current = false;
      setCurrentColumn("todo");
      currentColumnRef.current = "todo";
      await refreshTasks(project.slug);
      void transport.uiStateSet(project.slug);
    },
    [refreshTasks, transport],
  );

  const dismissToast = useCallback(() => setToast(null), []);

  const copyId = useCallback(async (displayId: string) => {
    await navigator.clipboard.writeText(displayId);
    setToast({ message: "Copied" });
  }, []);

  const reloadBoard = useCallback(async () => {
    const list = await transport.projectList(includeArchivedRef.current);
    setProjects(list);
    projectsRef.current = list;
    const current = selectedProjectRef.current;
    const nextProject = current
      ? list.find((p) => p.id === current.id || p.slug === current.slug)
      : undefined;
    if (!nextProject) {
      if (list[0]) await applyProject(list[0]);
      else {
        setSelectedProject(null);
        selectedProjectRef.current = null;
        setTasks([]);
        tasksRef.current = [];
        setSelectedId(null);
        selectedIdRef.current = null;
        applyDetail(null);
      }
      return;
    }
    selectedProjectRef.current = nextProject;
    setSelectedProject(nextProject);
    setProjectNote(nextProject.noteMarkdown);
    const nextTasks = await refreshTasks(nextProject.slug);
    const id = selectedIdRef.current;
    if (id && !nextTasks.some((t) => t.displayId === id)) {
      setSelectedId(null);
      selectedIdRef.current = null;
      applyDetail(null);
      setInspectorOpen(false);
      inspectorOpenRef.current = false;
    } else if (id) {
      try {
        applyDetail(await transport.taskShow(id));
      } catch (err) {
        if (isNotFound(err)) {
          setSelectedId(null);
          selectedIdRef.current = null;
          applyDetail(null);
          setInspectorOpen(false);
          inspectorOpenRef.current = false;
        } else {
          setToast({ message: errorMessage(err), error: true });
        }
      }
    }
  }, [transport, applyProject, refreshTasks]);

  const prevSequence = useRef(sequence);
  useEffect(() => {
    if (sequence === undefined) return;
    if (sequence === prevSequence.current) return;
    prevSequence.current = sequence;
    void reloadBoard();
  }, [sequence, reloadBoard]);

  const addProject = useCallback(
    async (name: string) => {
      const project = await transport.projectAdd({ name });
      setComposingProject(false);
      selectedProjectRef.current = project;
      selectedIdRef.current = null;
      setSelectedProject(project);
      setSelectedId(null);
      applyDetail(null);
      setInspectorOpen(false);
      inspectorOpenRef.current = false;
      setProjectNote(project.noteMarkdown);
      setProjects((prev) => {
        const next = prev.some((p) => p.id === project.id) ? prev : [...prev, project];
        projectsRef.current = next;
        return next;
      });
      await refreshTasks(project.slug);
      return project;
    },
    [transport, refreshTasks],
  );

  const refreshTrash = useCallback(async () => {
    setTrash(await transport.trashList());
  }, [transport]);

  const selectCard = useCallback(
    async (displayId: string, fromTrash = false) => {
      const task = tasksRef.current.find((t) => t.displayId === displayId);
      selectedIdRef.current = displayId;
      setSelectedId(displayId);
      setInspectorOpen(true);
      inspectorOpenRef.current = true;
      setConfirmDelete(false);
      confirmDeleteRef.current = false;
      setTrashedSelection(fromTrash);
      if (task) {
        setCurrentColumn(task.column);
        currentColumnRef.current = task.column;
      }
      try {
        const shown = await transport.taskShow(displayId);
        applyDetail(shown);
      } catch (err) {
        if (isNotFound(err)) {
          setSelectedId(null);
          selectedIdRef.current = null;
          applyDetail(null);
          setInspectorOpen(false);
          inspectorOpenRef.current = false;
        } else {
          setToast({ message: errorMessage(err), error: true });
        }
      }
    },
    [transport],
  );

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const list = await transport.projectList(false);
      if (cancelled) return;
      setProjects(list);
      let last: string | null = null;
      try {
        last = (await transport.uiState()).lastProjectSlug;
      } catch {
        last = null;
      }
      if (cancelled) return;
      const routedSlug = props.route?.project ?? null;
      const routed = routedSlug ? list.find((project) => project.slug === routedSlug) : undefined;
      const remembered = last ? list.find((project) => project.slug === last) : undefined;
      const next = routed ?? remembered ?? list[0];
      if (next) await applyProject(next);
      if (cancelled) return;
      const taskId = routed ? (props.route?.task ?? null) : null;
      if (taskId && tasksRef.current.some((task) => task.displayId === taskId)) {
        await selectCard(taskId);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [transport, applyProject, selectCard, props.route?.project, props.route?.task]);

  useEffect(() => {
    if (!selectedProject) return;
    props.onRouteChange?.({
      project: selectedProject.slug,
      task: selectedId,
    });
  }, [selectedProject, selectedId, props.onRouteChange]);

  const createTask = useCallback(
    async (title: string) => {
      const project = selectedProjectRef.current;
      if (!project) return;
      const created = await transport.taskCreate(project.slug, {
        title,
        column: currentColumnRef.current,
      });
      setComposingTask(false);
      await refreshTasks(project.slug);
      selectedIdRef.current = created.displayId;
      setSelectedId(created.displayId);
      applyDetail(created);
      setInspectorOpen(true);
      inspectorOpenRef.current = true;
    },
    [transport, refreshTasks],
  );

  const toggleUrgent = useCallback(async () => {
    const id = selectedIdRef.current;
    if (!id) return;
    const task = tasksRef.current.find((t) => t.displayId === id);
    const next = !(task?.urgent ?? detailRef.current?.urgent ?? false);
    const updated = await transport.taskUrgent(id, next, task?.revision ?? detailRef.current?.revision);
    applyDetail(updated);
    const project = selectedProjectRef.current;
    if (project) await refreshTasks(project.slug);
  }, [transport, refreshTasks]);

  const moveSelected = useCallback(
    async (column: Column): Promise<boolean> => {
      const id = selectedIdRef.current;
      if (!id) {
        setCurrentColumn(column);
        currentColumnRef.current = column;
        return false;
      }
      const task = tasksRef.current.find((t) => t.displayId === id);
      try {
        const updated = await transport.taskMove(id, column, task?.revision);
        applyDetail(updated);
        setCurrentColumn(column);
        currentColumnRef.current = column;
        const project = selectedProjectRef.current;
        if (project) await refreshTasks(project.slug);
        return true;
      } catch (err) {
        setToast({ message: errorMessage(err), error: true });
        return false;
      }
    },
    [transport, refreshTasks],
  );

  const performUndo = useCallback(async () => {
    try {
      await transport.undo();
      await reloadBoard();
      setToast({ message: "Undone" });
    } catch (err) {
      if (errorCode(err) === "undo_conflict") {
        setToast({
          message: `Cannot undo — ${errorField(err) ?? "entity"} changed`,
          error: true,
        });
      } else {
        setToast({ message: errorMessage(err), error: true });
      }
    }
  }, [transport, reloadBoard]);

  const selectInColumn = useCallback(
    (dir: 1 | -1) => {
      const col = currentColumnRef.current;
      const q = queryRef.current;
      const inCol = tasksRef.current.filter(
        (t) => t.column === col && taskMatchesQuery(t, q),
      );
      if (inCol.length === 0) return;
      const idx = inCol.findIndex((t) => t.displayId === selectedIdRef.current);
      let nextIdx: number;
      if (idx < 0) nextIdx = dir > 0 ? 0 : inCol.length - 1;
      else nextIdx = Math.max(0, Math.min(inCol.length - 1, idx + dir));
      const next = inCol[nextIdx];
      if (next) void selectCard(next.displayId);
    },
    [selectCard],
  );

  const jumpColumn = useCallback((index: number) => {
    const column = COLUMN_IDS[index];
    if (!column) return;
    setCurrentColumn(column);
    currentColumnRef.current = column;
  }, []);

  const switchProject = useCallback(
    (dir: -1 | 1) => {
      const list = projectsRef.current;
      const current = selectedProjectRef.current;
      if (!current || list.length === 0) return;
      const i = list.findIndex((p) => p.slug === current.slug);
      const next = list[i + dir];
      if (next) void applyProject(next);
    },
    [applyProject],
  );

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (isTypingTarget(e.target)) {
        if (e.key === "Escape" && e.target === searchRef.current) {
          setQuery("");
          queryRef.current = "";
          searchRef.current?.blur();
        }
        return;
      }

      if ((e.metaKey || e.ctrlKey) && (e.key === "z" || e.key === "Z")) {
        e.preventDefault();
        void performUndo();
        return;
      }

      if (e.key === "?") {
        e.preventDefault();
        setLegendOpen((open) => {
          legendOpenRef.current = !open;
          return !open;
        });
        return;
      }
      if (e.key === "/") {
        e.preventDefault();
        searchRef.current?.focus();
        return;
      }
      if (e.key === "Escape") {
        if (legendOpenRef.current) {
          setLegendOpen(false);
          legendOpenRef.current = false;
          return;
        }
        if (confirmDeleteRef.current) {
          setConfirmDelete(false);
          confirmDeleteRef.current = false;
          return;
        }
        if (trashOpenRef.current) {
          setTrashOpen(false);
          return;
        }
        if (queryRef.current) {
          setQuery("");
          queryRef.current = "";
          return;
        }
        closeInspector();
        return;
      }
      if (e.key === "Delete" || e.key === "Backspace") {
        if (!selectedIdRef.current || trashedSelectionRef.current) return;
        e.preventDefault();
        setInspectorOpen(true);
        inspectorOpenRef.current = true;
        setConfirmDelete(true);
        confirmDeleteRef.current = true;
        return;
      }
      if (e.key === "Enter") {
        const id = selectedIdRef.current;
        if (id) {
          setInspectorOpen(true);
          inspectorOpenRef.current = true;
          void selectCard(id);
        }
        return;
      }
      if (e.key === "p") {
        e.preventDefault();
        setComposingProject(true);
        return;
      }
      if (e.key === "n") {
        e.preventDefault();
        if (!selectedProjectRef.current) return;
        setComposingTask(true);
        return;
      }
      if (e.key === "u") {
        e.preventDefault();
        void toggleUrgent();
        return;
      }
      if (e.key === "j") {
        e.preventDefault();
        selectInColumn(1);
        return;
      }
      if (e.key === "k") {
        e.preventDefault();
        selectInColumn(-1);
        return;
      }
      if (e.key === "h" || e.key === "H" || e.key === "l" || e.key === "L") {
        e.preventDefault();
        const dir = e.key === "l" || e.key === "L" ? 1 : -1;
        const next = neighborColumn(currentColumnRef.current, dir);
        if (!next) return;
        if (e.shiftKey || !selectedIdRef.current) {
          setCurrentColumn(next);
          currentColumnRef.current = next;
          return;
        }
        void moveSelected(next).then((moved) => {
          if (!moved) return;
          const label = COLUMNS.find((column) => column.id === next)?.label ?? next;
          setToast({
            message: `Moved to ${label}`,
            action: { label: "Undo", onClick: () => void performUndo() },
          });
        });
        return;
      }
      if (e.key >= "1" && e.key <= "4") {
        e.preventDefault();
        jumpColumn(Number(e.key) - 1);
        return;
      }
      if (e.key === "[") {
        e.preventDefault();
        switchProject(-1);
        return;
      }
      if (e.key === "]") {
        e.preventDefault();
        switchProject(1);
        return;
      }
      if (e.key === "i") {
        e.preventDefault();
        setInboxExpanded((open) => !open);
        return;
      }
      if (e.key === "e") {
        e.preventDefault();
        setInspectorOpen(true);
        inspectorOpenRef.current = true;
        titleRef.current?.focus();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    addProject,
    createTask,
    jumpColumn,
    closeInspector,
    moveSelected,
    performUndo,
    reloadBoard,
    selectCard,
    selectInColumn,
    switchProject,
    toggleUrgent,
    transport,
  ]);

  const onMove = useCallback(
    async (displayId: string, column: Column) => {
      const task = tasksRef.current.find((t) => t.displayId === displayId);
      setTasks((prev) => {
        const next = prev.map((item) =>
          item.displayId === displayId ? { ...item, column } : item,
        );
        tasksRef.current = next;
        return next;
      });
      const updated = await transport.taskMove(displayId, column, task?.revision);
      if (selectedIdRef.current === displayId) {
        applyDetail(updated);
      }
      const project = selectedProjectRef.current;
      if (project) await refreshTasks(project.slug);
    },
    [transport, refreshTasks],
  );

  const onReorder = useCallback(
    async (displayId: string, beforeId: string) => {
      const task = tasksRef.current.find((t) => t.displayId === displayId);
      setTasks((prev) => {
        const next = [...prev];
        const from = next.findIndex((item) => item.displayId === displayId);
        if (from < 0) return prev;
        const [item] = next.splice(from, 1);
        const to = next.findIndex((other) => other.displayId === beforeId);
        next.splice(to < 0 ? next.length : to, 0, item);
        tasksRef.current = next;
        return next;
      });
      const updated = await transport.taskReorder(displayId, beforeId, task?.revision);
      if (selectedIdRef.current === displayId) {
        applyDetail(updated);
      }
      const project = selectedProjectRef.current;
      if (project) await refreshTasks(project.slug);
    },
    [transport, refreshTasks],
  );

  const onNoteChange = useCallback(
    async (markdown: string) => {
      const id = selectedIdRef.current;
      if (!id) return;
      try {
        const updated = await transport.taskNoteSet(id, markdown, detailRef.current?.revision);
        applyDetail(updated);
      } catch (err) {
        if (errorCode(err) === "revision_conflict") {
          setToast({ message: "Updated elsewhere", error: true });
          try {
            applyDetail(await transport.taskShow(id));
          } catch {
            /* keep current detail */
          }
        } else {
          setToast({ message: errorMessage(err), error: true });
        }
      }
    },
    [transport],
  );

  useEffect(() => {
    const project = selectedProject;
    if (!project) return;
    if (projectNote === project.noteMarkdown) return;
    const handle = window.setTimeout(() => {
      void transport.projectNoteSet(project.slug, projectNote, project.revision).then((updated) => {
        selectedProjectRef.current = updated;
        setSelectedProject(updated);
        setProjects((prev) => prev.map((p) => (p.id === updated.id ? updated : p)));
      });
    }, 400);
    return () => window.clearTimeout(handle);
  }, [projectNote, selectedProject, transport]);

  const onTitleCommit = useCallback(
    async (title: string) => {
      const id = selectedIdRef.current;
      if (!id) return;
      try {
        const updated = await transport.taskUpdate(id, { title }, detailRef.current?.revision);
        applyDetail(updated);
        const project = selectedProjectRef.current;
        if (project) await refreshTasks(project.slug);
      } catch (err) {
        if (errorCode(err) === "revision_conflict") {
          setToast({ message: "Updated elsewhere", error: true });
          try {
            applyDetail(await transport.taskShow(id));
          } catch {
            /* keep current detail */
          }
        } else {
          setToast({ message: errorMessage(err), error: true });
        }
      }
    },
    [transport, refreshTasks],
  );

  const onWorkspaceChange = useCallback(
    async (patch: { worktreePath?: string; branch?: string }) => {
      const id = selectedIdRef.current;
      if (!id) return;
      try {
        const updated = await transport.taskUpdate(id, patch, detailRef.current?.revision);
        applyDetail(updated);
        const project = selectedProjectRef.current;
        if (project) await refreshTasks(project.slug);
      } catch (err) {
        if (errorCode(err) === "revision_conflict") {
          setToast({ message: "Updated elsewhere", error: true });
          try {
            applyDetail(await transport.taskShow(id));
          } catch {
            /* keep current detail */
          }
        } else {
          setToast({ message: errorMessage(err), error: true });
        }
      }
    },
    [transport, refreshTasks],
  );

  const onDelete = useCallback(async () => {
    const id = selectedIdRef.current;
    if (!id) return;
    try {
      await transport.taskDelete(id, detailRef.current?.revision);
      setConfirmDelete(false);
      confirmDeleteRef.current = false;
      setTrashedSelection(false);
      trashedSelectionRef.current = false;
      setSelectedId(null);
      selectedIdRef.current = null;
      applyDetail(null);
      const project = selectedProjectRef.current;
      if (project) await refreshTasks(project.slug);
      await refreshTrash();
      setToast({
        message: `Deleted ${id}`,
        action: { label: "Undo", onClick: () => void performUndo() },
      });
    } catch (err) {
      setToast({ message: errorMessage(err), error: true });
    }
  }, [transport, refreshTasks, refreshTrash, performUndo]);

  const restoreTask = useCallback(
    async (displayId: string) => {
      await transport.taskRestore(displayId);
      setTrashedSelection(false);
      trashedSelectionRef.current = false;
      await refreshTrash();
      const project = selectedProjectRef.current;
      if (project) await refreshTasks(project.slug);
      await selectCard(displayId);
    },
    [transport, refreshTrash, refreshTasks, selectCard],
  );

  const restoreProject = useCallback(
    async (slug: string) => {
      const project = await transport.projectRestore(slug);
      await refreshTrash();
      const list = await transport.projectList(includeArchivedRef.current);
      setProjects(list);
      projectsRef.current = list;
      await applyProject(project);
    },
    [transport, refreshTrash, applyProject],
  );

  const toggleArchived = useCallback(async () => {
    const next = !includeArchivedRef.current;
    includeArchivedRef.current = next;
    setIncludeArchived(next);
    const list = await transport.projectList(next);
    setProjects(list);
    if (list[0]) await applyProject(list[0]);
    else {
      setSelectedProject(null);
      selectedProjectRef.current = null;
      setTasks([]);
    }
  }, [transport, applyProject]);

  return (
    <div className={styles.app}>
      <Sidebar
        projects={projects}
        selectedSlug={selectedProject?.slug ?? null}
        includeArchived={includeArchived}
        projectNote={projectNote}
        inboxCounts={inboxAll.reduce<Record<string, number>>((counts, item) => {
          counts[item.projectSlug] = (counts[item.projectSlug] ?? 0) + 1;
          return counts;
        }, {})}
        composing={composingProject}
        composerRef={projectComposerRef}
        onComposerSubmit={(name) => void addProject(name)}
        onComposerCancel={() => setComposingProject(false)}
        onNewProject={() => setComposingProject(true)}
        onSelectProject={(slug) => {
          const project = projectsRef.current.find((p) => p.slug === slug);
          if (project) void applyProject(project);
        }}
        onToggleArchived={() => void toggleArchived()}
        onProjectNoteChange={setProjectNote}
        onRename={(name) => {
          const current = selectedProjectRef.current;
          if (!current) return;
          void transport.projectUpdate(current.slug, { name }, current.revision).then((updated) => {
            selectedProjectRef.current = updated;
            setSelectedProject(updated);
            setProjects((prev) => prev.map((p) => (p.id === updated.id ? updated : p)));
          });
        }}
        onSetPath={(path) => {
          const current = selectedProjectRef.current;
          if (!current) return;
          void transport
            .projectUpdate(current.slug, { repoPath: path || null }, current.revision)
            .then((updated) => {
              selectedProjectRef.current = updated;
              setSelectedProject(updated);
              setProjects((prev) => prev.map((p) => (p.id === updated.id ? updated : p)));
            });
        }}
        onArchive={(archived) => {
          const current = selectedProjectRef.current;
          if (!current) return;
          void transport.projectArchive(current.slug, archived, current.revision).then(() => reloadBoard());
        }}
        onDeleteProject={() => {
          const current = selectedProjectRef.current;
          if (!current) return;
          void transport.projectDelete(current.slug, current.revision).then(() => {
            void refreshTrash();
            void reloadBoard();
          });
        }}
        onReorder={(slugs) => {
          void transport.projectReorder(slugs).then((list) => {
            setProjects(list);
            projectsRef.current = list;
          });
        }}
        onTrash={() => {
          void refreshTrash().then(() => setTrashOpen(true));
        }}
      />
      <main className={styles.main}>
        {selectedProject ? (
          <>
          <StatusStrip
            status={boardStatus}
            occupancy={occupancy}
            onSelect={(displayId) => void selectCard(displayId)}
          />
          <InboxStrip
            items={inboxItems}
            expanded={inboxExpanded}
            scope={inboxScope}
            onToggle={() => setInboxExpanded((open) => !open)}
            onScope={(scope) => {
              inboxScopeRef.current = scope;
              setInboxScope(scope);
              void refreshInbox();
            }}
            onSelect={(item) => {
              void (async () => {
                const project = projectsRef.current.find((p) => p.slug === item.projectSlug);
                if (project && project.slug !== selectedProjectRef.current?.slug) {
                  await applyProject(project);
                }
                await selectCard(item.displayId);
              })();
            }}
          />
          <Board
            tasks={tasks}
            selectedId={selectedId}
            currentColumn={currentColumn}
            query={query}
            searchRef={searchRef}
            boardRef={boardRef}
            onQueryChange={(value) => {
              setQuery(value);
              queryRef.current = value;
            }}
            onSelectCard={(id) => void selectCard(id)}
            onMove={(id, column) => void onMove(id, column)}
            onReorder={(id, beforeId) => void onReorder(id, beforeId)}
            onBackgroundClick={closeInspector}
            onCopyId={copyId}
            composing={composingTask}
            composerRef={taskComposerRef}
            onComposerSubmit={(title) => void createTask(title)}
            onComposerCancel={() => setComposingTask(false)}
            onNewTask={() => setComposingTask(true)}
          />
          </>
        ) : null}
      </main>
      {trashOpen ? (
        <TrashPanel
          trash={trash}
          onClose={() => setTrashOpen(false)}
          onRestoreTask={(displayId) => void restoreTask(displayId)}
          onRestoreProject={(slug) => void restoreProject(slug)}
          onSelectTask={(displayId) => void selectCard(displayId, true)}
        />
      ) : null}
      <Inspector
        task={detail}
        open={inspectorOpen}
        titleRef={titleRef}
        trashed={trashedSelection}
        confirming={confirmDelete}
        onTitleCommit={(title) => void onTitleCommit(title)}
        onWorkspaceChange={(patch) => void onWorkspaceChange(patch)}
        onSpawn={(title) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void (async () => {
            try {
              await transport.taskSpawn(id, [title]);
              const project = selectedProjectRef.current;
              if (project) await refreshTasks(project.slug);
              applyDetail(await transport.taskShow(id));
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
        onColumnChange={(column) => void moveSelected(column)}
        onUrgentChange={(urgent) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void transport.taskUrgent(id, urgent, detailRef.current?.revision).then(async (updated) => {
            applyDetail(updated);
            const project = selectedProjectRef.current;
            if (project) await refreshTasks(project.slug);
          });
        }}
        onNoteChange={onNoteChange}
        onDeleteRequest={() => {
          setConfirmDelete(true);
          confirmDeleteRef.current = true;
        }}
        onDelete={() => void onDelete()}
        onCancelDelete={() => {
          setConfirmDelete(false);
          confirmDeleteRef.current = false;
        }}
        onRestore={() => {
          const id = selectedIdRef.current;
          if (id) void restoreTask(id);
        }}
        onLinkAdd={(value) => {
          const id = selectedIdRef.current;
          if (!id) return;
          const kind = value.includes("://") ? "url" : "path";
          void transport.linkAdd(id, { kind, value }).then(applyDetail);
        }}
        onLinkRemove={(linkId) => {
          void transport.linkRemove(linkId).then(applyDetail);
        }}
        onCommentAdd={(body, continueWaiting) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void (async () => {
            try {
              await transport.commentAdd(id, body, continueWaiting);
              applyDetail(await transport.taskShow(id));
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
        onCommentRemoveLatest={() => {
          const id = selectedIdRef.current;
          if (!id) return;
          void (async () => {
            try {
              await transport.commentRemoveLatest(id);
              applyDetail(await transport.taskShow(id));
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
        onCheckAdd={(text) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void (async () => {
            try {
              await transport.checkAdd(id, text);
              applyDetail(await transport.taskShow(id));
              const project = selectedProjectRef.current;
              if (project) await refreshTasks(project.slug);
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
        onCheckToggle={(checkId) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void (async () => {
            try {
              await transport.checkToggle(checkId);
              applyDetail(await transport.taskShow(id));
              const project = selectedProjectRef.current;
              if (project) await refreshTasks(project.slug);
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
        onCheckRemove={(checkId) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void (async () => {
            try {
              await transport.checkRemove(checkId);
              applyDetail(await transport.taskShow(id));
              const project = selectedProjectRef.current;
              if (project) await refreshTasks(project.slug);
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
        onReview={(action, text) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void (async () => {
            try {
              applyDetail(await transport.review(id, { action, text }));
              const project = selectedProjectRef.current;
              if (project) await refreshTasks(project.slug);
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
        onRunCancel={(runDisplayId) => {
          void (async () => {
            try {
              await transport.runPatch(runDisplayId, { op: "cancel" });
              const id = selectedIdRef.current;
              if (id) applyDetail(await transport.taskShow(id));
              const project = selectedProjectRef.current;
              if (project) await refreshTasks(project.slug);
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
        onBlockedByAdd={(value) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void transport
            .linkAdd(id, { kind: "blocked_by", value })
            .then(async (updated) => {
              applyDetail(updated);
              const project = selectedProjectRef.current;
              if (project) await refreshTasks(project.slug);
            })
            .catch((err) => setToast({ message: errorMessage(err), error: true }));
        }}
        onCopyId={copyId}
        onClose={closeInspector}
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
