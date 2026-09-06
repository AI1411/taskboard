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

import { resolveDragEnd } from "./boardDrag";
import { Card } from "./Card";
import { COLUMNS } from "./columns";
import { Search } from "./Search";
import styles from "./Board.module.css";

export function Board(props: {
  tasks: TaskSummary[];
  selectedId: string | null;
  currentColumn: Column;
  query: string;
  searchRef?: Ref<HTMLInputElement>;
  onQueryChange: (value: string) => void;
  onSelectCard: (displayId: string) => void;
  onMove: (displayId: string, column: Column) => void;
  onReorder: (displayId: string, beforeId: string) => void;
  onNewTask?: () => void;
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
  );
  const [activeId, setActiveId] = useState<string | null>(null);
  const [overlayWidth, setOverlayWidth] = useState<number>();
  const q = props.query;
  const visible = q
    ? props.tasks.filter((t) => t.title.toLowerCase().includes(q.toLowerCase()))
    : props.tasks;
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
    <section className={styles.board}>
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
              >
                <SortableContext
                  items={columnTasks.map((t) => t.displayId)}
                  strategy={verticalListSortingStrategy}
                >
                  {columnTasks.map((task) => (
                    <SortableCard
                      key={task.displayId}
                      task={task}
                      selected={props.selectedId === task.displayId}
                      onSelect={() => props.onSelectCard(task.displayId)}
                    />
                  ))}
                  {columnTasks.length === 0 ? <div className={styles.slot} /> : null}
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
  children: ReactNode;
}) {
  const { setNodeRef } = useDroppable({ id: props.column });
  return (
    <div className={`${styles.column} ${props.current ? styles.columnCurrent : ""}`}>
      <h2 className={styles.header}>
        {props.label}
        <span className={styles.count}>{props.count}</span>
      </h2>
      <div ref={setNodeRef} role="list" aria-label={props.label} className={styles.list}>
        {props.children}
      </div>
    </div>
  );
}

function SortableCard(props: {
  task: TaskSummary;
  selected: boolean;
  onSelect: () => void;
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
      style={{
        transform: isDragging ? undefined : CSS.Transform.toString(transform),
        transition: undefined,
      }}
      {...attributes}
      {...listeners}
    />
  );
}
