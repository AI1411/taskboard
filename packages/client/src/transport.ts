import type {
  Check,
  Column,
  Comment,
  LinkKind,
  Project,
  ProjectPatch,
  Run,
  RunOp,
  SyncDelta,
  TaskDetail,
  TaskPatch,
  TaskSummary,
  InboxItem,
  Trash,
  UndoResult,
} from "@taskboard/types";

export type UiState = { lastProjectSlug: string | null };

export class TransportError extends Error {
  readonly code: string;
  readonly field?: string;
  readonly current?: unknown;

  constructor(dto: { code: string; message: string; field?: string; current?: unknown }) {
    super(dto.message);
    this.name = "TransportError";
    this.code = dto.code;
    this.field = dto.field;
    this.current = dto.current;
  }
}

export interface Transport {
  projectAdd(input: { name: string; repoPath?: string; slug?: string }): Promise<Project>;
  projectList(includeArchived: boolean): Promise<Project[]>;
  projectUpdate(slug: string, patch: ProjectPatch, revision?: number): Promise<Project>;
  projectReorder(slugs: string[]): Promise<Project[]>;
  projectArchive(slug: string, archived: boolean, revision?: number): Promise<Project>;
  projectDelete(slug: string, revision?: number): Promise<Project>;
  projectRestore(slug: string): Promise<Project>;
  projectNoteSet(slug: string, markdown: string, revision?: number): Promise<Project>;
  taskCreate(
    projectSlug: string,
    input: { title: string; column?: Column; urgent?: boolean },
  ): Promise<TaskDetail>;
  taskList(projectSlug: string): Promise<TaskSummary[]>;
  taskShow(displayId: string): Promise<TaskDetail>;
  taskUpdate(displayId: string, patch: TaskPatch, revision?: number): Promise<TaskDetail>;
  taskMove(displayId: string, column: Column, revision?: number): Promise<TaskDetail>;
  taskReorder(displayId: string, beforeDisplayId?: string, revision?: number): Promise<TaskDetail>;
  taskUrgent(displayId: string, urgent: boolean, revision?: number): Promise<TaskDetail>;
  taskDelete(displayId: string, revision?: number): Promise<TaskDetail>;
  taskRestore(displayId: string): Promise<TaskDetail>;
  taskNoteSet(displayId: string, markdown: string, revision?: number): Promise<TaskDetail>;
  linkAdd(displayId: string, input: { kind: LinkKind; value: string }): Promise<TaskDetail>;
  linkRemove(linkId: string, revision?: number): Promise<TaskDetail>;
  commentAdd(displayId: string, body: string, continueWaiting?: boolean): Promise<Comment>;
  checkAdd(displayId: string, text: string): Promise<Check>;
  checkToggle(displayId: string): Promise<Check>;
  runStart(displayId: string, input: { agent: string; sessionId?: string }): Promise<Run>;
  runPatch(runDisplayId: string, op: RunOp, revision?: number): Promise<Run>;
  inbox(opts?: { project?: string; includeArchived?: boolean }): Promise<InboxItem[]>;
  trashList(): Promise<Trash>;
  undo(): Promise<UndoResult>;
  sync(after: number): Promise<SyncDelta>;
  uiState(): Promise<UiState>;
  uiStateSet(lastProjectSlug: string | null): Promise<UiState>;
}
