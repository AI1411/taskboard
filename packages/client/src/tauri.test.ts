import assert from "node:assert/strict";
import { describe, it } from "vitest";

import { TauriTransport, type InvokeFn } from "./tauri";

type Call = { cmd: string; args?: Record<string, unknown> };

function recordingInvoke(result: unknown = {}): { calls: Call[]; invoke: InvokeFn } {
  const calls: Call[] = [];
  const invoke: InvokeFn = async (cmd, args) => {
    calls.push({ cmd, args });
    return result as never;
  };
  return { calls, invoke };
}

describe("TauriTransport", () => {
  it("invokes project_add with camelCase payload mapped to rust args", async () => {
    const calls: { cmd: string; args?: Record<string, unknown> }[] = [];
    const invoke: InvokeFn = async (cmd, args) => {
      calls.push({ cmd, args });
      return {
        id: "00000000-0000-7000-0000-000000000001",
        slug: "renai-sim",
        name: "Renai Sim",
        revision: 1,
      } as never;
    };
    const t = new TauriTransport(invoke);
    const p = await t.projectAdd({ name: "Renai Sim" });
    assert.equal(calls[0].cmd, "project_add");
    assert.equal(p.slug, "renai-sim");
  });

  it("maps camelCase keys to snake_case invoke args and omits undefined", async () => {
    const { calls, invoke } = recordingInvoke({ slug: "renai-sim" });
    const t = new TauriTransport(invoke);
    await t.projectAdd({ name: "Renai Sim", repoPath: "/tmp/renai" });
    assert.deepEqual(calls[0].args, { name: "Renai Sim", repo_path: "/tmp/renai" });
    assert.equal("repoPath" in (calls[0].args ?? {}), false);
  });

  it("includes revision when provided and maps display_id", async () => {
    const { calls, invoke } = recordingInvoke({ displayId: "TASK-1" });
    const t = new TauriTransport(invoke);
    await t.taskMove("TASK-1", "done", 4);
    assert.equal(calls[0].cmd, "task_move");
    assert.deepEqual(calls[0].args, { display_id: "TASK-1", column: "done", revision: 4 });
  });

  it("omits revision when not provided", async () => {
    const { calls, invoke } = recordingInvoke({ displayId: "TASK-1" });
    const t = new TauriTransport(invoke);
    await t.taskDelete("TASK-1");
    assert.deepEqual(calls[0].args, { display_id: "TASK-1" });
  });

  it("passes before_display_id null to move to end", async () => {
    const { calls, invoke } = recordingInvoke({ displayId: "TASK-1" });
    const t = new TauriTransport(invoke);
    await t.taskReorder("TASK-1");
    assert.equal(calls[0].cmd, "task_reorder");
    assert.equal(calls[0].args?.before_display_id, null);
    assert.equal("before_display_id" in (calls[0].args ?? {}), true);
  });

  it("passes before_display_id when placing before another card", async () => {
    const { calls, invoke } = recordingInvoke({ displayId: "TASK-1" });
    const t = new TauriTransport(invoke);
    await t.taskReorder("TASK-1", "TASK-2", 3);
    assert.deepEqual(calls[0].args, {
      display_id: "TASK-1",
      before_display_id: "TASK-2",
      revision: 3,
    });
  });

  it("rethrows AppErrorDto with .code on the Error", async () => {
    const invoke: InvokeFn = async () => {
      throw { code: "validation_error", message: "title is required", field: "title", current: null };
    };
    const t = new TauriTransport(invoke);
    await assert.rejects(
      () => t.projectAdd({ name: "x" }),
      (err: unknown) => {
        assert.ok(err instanceof Error);
        assert.equal(err.message, "title is required");
        assert.equal((err as Error & { code: string }).code, "validation_error");
        return true;
      },
    );
  });

  it("does not swallow non-dto invoke rejections", async () => {
    const invoke: InvokeFn = async () => {
      throw new Error("invoke exploded");
    };
    const t = new TauriTransport(invoke);
    await assert.rejects(() => t.undo(), { message: "invoke exploded" });
  });

  it("returns invoke results as-is without HTTP envelopes", async () => {
    const entity = { sequence: 2, projects: [{ slug: "a" }], tasks: [], runs: [] };
    const { invoke } = recordingInvoke(entity);
    const t = new TauriTransport(invoke);
    const delta = await t.sync(1);
    assert.equal(delta, entity);
  });

  it("does not call fetch", async () => {
    const fetches: string[] = [];
    const original = globalThis.fetch;
    globalThis.fetch = (async (input) => {
      fetches.push(String(input));
      return new Response("{}");
    }) as typeof fetch;
    try {
      const { invoke } = recordingInvoke({ slug: "a" });
      const t = new TauriTransport(invoke);
      await t.projectList(false);
      assert.deepEqual(fetches, []);
    } finally {
      globalThis.fetch = original;
    }
  });

  it("maps every Transport method to the snake_case command", async () => {
    const { calls, invoke } = recordingInvoke({});
    const t = new TauriTransport(invoke);

    await t.projectAdd({ name: "A" });
    await t.projectList(true);
    await t.projectUpdate("renai-sim", { name: "Renai", repoPath: "/tmp", slug: "renai" }, 2);
    await t.projectReorder(["a", "b"]);
    await t.projectArchive("a", true, 1);
    await t.projectDelete("a", 1);
    await t.projectRestore("a");
    await t.projectNoteSet("a", "# n", 1);
    await t.taskCreate("a", { title: "T", column: "todo", urgent: true });
    await t.taskList("a");
    await t.taskShow("TASK-1");
    await t.taskUpdate("TASK-1", { title: "U", noteMarkdown: "md" }, 1);
    await t.taskMove("TASK-1", "done");
    await t.taskReorder("TASK-1", undefined, 1);
    await t.taskUrgent("TASK-1", false, 1);
    await t.taskDelete("TASK-1");
    await t.taskRestore("TASK-1");
    await t.taskNoteSet("TASK-1", "note", 1);
    await t.linkAdd("TASK-1", { kind: "url", value: "https://example.com" });
    await t.linkRemove("link-id", 1);
    await t.runStart("TASK-1", { agent: "codex", sessionId: "sess" });
    await t.runPatch("RUN-1", { op: "fail", summary: "boom" }, 1);
    await t.trashList();
    await t.undo();
    await t.sync(0);

    const cmds = calls.map((c) => c.cmd);
    assert.deepEqual(cmds, [
      "project_add",
      "project_list",
      "project_update",
      "project_reorder",
      "project_archive",
      "project_delete",
      "project_restore",
      "project_note_set",
      "task_create",
      "task_list",
      "task_show",
      "task_update",
      "task_move",
      "task_reorder",
      "task_urgent",
      "task_delete",
      "task_restore",
      "task_note_set",
      "link_add",
      "link_remove",
      "run_start",
      "run_patch",
      "trash_list",
      "undo",
      "sync",
    ]);

    assert.deepEqual(calls[1].args, { include_archived: true });
    assert.deepEqual(calls[2].args, {
      slug: "renai-sim",
      name: "Renai",
      repo_path: "/tmp",
      new_slug: "renai",
      revision: 2,
    });
    assert.deepEqual(calls[7].args, { slug: "a", markdown: "# n", revision: 1 });
    assert.deepEqual(calls[8].args, {
      project_slug: "a",
      title: "T",
      column: "todo",
      urgent: true,
    });
    assert.deepEqual(calls[11].args, {
      display_id: "TASK-1",
      title: "U",
      note_markdown: "md",
      revision: 1,
    });
    assert.deepEqual(calls[20].args, {
      display_id: "TASK-1",
      agent: "codex",
      session_id: "sess",
    });
    assert.deepEqual(calls[21].args, {
      run_display_id: "RUN-1",
      op: "fail",
      summary: "boom",
      revision: 1,
    });
    assert.equal(calls[22].args, undefined);
    assert.equal(calls[23].args, undefined);
    assert.deepEqual(calls[24].args, { after: 0 });
  });
});
