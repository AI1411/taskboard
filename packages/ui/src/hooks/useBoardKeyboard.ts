import { useEffect, type MutableRefObject, type RefObject } from "react";
import type { Column, Project, TaskSummary } from "@taskboard/types";

import { COLUMNS, COLUMN_IDS, neighborColumn } from "../columns";
import { taskMatchesQuery } from "../searchMatch";
import type { ToastState } from "./useSelection";

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable;
}

export type UseBoardKeyboardArgs = {
  searchRef: RefObject<HTMLInputElement | null>;
  titleRef: RefObject<HTMLInputElement | null>;
  queryRef: MutableRefObject<string>;
  setQuery: (value: string) => void;
  legendOpenRef: MutableRefObject<boolean>;
  setLegendOpen: (value: boolean | ((open: boolean) => boolean)) => void;
  confirmDeleteRef: MutableRefObject<boolean>;
  setConfirmDelete: (value: boolean) => void;
  trashOpenRef: MutableRefObject<boolean>;
  setTrashOpen: (value: boolean) => void;
  selectedIdRef: MutableRefObject<string | null>;
  trashedSelectionRef: MutableRefObject<boolean>;
  inspectorOpenRef: MutableRefObject<boolean>;
  setInspectorOpen: (value: boolean) => void;
  selectedProjectRef: MutableRefObject<Project | null>;
  currentColumnRef: MutableRefObject<Column>;
  setCurrentColumn: (column: Column) => void;
  tasksRef: MutableRefObject<TaskSummary[]>;
  closeInspector: () => void;
  selectCard: (displayId: string, fromTrash?: boolean) => Promise<void>;
  toggleUrgent: () => Promise<void>;
  moveSelected: (column: Column) => Promise<boolean>;
  performUndo: () => Promise<void>;
  switchProject: (dir: -1 | 1) => void;
  setComposingProject: (value: boolean) => void;
  setComposingTask: (value: boolean) => void;
  setInboxExpanded: (value: boolean | ((open: boolean) => boolean)) => void;
  setToast: (toast: ToastState) => void;
};

export function useBoardKeyboard({
  searchRef,
  titleRef,
  queryRef,
  setQuery,
  legendOpenRef,
  setLegendOpen,
  confirmDeleteRef,
  setConfirmDelete,
  trashOpenRef,
  setTrashOpen,
  selectedIdRef,
  trashedSelectionRef,
  inspectorOpenRef,
  setInspectorOpen,
  selectedProjectRef,
  currentColumnRef,
  setCurrentColumn,
  tasksRef,
  closeInspector,
  selectCard,
  toggleUrgent,
  moveSelected,
  performUndo,
  switchProject,
  setComposingProject,
  setComposingTask,
  setInboxExpanded,
  setToast,
}: UseBoardKeyboardArgs) {
  useEffect(() => {
    function selectInColumn(dir: 1 | -1) {
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
    }

    function jumpColumn(index: number) {
      const column = COLUMN_IDS[index];
      if (!column) return;
      setCurrentColumn(column);
      currentColumnRef.current = column;
    }

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
    searchRef,
    titleRef,
    queryRef,
    setQuery,
    legendOpenRef,
    setLegendOpen,
    confirmDeleteRef,
    setConfirmDelete,
    trashOpenRef,
    setTrashOpen,
    selectedIdRef,
    trashedSelectionRef,
    inspectorOpenRef,
    setInspectorOpen,
    selectedProjectRef,
    currentColumnRef,
    setCurrentColumn,
    tasksRef,
    closeInspector,
    selectCard,
    toggleUrgent,
    moveSelected,
    performUndo,
    switchProject,
    setComposingProject,
    setComposingTask,
    setInboxExpanded,
    setToast,
  ]);
}
