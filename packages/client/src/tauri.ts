import type {
  Column,
  Project,
  ProjectPatch,
  Run,
  RunOp,
  SyncDelta,
  TaskDetail,
  TaskPatch,
  TaskSummary,
  Trash,
  UndoResult,
} from "@taskboard/types";

import type { Transport } from "./transport";

export type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

type AppErrorDto = {
  code: string;
  message: string;
  field?: string;
  current?: unknown;
};

export class TransportError extends Error {
  readonly code: string;
  readonly field?: string;
  readonly current?: unknown;

  constructor(dto: AppErrorDto) {
    super(dto.message);
    this.name = "TransportError";
    this.code = dto.code;
    this.field = dto.field;
    this.current = dto.current;
  }
}

function camelToSnake(key: string): string {
  return key.replace(/[A-Z]/g, (ch) => `_${ch.toLowerCase()}`);
}

function snakeArgs(obj: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(obj)) {
    if (value === undefined) continue;
    out[camelToSnake(key)] = value;
  }
  return out;
}

function isAppErrorDto(value: unknown): value is AppErrorDto {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as { code?: unknown }).code === "string"
  );
}

export class TauriTransport implements Transport {
  constructor(private readonly invokeFn: InvokeFn) {}

  projectAdd(input: { name: string; repoPath?: string; slug?: string }): Promise<Project> {
    return this.call("project_add", input);
  }

  projectList(includeArchived: boolean): Promise<Project[]> {
    return this.call("project_list", { includeArchived });
  }

  projectUpdate(slug: string, patch: ProjectPatch, revision?: number): Promise<Project> {
    const { slug: newSlug, ...rest } = patch;
    return this.call("project_update", {
      slug,
      ...rest,
      ...(newSlug !== undefined ? { newSlug } : {}),
      revision,
    });
  }

  projectReorder(slugs: string[]): Promise<Project[]> {
    return this.call("project_reorder", { slugs });
  }

  projectArchive(slug: string, archived: boolean, revision?: number): Promise<Project> {
    return this.call("project_archive", { slug, archived, revision });
  }

  projectDelete(slug: string, revision?: number): Promise<Project> {
    return this.call("project_delete", { slug, revision });
  }

  projectRestore(slug: string): Promise<Project> {
    return this.call("project_restore", { slug });
  }

  projectNoteSet(slug: string, markdown: string, revision?: number): Promise<Project> {
    return this.call("project_note_set", { slug, markdown, revision });
  }

  taskCreate(
    projectSlug: string,
    input: { title: string; column?: Column; urgent?: boolean },
  ): Promise<TaskDetail> {
    return this.call("task_create", { projectSlug, ...input });
  }

  taskList(projectSlug: string): Promise<TaskSummary[]> {
    return this.call("task_list", { projectSlug });
  }

  taskShow(displayId: string): Promise<TaskDetail> {
    return this.call("task_show", { displayId });
  }

  taskUpdate(displayId: string, patch: TaskPatch, revision?: number): Promise<TaskDetail> {
    return this.call("task_update", { displayId, ...patch, revision });
  }

  taskMove(displayId: string, column: Column, revision?: number): Promise<TaskDetail> {
    return this.call("task_move", { displayId, column, revision });
  }

  taskReorder(displayId: string, beforeDisplayId?: string, revision?: number): Promise<TaskDetail> {
    return this.call("task_reorder", {
      displayId,
      beforeDisplayId: beforeDisplayId ?? null,
      revision,
    });
  }

  taskUrgent(displayId: string, urgent: boolean, revision?: number): Promise<TaskDetail> {
    return this.call("task_urgent", { displayId, urgent, revision });
  }

  taskDelete(displayId: string, revision?: number): Promise<TaskDetail> {
    return this.call("task_delete", { displayId, revision });
  }

  taskRestore(displayId: string): Promise<TaskDetail> {
    return this.call("task_restore", { displayId });
  }

  taskNoteSet(displayId: string, markdown: string, revision?: number): Promise<TaskDetail> {
    return this.call("task_note_set", { displayId, markdown, revision });
  }

  linkAdd(displayId: string, input: { kind: "url" | "path"; value: string }): Promise<TaskDetail> {
    return this.call("link_add", { displayId, ...input });
  }

  linkRemove(linkId: string, revision?: number): Promise<TaskDetail> {
    return this.call("link_remove", { linkId, revision });
  }

  runStart(displayId: string, input: { agent: string; sessionId?: string }): Promise<Run> {
    return this.call("run_start", { displayId, ...input });
  }

  runPatch(runDisplayId: string, op: RunOp, revision?: number): Promise<Run> {
    return this.call("run_patch", { runDisplayId, ...op, revision });
  }

  trashList(): Promise<Trash> {
    return this.call("trash_list");
  }

  undo(): Promise<UndoResult> {
    return this.call("undo");
  }

  sync(after: number): Promise<SyncDelta> {
    return this.call("sync", { after });
  }

  private async call<T>(cmd: string, raw?: Record<string, unknown>): Promise<T> {
    const args = raw === undefined ? undefined : snakeArgs(raw);
    const invokeArgs = args && Object.keys(args).length > 0 ? args : undefined;
    try {
      return await this.invokeFn<T>(cmd, invokeArgs);
    } catch (err) {
      if (isAppErrorDto(err)) {
        throw new TransportError(err);
      }
      throw err;
    }
  }
}
