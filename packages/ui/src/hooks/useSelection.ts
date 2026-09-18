import { useCallback, useState, type MutableRefObject, type RefObject } from "react";
import type { Transport } from "@taskboard/client";
import type { Column, Project, TaskDetail, TaskSummary } from "@taskboard/types";

import { errorCode, errorMessage, isNotFound } from "../errors";
import { useLatestRef } from "./useLatestRef";

export type ToastState = {
  message: string;
  error?: boolean;
  action?: { label: string; onClick: () => void };
} | null;

export type UseSelectionArgs = {
  transport: Transport;
  setToast: (toast: ToastState) => void;
  boardRef: RefObject<HTMLElement | null>;
  refreshTasks: (slug: string) => Promise<TaskSummary[]>;
  refreshTrash: () => Promise<void>;
  selectedProjectRef: MutableRefObject<Project | null>;
  tasksRef: MutableRefObject<TaskSummary[]>;
  performUndo: () => Promise<void>;
};

export function useSelection({
  transport,
  setToast,
  boardRef,
  refreshTasks,
  refreshTrash,
  selectedProjectRef,
  tasksRef,
  performUndo,
}: UseSelectionArgs) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<TaskDetail | null>(null);
  const [currentColumn, setCurrentColumn] = useState<Column>("todo");
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [trashedSelection, setTrashedSelection] = useState(false);

  const selectedIdRef = useLatestRef(selectedId);
  const detailRef = useLatestRef(detail);
  const currentColumnRef = useLatestRef(currentColumn);
  const inspectorOpenRef = useLatestRef(inspectorOpen);
  const confirmDeleteRef = useLatestRef(confirmDelete);
  const trashedSelectionRef = useLatestRef(trashedSelection);

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
  }, [boardRef, selectedIdRef, confirmDeleteRef, inspectorOpenRef]);

  const mutateSelected = useCallback(
    async (fn: (id: string) => Promise<void>, opts?: { refreshBoard?: boolean }) => {
      const id = selectedIdRef.current;
      if (!id) return;
      try {
        await fn(id);
        applyDetail(await transport.taskShow(id));
        if (opts?.refreshBoard) {
          const project = selectedProjectRef.current;
          if (project) await refreshTasks(project.slug);
        }
      } catch (err) {
        setToast({ message: errorMessage(err), error: true });
      }
    },
    [transport, refreshTasks, selectedIdRef, selectedProjectRef, setToast],
  );

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
    [
      transport,
      tasksRef,
      selectedIdRef,
      inspectorOpenRef,
      confirmDeleteRef,
      currentColumnRef,
      setToast,
    ],
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
  }, [transport, refreshTasks, selectedIdRef, tasksRef, detailRef, selectedProjectRef]);

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
    [
      transport,
      refreshTasks,
      selectedIdRef,
      currentColumnRef,
      tasksRef,
      selectedProjectRef,
      setToast,
    ],
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
          }
        } else {
          setToast({ message: errorMessage(err), error: true });
        }
      }
    },
    [transport, selectedIdRef, detailRef, setToast],
  );

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
          }
        } else {
          setToast({ message: errorMessage(err), error: true });
        }
      }
    },
    [transport, refreshTasks, selectedIdRef, detailRef, selectedProjectRef, setToast],
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
          }
        } else {
          setToast({ message: errorMessage(err), error: true });
        }
      }
    },
    [transport, refreshTasks, selectedIdRef, detailRef, selectedProjectRef, setToast],
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
  }, [
    transport,
    refreshTasks,
    refreshTrash,
    performUndo,
    selectedIdRef,
    detailRef,
    confirmDeleteRef,
    trashedSelectionRef,
    selectedProjectRef,
    setToast,
  ]);

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
    [
      transport,
      refreshTrash,
      refreshTasks,
      selectCard,
      trashedSelectionRef,
      selectedProjectRef,
    ],
  );

  return {
    selectedId,
    setSelectedId,
    selectedIdRef,
    detail,
    detailRef,
    applyDetail,
    currentColumn,
    setCurrentColumn,
    currentColumnRef,
    inspectorOpen,
    setInspectorOpen,
    inspectorOpenRef,
    confirmDelete,
    setConfirmDelete,
    confirmDeleteRef,
    trashedSelection,
    setTrashedSelection,
    trashedSelectionRef,
    closeInspector,
    mutateSelected,
    selectCard,
    toggleUrgent,
    moveSelected,
    onNoteChange,
    onTitleCommit,
    onWorkspaceChange,
    onDelete,
    restoreTask,
  };
}

export type SelectionApi = ReturnType<typeof useSelection>;
