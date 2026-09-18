import type {
  Check,
  Comment,
  Project,
  ProjectPatch,
  ReviewAction,
  Run,
  RunOp,
  SyncDelta,
  TaskDetail,
  TaskPatch,
  TaskSummary,
  InboxItem,
  Trash,
  UndoResult,
  BoardStatus,
  OccupancyGroup,
} from "@taskboard/types";

import type { Transport, UiState } from "./transport";

export type TransportDispatch = <T>(spec: {
  cmd: string;
  args?: Record<string, unknown>;
  http: {
    method: string;
    path: string;
    body?: unknown;
    revision?: number;
    unwrap?: "entity" | "entities" | "raw";
  };
}) => Promise<T>;

function enc(value: string): string {
  return encodeURIComponent(value);
}

export function createTransport(dispatch: TransportDispatch): Transport {
  return {
    projectAdd: (input) =>
      dispatch<Project>({
        cmd: "project_add",
        args: input,
        http: { method: "POST", path: "/api/v1/projects", body: input },
      }),

    projectList: (includeArchived) =>
      dispatch<Project[]>({
        cmd: "project_list",
        args: { includeArchived },
        http: {
          method: "GET",
          path: `/api/v1/projects?archived=${includeArchived}`,
          unwrap: "entities",
        },
      }),

    projectUpdate: (slug, patch: ProjectPatch, revision) => {
      const { slug: newSlug, ...rest } = patch;
      return dispatch<Project>({
        cmd: "project_update",
        args: {
          slug,
          ...rest,
          ...(newSlug !== undefined ? { newSlug } : {}),
          revision,
        },
        http: {
          method: "PATCH",
          path: `/api/v1/projects/${enc(slug)}`,
          body: patch,
          revision,
        },
      });
    },

    projectReorder: (slugs) =>
      dispatch<Project[]>({
        cmd: "project_reorder",
        args: { slugs },
        http: {
          method: "POST",
          path: "/api/v1/projects/reorder",
          body: { slugs },
          unwrap: "entities",
        },
      }),

    projectArchive: (slug, archived, revision) =>
      dispatch<Project>({
        cmd: "project_archive",
        args: { slug, archived, revision },
        http: {
          method: "PATCH",
          path: `/api/v1/projects/${enc(slug)}`,
          body: { archived },
          revision,
        },
      }),

    projectDelete: (slug, revision) =>
      dispatch<Project>({
        cmd: "project_delete",
        args: { slug, revision },
        http: {
          method: "DELETE",
          path: `/api/v1/projects/${enc(slug)}`,
          revision,
        },
      }),

    projectRestore: (slug) =>
      dispatch<Project>({
        cmd: "project_restore",
        args: { slug },
        http: { method: "POST", path: `/api/v1/projects/${enc(slug)}/restore` },
      }),

    projectNoteSet: (slug, markdown, revision) =>
      dispatch<Project>({
        cmd: "project_note_set",
        args: { slug, markdown, revision },
        http: {
          method: "PATCH",
          path: `/api/v1/projects/${enc(slug)}`,
          body: { noteMarkdown: markdown },
          revision,
        },
      }),

    taskCreate: (projectSlug, input) =>
      dispatch<TaskDetail>({
        cmd: "task_create",
        args: { projectSlug, ...input },
        http: {
          method: "POST",
          path: `/api/v1/projects/${enc(projectSlug)}/tasks`,
          body: input,
        },
      }),

    taskList: (projectSlug) =>
      dispatch<TaskSummary[]>({
        cmd: "task_list",
        args: { projectSlug },
        http: {
          method: "GET",
          path: `/api/v1/projects/${enc(projectSlug)}/tasks`,
          unwrap: "entities",
        },
      }),

    taskShow: (displayId) =>
      dispatch<TaskDetail>({
        cmd: "task_show",
        args: { displayId },
        http: { method: "GET", path: `/api/v1/tasks/${enc(displayId)}` },
      }),

    taskUpdate: (displayId, patch: TaskPatch, revision) =>
      dispatch<TaskDetail>({
        cmd: "task_update",
        args: { displayId, ...patch, revision },
        http: {
          method: "PATCH",
          path: `/api/v1/tasks/${enc(displayId)}`,
          body: patch,
          revision,
        },
      }),

    taskMove: (displayId, column, revision) =>
      dispatch<TaskDetail>({
        cmd: "task_move",
        args: { displayId, column, revision },
        http: {
          method: "PATCH",
          path: `/api/v1/tasks/${enc(displayId)}`,
          body: { column },
          revision,
        },
      }),

    taskReorder: (displayId, beforeDisplayId, revision) =>
      dispatch<TaskDetail>({
        cmd: "task_reorder",
        args: {
          displayId,
          beforeDisplayId: beforeDisplayId ?? null,
          revision,
        },
        http: {
          method: "PATCH",
          path: `/api/v1/tasks/${enc(displayId)}`,
          body: { beforeDisplayId: beforeDisplayId ?? null },
          revision,
        },
      }),

    taskUrgent: (displayId, urgent, revision) =>
      dispatch<TaskDetail>({
        cmd: "task_urgent",
        args: { displayId, urgent, revision },
        http: {
          method: "PATCH",
          path: `/api/v1/tasks/${enc(displayId)}`,
          body: { urgent },
          revision,
        },
      }),

    taskDelete: (displayId, revision) =>
      dispatch<TaskDetail>({
        cmd: "task_delete",
        args: { displayId, revision },
        http: {
          method: "DELETE",
          path: `/api/v1/tasks/${enc(displayId)}`,
          revision,
        },
      }),

    taskRestore: (displayId) =>
      dispatch<TaskDetail>({
        cmd: "task_restore",
        args: { displayId },
        http: { method: "POST", path: `/api/v1/tasks/${enc(displayId)}/restore` },
      }),

    taskNoteSet: (displayId, markdown, revision) =>
      dispatch<TaskDetail>({
        cmd: "task_note_set",
        args: { displayId, markdown, revision },
        http: {
          method: "PATCH",
          path: `/api/v1/tasks/${enc(displayId)}`,
          body: { noteMarkdown: markdown },
          revision,
        },
      }),

    linkAdd: (displayId, input) =>
      dispatch<TaskDetail>({
        cmd: "link_add",
        args: { displayId, ...input },
        http: {
          method: "POST",
          path: `/api/v1/tasks/${enc(displayId)}/links`,
          body: input,
        },
      }),

    linkRemove: (linkId, revision) =>
      dispatch<TaskDetail>({
        cmd: "link_remove",
        args: { linkId, revision },
        http: {
          method: "DELETE",
          path: `/api/v1/links/${enc(linkId)}`,
          revision,
        },
      }),

    commentAdd: (displayId, body, continueWaiting) =>
      dispatch<Comment>({
        cmd: "comment_add",
        args: { displayId, body, continueWaiting: continueWaiting ?? false },
        http: {
          method: "POST",
          path: `/api/v1/tasks/${enc(displayId)}/comments`,
          body: { body, continue: continueWaiting ?? false },
        },
      }),

    commentRemoveLatest: (displayId) =>
      dispatch<Comment>({
        cmd: "comment_remove_latest",
        args: { displayId },
        http: {
          method: "DELETE",
          path: `/api/v1/tasks/${enc(displayId)}/comments/latest`,
        },
      }),

    checkAdd: (displayId, text) =>
      dispatch<Check>({
        cmd: "check_add",
        args: { displayId, text },
        http: {
          method: "POST",
          path: `/api/v1/tasks/${enc(displayId)}/checks`,
          body: { text },
        },
      }),

    checkToggle: (displayId) =>
      dispatch<Check>({
        cmd: "check_toggle",
        args: { displayId },
        http: {
          method: "PATCH",
          path: `/api/v1/checks/${enc(displayId)}`,
          body: {},
        },
      }),

    checkRemove: (displayId) =>
      dispatch<Check>({
        cmd: "check_remove",
        args: { displayId },
        http: { method: "DELETE", path: `/api/v1/checks/${enc(displayId)}` },
      }),

    runStart: (displayId, input) =>
      dispatch<Run>({
        cmd: "run_start",
        args: { displayId, ...input },
        http: {
          method: "POST",
          path: `/api/v1/tasks/${enc(displayId)}/runs`,
          body: input,
        },
      }),

    runPatch: (runDisplayId, op: RunOp, revision) =>
      dispatch<Run>({
        cmd: "run_patch",
        args: { runDisplayId, ...op, revision },
        http: {
          method: "PATCH",
          path: `/api/v1/runs/${enc(runDisplayId)}`,
          body: op,
          revision,
        },
      }),

    review: (displayId, input: { action: ReviewAction; text: string }) =>
      dispatch<TaskDetail>({
        cmd: "review",
        args: { displayId, ...input },
        http: {
          method: "POST",
          path: `/api/v1/tasks/${enc(displayId)}/review`,
          body: input,
        },
      }),

    status: (project) => {
      const q = project ? `?project=${enc(project)}` : "";
      return dispatch<BoardStatus>({
        cmd: "status",
        args: { project },
        http: { method: "GET", path: `/api/v1/status${q}` },
      });
    },

    occupancy: (path) => {
      const q = path ? `?path=${enc(path)}` : "";
      return dispatch<OccupancyGroup[]>({
        cmd: "occupancy",
        args: { path },
        http: { method: "GET", path: `/api/v1/occupancy${q}`, unwrap: "entities" },
      });
    },

    taskSpawn: (displayId, titles) =>
      dispatch<TaskDetail[]>({
        cmd: "task_spawn",
        args: { displayId, titles },
        http: {
          method: "POST",
          path: `/api/v1/tasks/${enc(displayId)}/spawn`,
          body: { titles },
          unwrap: "entities",
        },
      }),

    inbox: (opts) => {
      const archived = opts?.includeArchived ?? false;
      const params = new URLSearchParams({ archived: String(archived) });
      if (opts?.project) params.set("project", opts.project);
      return dispatch<InboxItem[]>({
        cmd: "inbox",
        args: {
          project: opts?.project,
          includeArchived: opts?.includeArchived ?? false,
        },
        http: {
          method: "GET",
          path: `/api/v1/inbox?${params.toString()}`,
          unwrap: "entities",
        },
      });
    },

    trashList: () =>
      dispatch<Trash>({
        cmd: "trash_list",
        http: { method: "GET", path: "/api/v1/trash" },
      }),

    undo: () =>
      dispatch<UndoResult>({
        cmd: "undo",
        http: { method: "POST", path: "/api/v1/undo" },
      }),

    sync: (after) =>
      dispatch<SyncDelta>({
        cmd: "sync",
        args: { after },
        http: {
          method: "GET",
          path: `/api/v1/sync?after=${after}`,
          unwrap: "raw",
        },
      }),

    uiState: () =>
      dispatch<UiState>({
        cmd: "ui_state",
        http: { method: "GET", path: "/api/v1/ui-state" },
      }),

    uiStateSet: (lastProjectSlug) =>
      dispatch<UiState>({
        cmd: "ui_state_set",
        args: { lastProjectSlug },
        http: {
          method: "PATCH",
          path: "/api/v1/ui-state",
          body: { lastProjectSlug },
        },
      }),
  };
}
