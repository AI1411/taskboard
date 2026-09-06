import {
  DndContext,
  PointerSensor,
  closestCenter,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { Column, TaskSummary } from "@taskboard/types";

import type { ReactNode, Ref } from "react";

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
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
  );
  const q = props.query;
  const visible = q
    ? props.tasks.filter((t) => t.title.toLowerCase().includes(q.toLowerCase()))
    : props.tasks;

  function handleDragEnd(event: DragEndEvent) {
    const action = resolveDragEnd(
      String(event.active.id),
      event.over ? String(event.over.id) : null,
      props.tasks,
    );
    if (action.kind === "move") props.onMove(action.displayId, action.column);
    if (action.kind === "reorder") props.onReorder(action.displayId, action.beforeId);
  }

  return (
    <section className={styles.board}>
      <div className={styles.toolbar}>
        <Search value={props.query} onChange={props.onQueryChange} inputRef={props.searchRef} />
      </div>
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
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
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: props.task.displayId,
  });
  return (
    <Card
      ref={setNodeRef}
      task={props.task}
      selected={props.selected}
      grabbed={isDragging}
      onClick={props.onSelect}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      {...attributes}
      {...listeners}
    />
  );
}
