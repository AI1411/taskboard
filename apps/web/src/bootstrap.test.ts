import assert from "node:assert/strict";
import { describe, it } from "vitest";

import {
  fetchBootstrap,
  refetchInterval,
  transportFromBootstrap,
} from "./bootstrap";

describe("bootstrap", () => {
  it("constructs HttpTransport from bootstrap JSON", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(
        JSON.stringify({ sequence: 1, projects: [], tasks: [], runs: [] }),
        { headers: { "Content-Type": "application/json" } },
      );
    };
    const transport = transportFromBootstrap(
      { session: "deadbeef" },
      "http://127.0.0.1:9",
      fetchImpl,
    );
    await transport.sync(0);
    assert.equal(fetches[0].headers.get("X-Taskboard-Session"), "deadbeef");
    assert.equal(new URL(fetches[0].url).origin, "http://127.0.0.1:9");
    assert.equal(new URL(fetches[0].url).pathname, "/api/v1/sync");
  });

  it("fetches bootstrap with credentials include", async () => {
    const paths: string[] = [];
    const credentials: RequestCredentials[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      paths.push(typeof input === "string" ? input : String(input));
      credentials.push(init?.credentials ?? "same-origin");
      return new Response(JSON.stringify({ session: "tok", activitySequence: 4 }), {
        headers: { "Content-Type": "application/json" },
      });
    };
    const body = await fetchBootstrap(fetchImpl);
    assert.equal(body.session, "tok");
    assert.equal(body.activitySequence, 4);
    assert.equal(paths[0], "/api/v1/bootstrap");
    assert.equal(credentials[0], "include");
  });

  it("polls every 1000ms only while the document is visible", () => {
    assert.equal(refetchInterval("visible"), 1000);
    assert.equal(refetchInterval("hidden"), false);
  });
});
