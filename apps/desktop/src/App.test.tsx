import { render } from "@testing-library/react";
import type { InvokeFn } from "@taskboard/client";
import { describe, expect, it } from "vitest";

import { AppWithInvoke } from "./App";

describe("desktop App", () => {
  it("does not call fetch on first render", async () => {
    const invoke: InvokeFn = async (cmd) => {
      if (cmd === "project_list") return [] as never;
      if (cmd === "sync") {
        return { sequence: 0, projects: [], tasks: [], runs: [] } as never;
      }
      return { projects: [], sequence: 0 } as never;
    };
    const fetchCalls: string[] = [];
    const original = globalThis.fetch;
    globalThis.fetch = async (input, init) => {
      fetchCalls.push(String(input));
      return original(input, init);
    };
    try {
      render(<AppWithInvoke invoke={invoke} />);
      expect(fetchCalls).toEqual([]);
    } finally {
      globalThis.fetch = original;
    }
  });
});
