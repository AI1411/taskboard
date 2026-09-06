export type Column = "todo" | "in-progress" | "in-review" | "done";

export type DisplayStatus = "idle" | "running" | "waiting" | "failed" | "completed";

export type RunStatus = "running" | "waiting" | "failed" | "completed";

export type LinkKind = "url" | "path";

export type ActorKind = "cli" | "desktop" | "web";

export type EntityType = "project" | "task" | "run" | "link";

export interface Project {
  id: string;
  slug: string;
  name: string;
  repoPath: string | null;
  archived: boolean;
  noteMarkdown: string;
  sortOrder: number;
  revision: number;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
}

export interface TaskSummary {
  id: string;
  displayId: string;
  projectId: string;
  title: string;
  column: Column;
  urgent: boolean;
  revision: number;
  displayStatus: DisplayStatus;
  runMessage: string | null;
}

export interface Link {
  id: string;
  taskId: string;
  kind: LinkKind;
  value: string;
  sortOrder: number;
}

export interface Run {
  id: string;
  displayId: string;
  taskId: string;
  agent: string;
  sessionId: string | null;
  status: RunStatus;
  message: string | null;
  waitingReason: string | null;
  summary: string | null;
  startedAt: string;
  endedAt: string | null;
  revision: number;
  createdAt: string;
  updatedAt: string;
}

export interface Activity {
  id: string;
  sequence: number;
  actorKind: ActorKind;
  actorLabel: string;
  operation: string;
  entityType: EntityType;
  entityId: string;
  previousRevision: number | null;
  beforeJson: unknown;
  afterJson: unknown;
  createdAt: string;
}

/** Flattened task show/mutation DTO — fields live on the task itself, not nested `.task`. */
export interface TaskDetail extends TaskSummary {
  noteMarkdown: string;
  links: Link[];
  runs: Run[];
  recentActivities: Activity[];
}

export interface Trash {
  projects: Project[];
  tasks: TaskSummary[];
}

export interface SyncDelta {
  sequence: number;
  projects: Project[];
  tasks: TaskDetail[];
  runs: Run[];
}

export type UndoResult = Record<string, unknown>;

export interface ProjectPatch {
  name?: string;
  repoPath?: string | null;
  slug?: string;
}

export interface TaskPatch {
  title?: string;
  noteMarkdown?: string;
  urgent?: boolean;
  column?: Column;
  beforeDisplayId?: string | null;
}

export type RunOp =
  | { op: "update"; message?: string }
  | { op: "wait"; reason: string }
  | { op: "fail"; summary: string }
  | { op: "finish"; summary: string };
