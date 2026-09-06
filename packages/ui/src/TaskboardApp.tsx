import { useCallback, useEffect, useRef, useState } from "react";
import type { Transport } from "@taskboard/client";
import type { Column, Project, TaskDetail, TaskSummary } from "@taskboard/types";

import { Board } from "./Board";
import { Inspector } from "./Inspector";
import { Sidebar } from "./Sidebar";
import { COLUMN_IDS, neighborColumn } from "./columns";
import styles from "./TaskboardApp.module.css";
import "./theme.css";

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable;
}

export function TaskboardApp(props: { transport: Transport }) {
  const { transport } = props;
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [tasks, setTasks] = useState<TaskSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<TaskDetail | null>(null);
  const [currentColumn, setCurrentColumn] = useState<Column>("todo");
  const [query, setQuery] = useState("");
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [includeArchived, setIncludeArchived] = useState(false);
  const [projectNote, setProjectNote] = useState("");

  const searchRef = useRef<HTMLInputElement>(null);
  const titleRef = useRef<HTMLInputElement>(null);
  const pendingProject = useRef<Promise<Project> | null>(null);
  const selectedProjectRef = useRef<Project | null>(null);
  const selectedIdRef = useRef<string | null>(null);
  const tasksRef = useRef<TaskSummary[]>([]);
  const currentColumnRef = useRef<Column>("todo");
  const queryRef = useRef("");
  const inspectorOpenRef = useRef(true);
  const projectsRef = useRef<Project[]>([]);
  const detailRef = useRef<TaskDetail | null>(null);
  const includeArchivedRef = useRef(false);

  selectedProjectRef.current = selectedProject;
  selectedIdRef.current = selectedId;
  tasksRef.current = tasks;
  currentColumnRef.current = currentColumn;
  queryRef.current = query;
  inspectorOpenRef.current = inspectorOpen;
  projectsRef.current = projects;
  includeArchivedRef.current = includeArchived;

  const applyDetail = (updated: TaskDetail | null) => {
    detailRef.current = updated;
    setDetail(updated);
  };

  const refreshTasks = useCallback(
    async (slug: string) => {
      const list = await transport.taskList(slug);
      setTasks(list);
      tasksRef.current = list;
      return list;
    },
    [transport],
  );

  const applyProject = useCallback(
    async (project: Project) => {
      selectedProjectRef.current = project;
      setSelectedProject(project);
      setProjectNote(project.noteMarkdown);
      setSelectedId(null);
      selectedIdRef.current = null;
      applyDetail(null);
      setInspectorOpen(true);
      inspectorOpenRef.current = true;
      setCurrentColumn("todo");
      currentColumnRef.current = "todo";
      await refreshTasks(project.slug);
    },
    [refreshTasks],
  );

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const list = await transport.projectList(false);
      if (cancelled) return;
      setProjects(list);
      if (list[0]) await applyProject(list[0]);
    })();
    return () => {
      cancelled = true;
    };
  }, [transport, applyProject]);

  const addProject = useCallback(() => {
    const promise = (async () => {
      const project = await transport.projectAdd({ name: "Untitled" });
      selectedProjectRef.current = project;
      selectedIdRef.current = null;
      setSelectedProject(project);
      setSelectedId(null);
      applyDetail(null);
      setInspectorOpen(true);
      inspectorOpenRef.current = true;
      setProjectNote(project.noteMarkdown);
      setProjects((prev) => {
        const next = prev.some((p) => p.id === project.id) ? prev : [...prev, project];
        projectsRef.current = next;
        return next;
      });
      await refreshTasks(project.slug);
      return project;
    })();
    pendingProject.current = promise;
    return promise;
  }, [transport, refreshTasks]);

  const selectCard = useCallback(
    async (displayId: string) => {
      const task = tasksRef.current.find((t) => t.displayId === displayId);
      selectedIdRef.current = displayId;
      setSelectedId(displayId);
      setInspectorOpen(true);
      inspectorOpenRef.current = true;
      if (task) {
        setCurrentColumn(task.column);
        currentColumnRef.current = task.column;
      }
      const shown = await transport.taskShow(displayId);
      applyDetail(shown);
    },
    [transport],
  );

  const createTask = useCallback(async () => {
    if (pendingProject.current) {
      await pendingProject.current;
    }
    const project = selectedProjectRef.current;
    if (!project) return;
    const created = await transport.taskCreate(project.slug, {
      title: "Untitled",
      column: currentColumnRef.current,
    });
    await refreshTasks(project.slug);
    selectedIdRef.current = created.displayId;
    setSelectedId(created.displayId);
    applyDetail(created);
    setInspectorOpen(true);
    inspectorOpenRef.current = true;
  }, [transport, refreshTasks]);

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
    async (column: Column) => {
      const id = selectedIdRef.current;
      if (!id) {
        setCurrentColumn(column);
        currentColumnRef.current = column;
        return;
      }
      const task = tasksRef.current.find((t) => t.displayId === id);
      const updated = await transport.taskMove(id, column, task?.revision);
      applyDetail(updated);
      setCurrentColumn(column);
      currentColumnRef.current = column;
      const project = selectedProjectRef.current;
      if (project) await refreshTasks(project.slug);
    },
    [transport, refreshTasks],
  );

  const selectInColumn = useCallback(
    (dir: 1 | -1) => {
      const col = currentColumnRef.current;
      const q = queryRef.current;
      const inCol = tasksRef.current.filter(
        (t) => t.column === col && (!q || t.title.toLowerCase().includes(q.toLowerCase())),
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

      if (e.key === "/") {
        e.preventDefault();
        searchRef.current?.focus();
        return;
      }
      if (e.key === "Escape") {
        if (queryRef.current) {
          setQuery("");
          queryRef.current = "";
          return;
        }
        setInspectorOpen(false);
        inspectorOpenRef.current = false;
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
        void addProject();
        return;
      }
      if (e.key === "n") {
        e.preventDefault();
        void createTask();
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
      if (e.key === "h" || e.key === "l") {
        e.preventDefault();
        const dir = e.key === "l" ? 1 : -1;
        const next = neighborColumn(currentColumnRef.current, dir);
        if (!next) return;
        if (selectedIdRef.current) void moveSelected(next);
        else {
          setCurrentColumn(next);
          currentColumnRef.current = next;
        }
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
      if (e.key === "e") {
        e.preventDefault();
        setInspectorOpen(true);
        inspectorOpenRef.current = true;
        titleRef.current?.focus();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [addProject, createTask, jumpColumn, moveSelected, selectCard, selectInColumn, switchProject, toggleUrgent]);

  const onMove = useCallback(
    async (displayId: string, column: Column) => {
      const task = tasksRef.current.find((t) => t.displayId === displayId);
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
      await transport.taskReorder(displayId, beforeId, task?.revision);
      const project = selectedProjectRef.current;
      if (project) await refreshTasks(project.slug);
    },
    [transport, refreshTasks],
  );

  const onNoteChange = useCallback(
    async (markdown: string) => {
      const id = selectedIdRef.current;
      if (!id) return;
      const updated = await transport.taskNoteSet(id, markdown, detailRef.current?.revision);
      applyDetail(updated);
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
      const updated = await transport.taskUpdate(id, { title }, detailRef.current?.revision);
      applyDetail(updated);
      const project = selectedProjectRef.current;
      if (project) await refreshTasks(project.slug);
    },
    [transport, refreshTasks],
  );

  const onDelete = useCallback(async () => {
    const id = selectedIdRef.current;
    if (!id) return;
    await transport.taskDelete(id, detailRef.current?.revision);
    setSelectedId(null);
    selectedIdRef.current = null;
    applyDetail(null);
    const project = selectedProjectRef.current;
    if (project) await refreshTasks(project.slug);
  }, [transport, refreshTasks]);

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
    <div className={`${styles.app} ${inspectorOpen ? "" : styles.collapsed}`}>
      <Sidebar
        projects={projects}
        selectedSlug={selectedProject?.slug ?? null}
        includeArchived={includeArchived}
        projectNote={projectNote}
        onNewProject={() => void addProject()}
        onSelectProject={(slug) => {
          const project = projectsRef.current.find((p) => p.slug === slug);
          if (project) void applyProject(project);
        }}
        onToggleArchived={() => void toggleArchived()}
        onProjectNoteChange={setProjectNote}
        onTrash={() => void transport.trashList()}
      />
      <main className={styles.main}>
        {selectedProject ? (
          <Board
            tasks={tasks}
            selectedId={selectedId}
            currentColumn={currentColumn}
            query={query}
            searchRef={searchRef}
            onQueryChange={(value) => {
              setQuery(value);
              queryRef.current = value;
            }}
            onSelectCard={(id) => void selectCard(id)}
            onMove={(id, column) => void onMove(id, column)}
            onReorder={(id, beforeId) => void onReorder(id, beforeId)}
          />
        ) : null}
      </main>
      <Inspector
        task={detail}
        open={inspectorOpen}
        titleRef={titleRef}
        onTitleCommit={(title) => void onTitleCommit(title)}
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
        onDelete={() => void onDelete()}
      />
    </div>
  );
}
