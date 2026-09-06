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
});
