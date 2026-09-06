import { vi } from "vitest";
import type { Transport } from "@taskboard/client";
import type {
  Column,
  Project,
  TaskDetail,
  TaskSummary,
} from "@taskboard/types";

function now(): string {
  return "2026-09-05T00:00:00Z";
}

function slugify(name: string, existing: Project[]): string {
  const base = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "") || "untitled";
  let slug = base;
  let n = 2;
  const used = new Set(existing.map((p) => p.slug));
  while (used.has(slug)) {
    slug = `${base}-${n}`;
    n += 1;
  }
  return slug;
}

function asDetail(task: TaskSummary, extras?: Partial<TaskDetail>): TaskDetail {
  return {
    ...task,
    noteMarkdown: "",
    links: [],
    runs: [],
    recentActivities: [],
    ...extras,
  };
}

export function fakeTransport(): Transport {
  const projects: Project[] = [];
  const tasks: TaskSummary[] = [];
  const details = new Map<string, TaskDetail>();
  let projectSeq = 0;
  let taskSeq = 0;

  const impl: Transport = {
    async projectAdd(input) {
      projectSeq += 1;
      const project: Project = {
        id: `p${projectSeq}`,
        slug: slugify(input.name, projects),
        name: input.name,
        repoPath: input.repoPath ?? null,
        archived: false,
        noteMarkdown: "",
        sortOrder: projects.length,
        revision: 1,
        createdAt: now(),
        updatedAt: now(),
        deletedAt: null,
      };
      projects.push(project);
      return project;
    },
    async projectList(includeArchived) {
      return projects.filter((p) => includeArchived || !p.archived);
    },
    async projectUpdate(slug, patch, revision) {
      const p = projects.find((x) => x.slug === slug);
      if (!p) throw new Error("not found");
      if (patch.name !== undefined) p.name = patch.name;
      if (patch.repoPath !== undefined) p.repoPath = patch.repoPath;
      if (patch.slug !== undefined) p.slug = patch.slug;
      p.revision = (revision ?? p.revision) + 1;
      p.updatedAt = now();
      return { ...p };
    },
    async projectReorder(slugs) {
      slugs.forEach((slug, i) => {
        const p = projects.find((x) => x.slug === slug);
        if (p) p.sortOrder = i;
      });
      projects.sort((a, b) => a.sortOrder - b.sortOrder);
      return [...projects];
    },
    async projectArchive(slug, archived, revision) {
      const p = projects.find((x) => x.slug === slug);
      if (!p) throw new Error("not found");
      p.archived = archived;
      p.revision = (revision ?? p.revision) + 1;
      return { ...p };
    },
    async projectDelete(slug, revision) {
      const p = projects.find((x) => x.slug === slug);
      if (!p) throw new Error("not found");
      p.deletedAt = now();
      p.revision = (revision ?? p.revision) + 1;
      return { ...p };
    },
    async projectRestore(slug) {
      const p = projects.find((x) => x.slug === slug);
      if (!p) throw new Error("not found");
      p.deletedAt = null;
      p.revision += 1;
      return { ...p };
    },
    async projectNoteSet(slug, markdown, revision) {
      const p = projects.find((x) => x.slug === slug);
      if (!p) throw new Error("not found");
      p.noteMarkdown = markdown;
      p.revision = (revision ?? p.revision) + 1;
      return { ...p };
    },
    async taskCreate(projectSlug, input) {
      const project = projects.find((x) => x.slug === projectSlug);
      if (!project) throw new Error("not found");
      taskSeq += 1;
      const task: TaskSummary = {
        id: `t${taskSeq}`,
        displayId: `TASK-${taskSeq}`,
        projectId: project.id,
        title: input.title,
        column: input.column ?? "todo",
        urgent: input.urgent ?? false,
        revision: 1,
        displayStatus: "idle",
        runMessage: null,
      };
      tasks.push(task);
      const detail = asDetail(task);
      details.set(task.displayId, detail);
      return detail;
    },
    async taskList(projectSlug) {
      const project = projects.find((x) => x.slug === projectSlug);
      if (!project) return [];
      return tasks.filter((t) => t.projectId === project.id);
    },
    async taskShow(displayId) {
      const detail = details.get(displayId);
      if (!detail) throw new Error("not found");
      return { ...detail, links: [...detail.links], runs: [...detail.runs], recentActivities: [...detail.recentActivities] };
    },
    async taskUpdate(displayId, patch, revision) {
      const task = tasks.find((t) => t.displayId === displayId);
      const detail = details.get(displayId);
      if (!task || !detail) throw new Error("not found");
      if (patch.title !== undefined) task.title = patch.title;
      if (patch.urgent !== undefined) task.urgent = patch.urgent;
      if (patch.column !== undefined) task.column = patch.column;
      if (patch.noteMarkdown !== undefined) detail.noteMarkdown = patch.noteMarkdown;
      task.revision = (revision ?? task.revision) + 1;
      Object.assign(detail, task);
      details.set(displayId, { ...detail });
      return details.get(displayId)!;
    },
    async taskMove(displayId, column: Column, revision) {
      return impl.taskUpdate(displayId, { column }, revision);
    },
    async taskReorder(displayId, beforeDisplayId, revision) {
      const from = tasks.findIndex((t) => t.displayId === displayId);
      if (from < 0) throw new Error("not found");
      const [item] = tasks.splice(from, 1);
      if (beforeDisplayId) {
        const to = tasks.findIndex((t) => t.displayId === beforeDisplayId);
        tasks.splice(to < 0 ? tasks.length : to, 0, item);
      } else {
        tasks.push(item);
      }
      item.revision = (revision ?? item.revision) + 1;
      const detail = details.get(displayId);
      if (detail) {
        Object.assign(detail, item);
        details.set(displayId, { ...detail });
      }
      return details.get(displayId)!;
    },
    async taskUrgent(displayId, urgent, revision) {
      return impl.taskUpdate(displayId, { urgent }, revision);
    },
    async taskDelete(displayId, revision) {
      const task = tasks.find((t) => t.displayId === displayId);
      if (!task) throw new Error("not found");
      task.revision = (revision ?? task.revision) + 1;
      const idx = tasks.findIndex((t) => t.displayId === displayId);
      tasks.splice(idx, 1);
      return details.get(displayId)!;
    },
    async taskRestore(displayId) {
      const detail = details.get(displayId);
      if (!detail) throw new Error("not found");
      if (!tasks.some((t) => t.displayId === displayId)) {
        tasks.push(detail);
      }
      return { ...detail };
    },
    async taskNoteSet(displayId, markdown, revision) {
      return impl.taskUpdate(displayId, { noteMarkdown: markdown }, revision);
    },
    async linkAdd(displayId, input) {
      const detail = details.get(displayId);
      if (!detail) throw new Error("not found");
      detail.links.push({
        id: `l${detail.links.length + 1}`,
        taskId: detail.id,
        kind: input.kind,
        value: input.value,
        sortOrder: detail.links.length,
      });
      return { ...detail, links: [...detail.links] };
    },
    async linkRemove(linkId) {
      for (const detail of details.values()) {
        const i = detail.links.findIndex((l) => l.id === linkId);
        if (i >= 0) {
          detail.links.splice(i, 1);
          return { ...detail, links: [...detail.links] };
        }
      }
      throw new Error("not found");
    },
    async runStart() {
      throw new Error("not implemented");
    },
    async runPatch() {
      throw new Error("not implemented");
    },
    async trashList() {
      return { projects: [], tasks: [] };
    },
    async undo() {
      return {};
    },
    async sync() {
      return { sequence: 0, projects: [], tasks: [], runs: [] };
    },
  };

  return Object.fromEntries(
    (Object.keys(impl) as (keyof Transport)[]).map((key) => [key, vi.fn(impl[key])]),
  ) as unknown as Transport;
}
