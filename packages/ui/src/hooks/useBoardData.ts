import { useCallback, useEffect, useRef, useState } from "react";
import type { Transport } from "@taskboard/client";
import type {
  BoardStatus,
  Column,
  InboxItem,
  OccupancyGroup,
  Project,
  TaskSummary,
  Trash,
} from "@taskboard/types";

import { errorCode, errorField, errorMessage, isNotFound } from "../errors";
import { useLatestRef } from "./useLatestRef";
import type { SelectionApi, ToastState } from "./useSelection";

export type UseBoardDataArgs = {
  transport: Transport;
  sequence?: number;
  setToast: (toast: ToastState) => void;
  setComposingTask: (value: boolean) => void;
  setComposingProject: (value: boolean) => void;
  /** Late-bound selection API; assigned by the shell after `useSelection`. */
  getSelection: () => SelectionApi;
};

export function useBoardData({
  transport,
  sequence,
  setToast,
  setComposingTask,
  setComposingProject,
  getSelection,
}: UseBoardDataArgs) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [tasks, setTasks] = useState<TaskSummary[]>([]);
  const [includeArchived, setIncludeArchived] = useState(false);
  const [projectNote, setProjectNote] = useState("");
  const [inboxItems, setInboxItems] = useState<InboxItem[]>([]);
  const [inboxAll, setInboxAll] = useState<InboxItem[]>([]);
  const [inboxExpanded, setInboxExpanded] = useState(false);
  const [inboxScope, setInboxScope] = useState<"this" | "all">("this");
  const [boardStatus, setBoardStatus] = useState<BoardStatus | null>(null);
  const [occupancy, setOccupancy] = useState<OccupancyGroup[]>([]);
  const [trash, setTrash] = useState<Trash>({ projects: [], tasks: [] });

  const selectedProjectRef = useLatestRef(selectedProject);
  const tasksRef = useLatestRef(tasks);
  const projectsRef = useLatestRef(projects);
  const includeArchivedRef = useLatestRef(includeArchived);
  const inboxScopeRef = useLatestRef(inboxScope);

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
  }, [transport, selectedProjectRef, inboxScopeRef]);

  const refreshTasks = useCallback(
    async (slug: string) => {
      const list = await transport.taskList(slug);
      setTasks(list);
      tasksRef.current = list;
      await refreshInbox();
      return list;
    },
    [transport, refreshInbox, tasksRef],
  );

  const applyProject = useCallback(
    async (project: Project) => {
      const selection = getSelection();
      selectedProjectRef.current = project;
      setSelectedProject(project);
      setProjectNote(project.noteMarkdown);
      selection.setSelectedId(null);
      selection.selectedIdRef.current = null;
      selection.applyDetail(null);
      selection.setInspectorOpen(false);
      selection.inspectorOpenRef.current = false;
      selection.setCurrentColumn("todo");
      selection.currentColumnRef.current = "todo";
      await refreshTasks(project.slug);
      void transport.uiStateSet(project.slug);
    },
    [refreshTasks, transport, getSelection, selectedProjectRef],
  );

  const reloadBoard = useCallback(async () => {
    const selection = getSelection();
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
        selection.setSelectedId(null);
        selection.selectedIdRef.current = null;
        selection.applyDetail(null);
      }
      return;
    }
    selectedProjectRef.current = nextProject;
    setSelectedProject(nextProject);
    setProjectNote(nextProject.noteMarkdown);
    const nextTasks = await refreshTasks(nextProject.slug);
    const id = selection.selectedIdRef.current;
    if (id && !nextTasks.some((t) => t.displayId === id)) {
      selection.setSelectedId(null);
      selection.selectedIdRef.current = null;
      selection.applyDetail(null);
      selection.setInspectorOpen(false);
      selection.inspectorOpenRef.current = false;
    } else if (id) {
      try {
        selection.applyDetail(await transport.taskShow(id));
      } catch (err) {
        if (isNotFound(err)) {
          selection.setSelectedId(null);
          selection.selectedIdRef.current = null;
          selection.applyDetail(null);
          selection.setInspectorOpen(false);
          selection.inspectorOpenRef.current = false;
        } else {
          setToast({ message: errorMessage(err), error: true });
        }
      }
    }
  }, [
    transport,
    applyProject,
    refreshTasks,
    getSelection,
    includeArchivedRef,
    projectsRef,
    selectedProjectRef,
    tasksRef,
    setToast,
  ]);

  const prevSequence = useRef(sequence);
  useEffect(() => {
    if (sequence === undefined) return;
    if (sequence === prevSequence.current) return;
    prevSequence.current = sequence;
    void reloadBoard();
  }, [sequence, reloadBoard]);

  const addProject = useCallback(
    async (name: string) => {
      const selection = getSelection();
      const project = await transport.projectAdd({ name });
      setComposingProject(false);
      selectedProjectRef.current = project;
      selection.selectedIdRef.current = null;
      setSelectedProject(project);
      selection.setSelectedId(null);
      selection.applyDetail(null);
      selection.setInspectorOpen(false);
      selection.inspectorOpenRef.current = false;
      setProjectNote(project.noteMarkdown);
      setProjects((prev) => {
        const next = prev.some((p) => p.id === project.id) ? prev : [...prev, project];
        projectsRef.current = next;
        return next;
      });
      await refreshTasks(project.slug);
      return project;
    },
    [
      transport,
      refreshTasks,
      getSelection,
      setComposingProject,
      selectedProjectRef,
      projectsRef,
    ],
  );

  const refreshTrash = useCallback(async () => {
    setTrash(await transport.trashList());
  }, [transport]);

  const createTask = useCallback(
    async (title: string) => {
      const selection = getSelection();
      const project = selectedProjectRef.current;
      if (!project) return;
      const created = await transport.taskCreate(project.slug, {
        title,
        column: selection.currentColumnRef.current,
      });
      setComposingTask(false);
      await refreshTasks(project.slug);
      selection.selectedIdRef.current = created.displayId;
      selection.setSelectedId(created.displayId);
      selection.applyDetail(created);
      selection.setInspectorOpen(true);
      selection.inspectorOpenRef.current = true;
    },
    [transport, refreshTasks, getSelection, setComposingTask, selectedProjectRef],
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
  }, [transport, reloadBoard, setToast]);

  const onMove = useCallback(
    async (displayId: string, column: Column) => {
      const selection = getSelection();
      const task = tasksRef.current.find((t) => t.displayId === displayId);
      setTasks((prev) => {
        const next = prev.map((item) =>
          item.displayId === displayId ? { ...item, column } : item,
        );
        tasksRef.current = next;
        return next;
      });
      const updated = await transport.taskMove(displayId, column, task?.revision);
      if (selection.selectedIdRef.current === displayId) {
        selection.applyDetail(updated);
      }
      const project = selectedProjectRef.current;
      if (project) await refreshTasks(project.slug);
    },
    [transport, refreshTasks, getSelection, tasksRef, selectedProjectRef],
  );

  const onReorder = useCallback(
    async (displayId: string, beforeId: string) => {
      const selection = getSelection();
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
      if (selection.selectedIdRef.current === displayId) {
        selection.applyDetail(updated);
      }
      const project = selectedProjectRef.current;
      if (project) await refreshTasks(project.slug);
    },
    [transport, refreshTasks, getSelection, tasksRef, selectedProjectRef],
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
  }, [projectNote, selectedProject, transport, selectedProjectRef]);

  const restoreProject = useCallback(
    async (slug: string) => {
      const project = await transport.projectRestore(slug);
      await refreshTrash();
      const list = await transport.projectList(includeArchivedRef.current);
      setProjects(list);
      projectsRef.current = list;
      await applyProject(project);
    },
    [transport, refreshTrash, applyProject, includeArchivedRef, projectsRef],
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
  }, [transport, applyProject, includeArchivedRef, selectedProjectRef]);

  const switchProject = useCallback(
    (dir: -1 | 1) => {
      const list = projectsRef.current;
      const current = selectedProjectRef.current;
      if (!current || list.length === 0) return;
      const i = list.findIndex((p) => p.slug === current.slug);
      const next = list[i + dir];
      if (next) void applyProject(next);
    },
    [applyProject, projectsRef, selectedProjectRef],
  );

  return {
    projects,
    setProjects,
    projectsRef,
    selectedProject,
    setSelectedProject,
    selectedProjectRef,
    tasks,
    setTasks,
    tasksRef,
    includeArchived,
    setIncludeArchived,
    includeArchivedRef,
    projectNote,
    setProjectNote,
    inboxItems,
    inboxAll,
    inboxExpanded,
    setInboxExpanded,
    inboxScope,
    setInboxScope,
    inboxScopeRef,
    boardStatus,
    occupancy,
    trash,
    refreshInbox,
    refreshTasks,
    applyProject,
    reloadBoard,
    addProject,
    refreshTrash,
    createTask,
    performUndo,
    onMove,
    onReorder,
    restoreProject,
    toggleArchived,
    switchProject,
  };
}
