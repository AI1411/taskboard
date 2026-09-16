import { describe, expect, it, vi } from "vitest";

import { readBoardRoute, replaceBoardRoute } from "./route";

describe("web board route", () => {
  it("reads project and task from the query string", () => {
    expect(readBoardRoute("?project=alpha&task=TASK-1")).toEqual({
      project: "alpha",
      task: "TASK-1",
    });
  });

  it("replaceState writes project and task", () => {
    const replaceState = vi.fn();
    const next = replaceBoardRoute(
      { project: "alpha", task: "TASK-1" },
      {
        href: "http://localhost:5173/",
        pathname: "/",
        search: "",
        hash: "",
      },
      { replaceState },
    );
    expect(next).toBe("/?project=alpha&task=TASK-1");
    expect(replaceState).toHaveBeenCalledWith(null, "", "/?project=alpha&task=TASK-1");
  });
});
