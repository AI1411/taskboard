import assert from "node:assert/strict";
import { describe, it } from "vitest";

import { HttpTransport } from "./http";

describe("HttpTransport", () => {
  it("sends session and camelCase body", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(JSON.stringify({ ok: true, entity: { slug: "renai-sim", revision: 1 }, revision: 1 }), {
        headers: { "Content-Type": "application/json" },
      });
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    const p = await t.projectAdd({ name: "Renai Sim" });
    assert.equal(p.slug, "renai-sim");
    assert.equal(fetches[0].headers.get("X-Taskboard-Session"), "deadbeef");
  });

  it("sends camelCase projectAdd body and unwraps entity", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(JSON.stringify({ ok: true, entity: { slug: "renai-sim", revision: 1 }, revision: 1 }), {
        headers: { "Content-Type": "application/json" },
      });
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    await t.projectAdd({ name: "Renai Sim", repoPath: "/tmp/renai" });
    const req = fetches[0];
    assert.equal(req.method, "POST");
    assert.equal(new URL(req.url).pathname, "/api/v1/projects");
    assert.equal(req.headers.get("Content-Type"), "application/json");
    const body = (await req.json()) as Record<string, unknown>;
    assert.equal(body.name, "Renai Sim");
    assert.equal(body.repoPath, "/tmp/renai");
    assert.equal("repo_path" in body, false);
  });

  it("sends If-Match when revision is provided", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(JSON.stringify({ ok: true, entity: { slug: "renai-sim", name: "Renai" }, revision: 4 }), {
        headers: { "Content-Type": "application/json" },
      });
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    await t.projectUpdate("renai-sim", { name: "Renai" }, 3);
    const req = fetches[0];
    assert.equal(req.method, "PATCH");
    assert.equal(new URL(req.url).pathname, "/api/v1/projects/renai-sim");
    assert.equal(req.headers.get("If-Match"), "3");
    assert.equal(req.headers.get("X-Taskboard-Session"), "deadbeef");
  });

  it("sends beforeDisplayId null to move to end", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(JSON.stringify({ ok: true, entity: { displayId: "TASK-1" }, revision: 2 }), {
        headers: { "Content-Type": "application/json" },
      });
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    await t.taskReorder("TASK-1");
    const req = fetches[0];
    assert.equal(req.method, "PATCH");
    assert.equal(new URL(req.url).pathname, "/api/v1/tasks/TASK-1");
    const body = (await req.json()) as Record<string, unknown>;
    assert.equal(body.beforeDisplayId, null);
    assert.equal("beforeDisplayId" in body, true);
  });

  it("sends worktreePath and branch on taskUpdate", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(
        JSON.stringify({
          ok: true,
          entity: { displayId: "TASK-1", worktreePath: "/tmp/wt", branch: "cursor/foo-88ba" },
          revision: 2,
        }),
        { headers: { "Content-Type": "application/json" } },
      );
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    await t.taskUpdate("TASK-1", { worktreePath: "/tmp/wt", branch: "cursor/foo-88ba" }, 1);
    const body = (await fetches[0].json()) as Record<string, unknown>;
    assert.equal(body.worktreePath, "/tmp/wt");
    assert.equal(body.branch, "cursor/foo-88ba");
    assert.equal("worktree_path" in body, false);
  });

  it("unwraps list entities and archived query", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(JSON.stringify({ ok: true, entities: [{ slug: "a" }, { slug: "b" }] }), {
        headers: { "Content-Type": "application/json" },
      });
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    const list = await t.projectList(false);
    assert.equal(list.length, 2);
    assert.equal(list[0].slug, "a");
    const url = new URL(fetches[0].url);
    assert.equal(url.pathname, "/api/v1/projects");
    assert.equal(url.searchParams.get("archived"), "false");
  });

  it("inbox sends project and archived query", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(JSON.stringify({ ok: true, entities: [] }), {
        headers: { "Content-Type": "application/json" },
      });
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    await t.inbox({ project: "renai-sim", includeArchived: true });
    const url = new URL(fetches[0].url);
    assert.equal(url.pathname, "/api/v1/inbox");
    assert.equal(url.searchParams.get("project"), "renai-sim");
    assert.equal(url.searchParams.get("archived"), "true");
  });

  it("uiState get and set lastProjectSlug", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(JSON.stringify({ ok: true, entity: { lastProjectSlug: "alpha" } }), {
        headers: { "Content-Type": "application/json" },
      });
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    await t.uiState();
    assert.equal(new URL(fetches[0].url).pathname, "/api/v1/ui-state");
    assert.equal(fetches[0].method, "GET");
    await t.uiStateSet("alpha");
    assert.equal(fetches[1].method, "PATCH");
    const body = (await fetches[1].json()) as Record<string, unknown>;
    assert.equal(body.lastProjectSlug, "alpha");
  });

  it("posts comments and checks and patches toggle", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(
        JSON.stringify({
          ok: true,
          entity: { displayId: "CHECK-1", body: "hi", done: true },
          revision: 0,
        }),
        { headers: { "Content-Type": "application/json" } },
      );
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    await t.commentAdd("TASK-1", "hi");
    await t.checkAdd("TASK-1", "Write tests");
    await t.checkToggle("CHECK-1");
    await t.linkAdd("TASK-1", { kind: "blocked_by", value: "TASK-2" });
    await t.review("TASK-1", { action: "approve", text: "lgtm" });
    assert.equal(fetches[0].method, "POST");
    assert.equal(new URL(fetches[0].url).pathname, "/api/v1/tasks/TASK-1/comments");
    assert.deepEqual(await fetches[0].json(), { body: "hi", continue: false });
    assert.equal(new URL(fetches[1].url).pathname, "/api/v1/tasks/TASK-1/checks");
    assert.deepEqual(await fetches[1].json(), { text: "Write tests" });
    assert.equal(fetches[2].method, "PATCH");
    assert.equal(new URL(fetches[2].url).pathname, "/api/v1/checks/CHECK-1");
    assert.equal(new URL(fetches[3].url).pathname, "/api/v1/tasks/TASK-1/links");
    assert.deepEqual(await fetches[3].json(), { kind: "blocked_by", value: "TASK-2" });
    assert.equal(new URL(fetches[4].url).pathname, "/api/v1/tasks/TASK-1/review");
    assert.deepEqual(await fetches[4].json(), { action: "approve", text: "lgtm" });
  });

  it("loads status occupancy and spawn as human routes", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(
        JSON.stringify({
          ok: true,
          entity: { ready: 1, inbox: { total: 0 } },
          entities: [{ worktreePath: "/tmp/shared" }, { displayId: "TASK-2" }],
          revision: 0,
        }),
        { headers: { "Content-Type": "application/json" } },
      );
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    await t.status("renai-sim");
    assert.equal(new URL(fetches[0].url).pathname, "/api/v1/status");
    assert.equal(new URL(fetches[0].url).searchParams.get("project"), "renai-sim");
    await t.occupancy();
    assert.equal(new URL(fetches[1].url).pathname, "/api/v1/occupancy");
    await t.taskSpawn("TASK-1", ["Child"]);
    assert.equal(fetches[2].method, "POST");
    assert.equal(new URL(fetches[2].url).pathname, "/api/v1/tasks/TASK-1/spawn");
    assert.deepEqual(await fetches[2].json(), { titles: ["Child"] });
  });
});
