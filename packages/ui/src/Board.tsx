import {
  DndContext,
  DragOverlay,
  PointerSensor,
  closestCenter,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { Column, TaskSummary } from "@taskboard/types";

import { useState, type ReactNode, type Ref } from "react";

import { Composer } from "./Composer";

import { resolveDragEnd } from "./boardDrag";
import { Card } from "./Card";
import { COLUMNS } from "./columns";
import { Search } from "./Search";
import { taskMatchesQuery } from "./searchMatch";
import styles from "./Board.module.css";

export function Board(props: {
  tasks: TaskSummary[];
  selectedId: string | null;
  currentColumn: Column;
  query: string;
  searchRef?: Ref<HTMLInputElement>;
  boardRef?: Ref<HTMLElement>;
  onQueryChange: (value: string) => void;
  onSelectCard: (displayId: string) => void;
  onMove: (displayId: string, column: Column) => void;
  onReorder: (displayId: string, beforeId: string) => void;
  onBackgroundClick?: () => void;
  onCopyId?: (displayId: string) => void;
  onNewTask?: () => void;
  composing?: boolean;
  composerRef?: Ref<HTMLInputElement>;
  onComposerSubmit?: (title: string) => void;
  onComposerCancel?: () => void;
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
  );
  const [activeId, setActiveId] = useState<string | null>(null);
  const [overlayWidth, setOverlayWidth] = useState<number>();
  const [doneCollapsed, setDoneCollapsed] = useState(false);
  const q = props.query;
  const visible = props.tasks.filter((t) => taskMatchesQuery(t, q));
  const emptyBoard = props.tasks.length === 0;
  const noMatches = Boolean(q.trim()) && visible.length === 0;
  const activeTask = activeId ? props.tasks.find((task) => task.displayId === activeId) : undefined;

  function handleDragStart(event: DragStartEvent) {
    setActiveId(String(event.active.id));
    setOverlayWidth(event.active.rect.current.initial?.width);
  }

  function clearDrag() {
    setActiveId(null);
    setOverlayWidth(undefined);
  }

  function handleDragEnd(event: DragEndEvent) {
    const action = resolveDragEnd(
      String(event.active.id),
      event.over ? String(event.over.id) : null,
      props.tasks,
    );
    if (action.kind === "move") props.onMove(action.displayId, action.column);
    if (action.kind === "reorder") props.onReorder(action.displayId, action.beforeId);
    clearDrag();
  }

  return (
    <section
      ref={props.boardRef}
      className={styles.board}
      aria-label="Board"
      tabIndex={-1}
    >
      <div className={styles.toolbar}>
        <div className={styles.searchWrap}>
          <Search value={props.query} onChange={props.onQueryChange} inputRef={props.searchRef} />
          <kbd className={styles.searchHint} aria-hidden="true">
            /
          </kbd>
        </div>
        {props.onNewTask ? (
          <button type="button" className={styles.newTask} onClick={props.onNewTask}>
            New task
          </button>
        ) : null}
      </div>
      {emptyBoard ? <p className={styles.banner}>No tasks — New task or press n</p> : null}
      {noMatches ? (
        <p className={styles.banner}>
          No matching tasks{" "}
          <kbd className={styles.hint}>Esc</kbd>
        </p>
      ) : null}
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragStart={handleDragStart}
        onDragCancel={clearDrag}
        onDragEnd={handleDragEnd}
      >
        <div className={styles.columns}>
          {COLUMNS.map((col) => {
            const columnTasks = visible.filter((t) => t.column === col.id);
            return (
              <ColumnDrop
                key={col.id}
                column={col.id}
                label={col.label}
                count={columnTasks.length}
                current={props.currentColumn === col.id}
                collapsed={col.id === "done" && doneCollapsed}
                onToggle={
                  col.id === "done" ? () => setDoneCollapsed((open) => !open) : undefined
                }
                onBackgroundClick={props.onBackgroundClick}
              >
                <SortableContext
                  items={columnTasks.map((t) => t.displayId)}
                  strategy={verticalListSortingStrategy}
                >
                  {props.composing && props.currentColumn === col.id ? (
                    <Composer
                      placeholder="Task title"
                      inputRef={props.composerRef}
                      onSubmit={(title) => props.onComposerSubmit?.(title)}
                      onCancel={() => props.onComposerCancel?.()}
                    />
                  ) : null}
                  {columnTasks.map((task) => (
                    <SortableCard
                      key={task.displayId}
                      task={task}
                      selected={props.selectedId === task.displayId}
                      onSelect={() => props.onSelectCard(task.displayId)}
                      onCopyId={props.onCopyId}
                    />
                  ))}
                  {columnTasks.length === 0 && !noMatches ? (
                    <div className={styles.slot} data-empty-slot="true" />
                  ) : null}
                </SortableContext>
              </ColumnDrop>
            );
          })}
        </div>
        <DragOverlay dropAnimation={null}>
          {activeTask ? (
            <Card
              task={activeTask}
              selected={props.selectedId === activeTask.displayId}
              lifted
              onCopyId={props.onCopyId}
              style={overlayWidth ? { width: overlayWidth } : undefined}
            />
          ) : null}
        </DragOverlay>
      </DndContext>
    </section>
  );
}

function ColumnDrop(props: {
  column: Column;
  label: string;
  count: number;
  current: boolean;
  collapsed?: boolean;
  onToggle?: () => void;
  onBackgroundClick?: () => void;
  children: ReactNode;
}) {
  const { setNodeRef } = useDroppable({ id: props.column });
  return (
    <div
      className={`${styles.column} ${props.current ? styles.columnCurrent : ""}`}
      onClick={(event) => {
        const target = event.target as HTMLElement;
        if (target.closest('[role="listitem"]')) return;
        if (target.closest("input, textarea, select, button, [contenteditable='true']")) return;
        props.onBackgroundClick?.();
      }}
    >
      <h2 className={styles.header}>
        {props.onToggle ? (
          <button
            type="button"
            className={styles.collapse}
            aria-expanded={!props.collapsed}
            onClick={props.onToggle}
          >
            {props.label}
          </button>
        ) : (
          props.label
        )}
        <span className={styles.count}>{props.count}</span>
      </h2>
      <div ref={setNodeRef} role="list" aria-label={props.label} className={styles.list}>
        {props.collapsed ? null : props.children}
      </div>
    </div>
  );
}

function SortableCard(props: {
  task: TaskSummary;
  selected: boolean;
  onSelect: () => void;
  onCopyId?: (displayId: string) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useSortable({
    id: props.task.displayId,
    animateLayoutChanges: () => false,
  });
  return (
    <Card
      ref={setNodeRef}
      task={props.task}
      selected={props.selected}
      grabbed={isDragging}
      onClick={props.onSelect}
      onCopyId={props.onCopyId}
      style={{
        transform: isDragging ? undefined : CSS.Transform.toString(transform),
        transition: undefined,
      }}
      {...attributes}
      {...listeners}
    />
  );
}
