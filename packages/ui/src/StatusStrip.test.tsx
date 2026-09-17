import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

describe("Status strip", () => {
  it("shows tb status counts and focuses the first ready card", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Ready one", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("button", { name: /Ready 1/ }));
    await waitFor(() => expect(transport.taskShow).toHaveBeenCalledWith("TASK-1"));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
  });

  it("shows one occupancy line for a colliding worktree", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const a = await transport.taskCreate(project.slug, { title: "A", column: "todo" });
    const b = await transport.taskCreate(project.slug, { title: "B", column: "todo" });
    a.worktreePath = "/tmp/shared";
    b.worktreePath = "/tmp/shared";
    a.displayStatus = "running";
    b.displayStatus = "running";
    const summaryA = (await transport.taskList(project.slug)).find((t) => t.displayId === "TASK-1");
    const summaryB = (await transport.taskList(project.slug)).find((t) => t.displayId === "TASK-2");
    if (summaryA) {
      summaryA.worktreePath = "/tmp/shared";
      summaryA.displayStatus = "running";
    }
    if (summaryB) {
      summaryB.worktreePath = "/tmp/shared";
      summaryB.displayStatus = "running";
    }
    render(<TaskboardApp transport={transport} />);
    const line = await screen.findByLabelText("Occupancy");
    expect(line.textContent).toContain("/tmp/shared");
    expect(line.textContent).toMatch(/TASK-1/);
    expect(line.textContent).toMatch(/TASK-2/);
  });
});
