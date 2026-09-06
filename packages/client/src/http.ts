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

type Envelope = {
  ok?: boolean;
  entity?: unknown;
  entities?: unknown;
  error?: { message?: string; code?: string };
};

export class HttpTransport implements Transport {
  constructor(
    private readonly baseUrl: string,
    private readonly session: string,
    private readonly fetchImpl: typeof fetch = fetch,
  ) {}

  projectAdd(input: { name: string; repoPath?: string; slug?: string }): Promise<Project> {
    return this.request("POST", "/api/v1/projects", { body: input });
  }

  projectList(includeArchived: boolean): Promise<Project[]> {
    return this.request("GET", `/api/v1/projects?archived=${includeArchived}`, {
      unwrap: "entities",
    });
  }

  projectUpdate(slug: string, patch: ProjectPatch, revision?: number): Promise<Project> {
    return this.request("PATCH", `/api/v1/projects/${enc(slug)}`, { body: patch, revision });
  }

  projectReorder(slugs: string[]): Promise<Project[]> {
    return this.request("POST", "/api/v1/projects/reorder", {
      body: { slugs },
      unwrap: "entities",
    });
  }

  projectArchive(slug: string, archived: boolean, revision?: number): Promise<Project> {
    return this.request("PATCH", `/api/v1/projects/${enc(slug)}`, {
      body: { archived },
      revision,
    });
  }

  projectDelete(slug: string, revision?: number): Promise<Project> {
    return this.request("DELETE", `/api/v1/projects/${enc(slug)}`, { revision });
  }

  projectRestore(slug: string): Promise<Project> {
    return this.request("POST", `/api/v1/projects/${enc(slug)}/restore`);
  }

  projectNoteSet(slug: string, markdown: string, revision?: number): Promise<Project> {
    return this.request("PATCH", `/api/v1/projects/${enc(slug)}`, {
      body: { noteMarkdown: markdown },
      revision,
    });
  }

  taskCreate(
    projectSlug: string,
    input: { title: string; column?: Column; urgent?: boolean },
  ): Promise<TaskDetail> {
    return this.request("POST", `/api/v1/projects/${enc(projectSlug)}/tasks`, { body: input });
  }

  taskList(projectSlug: string): Promise<TaskSummary[]> {
    return this.request("GET", `/api/v1/projects/${enc(projectSlug)}/tasks`, {
      unwrap: "entities",
    });
  }

  taskShow(displayId: string): Promise<TaskDetail> {
    return this.request("GET", `/api/v1/tasks/${enc(displayId)}`);
  }

  taskUpdate(displayId: string, patch: TaskPatch, revision?: number): Promise<TaskDetail> {
    return this.request("PATCH", `/api/v1/tasks/${enc(displayId)}`, { body: patch, revision });
  }

  taskMove(displayId: string, column: Column, revision?: number): Promise<TaskDetail> {
    return this.request("PATCH", `/api/v1/tasks/${enc(displayId)}`, {
      body: { column },
      revision,
    });
  }

  taskReorder(displayId: string, beforeDisplayId?: string, revision?: number): Promise<TaskDetail> {
    return this.request("PATCH", `/api/v1/tasks/${enc(displayId)}`, {
      body: { beforeDisplayId: beforeDisplayId ?? null },
      revision,
    });
  }

  taskUrgent(displayId: string, urgent: boolean, revision?: number): Promise<TaskDetail> {
    return this.request("PATCH", `/api/v1/tasks/${enc(displayId)}`, {
      body: { urgent },
      revision,
    });
  }

  taskDelete(displayId: string, revision?: number): Promise<TaskDetail> {
    return this.request("DELETE", `/api/v1/tasks/${enc(displayId)}`, { revision });
  }

  taskRestore(displayId: string): Promise<TaskDetail> {
    return this.request("POST", `/api/v1/tasks/${enc(displayId)}/restore`);
  }

  taskNoteSet(displayId: string, markdown: string, revision?: number): Promise<TaskDetail> {
    return this.request("PATCH", `/api/v1/tasks/${enc(displayId)}`, {
      body: { noteMarkdown: markdown },
      revision,
    });
  }

  linkAdd(displayId: string, input: { kind: "url" | "path"; value: string }): Promise<TaskDetail> {
    return this.request("POST", `/api/v1/tasks/${enc(displayId)}/links`, { body: input });
  }

  linkRemove(linkId: string, revision?: number): Promise<TaskDetail> {
    return this.request("DELETE", `/api/v1/links/${enc(linkId)}`, { revision });
  }

  runStart(displayId: string, input: { agent: string; sessionId?: string }): Promise<Run> {
    return this.request("POST", `/api/v1/tasks/${enc(displayId)}/runs`, { body: input });
  }

  runPatch(runDisplayId: string, op: RunOp, revision?: number): Promise<Run> {
    return this.request("PATCH", `/api/v1/runs/${enc(runDisplayId)}`, { body: op, revision });
  }

  trashList(): Promise<Trash> {
    return this.request("GET", "/api/v1/trash");
  }

  undo(): Promise<UndoResult> {
    return this.request("POST", "/api/v1/undo");
  }

  sync(after: number): Promise<SyncDelta> {
    return this.request("GET", `/api/v1/sync?after=${after}`, { unwrap: "raw" });
  }

  private url(path: string): string {
    return `${this.baseUrl.replace(/\/$/, "")}${path}`;
  }

  private async request<T>(
    method: string,
    path: string,
    opts: {
      body?: unknown;
      revision?: number;
      unwrap?: "entity" | "entities" | "raw";
    } = {},
  ): Promise<T> {
    const headers = new Headers();
    headers.set("X-Taskboard-Session", this.session);
    if (opts.body !== undefined) {
      headers.set("Content-Type", "application/json");
    }
    if (opts.revision !== undefined) {
      headers.set("If-Match", String(opts.revision));
    }

    const res = await this.fetchImpl(this.url(path), {
      method,
      headers,
      body: opts.body === undefined ? undefined : JSON.stringify(opts.body),
    });

    const json = (await res.json()) as Envelope;
    if (!res.ok || json.error) {
      throw new Error(json.error?.message ?? `HTTP ${res.status}`);
    }

    const unwrap = opts.unwrap ?? "entity";
    if (unwrap === "raw") {
      return json as T;
    }
    if (unwrap === "entities") {
      return json.entities as T;
    }
    return json.entity as T;
  }
}

function enc(value: string): string {
  return encodeURIComponent(value);
}
