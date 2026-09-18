import { useEffect, type MutableRefObject } from "react";
import type { Transport } from "@taskboard/client";
import type { Project, TaskSummary } from "@taskboard/types";

export type UseRouteSyncArgs = {
  transport: Transport;
  route?: { project?: string | null; task?: string | null };
  onRouteChange?: (route: { project: string | null; task: string | null }) => void;
  applyProject: (project: Project) => Promise<void>;
  selectCard: (displayId: string, fromTrash?: boolean) => Promise<void>;
  selectedProject: Project | null;
  selectedId: string | null;
  setProjects: (projects: Project[]) => void;
  tasksRef: MutableRefObject<TaskSummary[]>;
};

export function useRouteSync({
  transport,
  route,
  onRouteChange,
  applyProject,
  selectCard,
  selectedProject,
  selectedId,
  setProjects,
  tasksRef,
}: UseRouteSyncArgs) {
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
      const routedSlug = route?.project ?? null;
      const routed = routedSlug ? list.find((project) => project.slug === routedSlug) : undefined;
      const remembered = last ? list.find((project) => project.slug === last) : undefined;
      const next = routed ?? remembered ?? list[0];
      if (next) await applyProject(next);
      if (cancelled) return;
      const taskId = routed ? (route?.task ?? null) : null;
      if (taskId && tasksRef.current.some((task) => task.displayId === taskId)) {
        await selectCard(taskId);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [transport, applyProject, selectCard, route?.project, route?.task, setProjects, tasksRef]);

  useEffect(() => {
    if (!selectedProject) return;
    onRouteChange?.({
      project: selectedProject.slug,
      task: selectedId,
    });
  }, [selectedProject, selectedId, onRouteChange]);
}
