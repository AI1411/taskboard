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
  linkAdd(displayId: string, input: { kind: "url" | "path"; value: string }): Promise<TaskDetail>;
  linkRemove(linkId: string, revision?: number): Promise<TaskDetail>;
  runStart(displayId: string, input: { agent: string; sessionId?: string }): Promise<Run>;
  runPatch(runDisplayId: string, op: RunOp, revision?: number): Promise<Run>;
  trashList(): Promise<Trash>;
  undo(): Promise<UndoResult>;
  sync(after: number): Promise<SyncDelta>;
}
