import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

describe("TrashPanel", () => {
  it("shows empty copy when trash has nothing", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Alpha" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByRole("list", { name: "Todo" });
    await userEvent.click(screen.getByRole("button", { name: "Trash" }));
    expect(await screen.findByText("Trash is empty")).toBeTruthy();
  });

  it("restores a deleted task from the panel", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Gone", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Gone"));
    await userEvent.click(await screen.findByRole("button", { name: "Delete" }));
    expect(transport.taskDelete).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(transport.taskDelete).toHaveBeenCalled());
    await userEvent.click(screen.getByRole("button", { name: "Trash" }));
    expect(await screen.findByText("Gone")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Restore TASK-1" }));
    await waitFor(() => expect(transport.taskRestore).toHaveBeenCalledWith("TASK-1"));
    expect(await screen.findByText("Gone")).toBeTruthy();
  });

  it("restores a deleted project from the panel", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Alpha" });
    await transport.projectDelete("alpha");
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(screen.getByRole("button", { name: "Trash" }));
    expect(await screen.findByText("Alpha")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Restore alpha" }));
    await waitFor(() => expect(transport.projectRestore).toHaveBeenCalledWith("alpha"));
  });

  it("selecting a trashed task shows Restore in the inspector", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Gone", column: "todo" });
    await transport.taskDelete("TASK-1");
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(screen.getByRole("button", { name: "Trash" }));
    await userEvent.click(await screen.findByText("Gone"));
    expect(await screen.findByRole("button", { name: /^Restore$/ })).toBeTruthy();
    expect(screen.queryByRole("button", { name: /^Delete$/ })).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: /^Restore$/ }));
    await waitFor(() => expect(transport.taskRestore).toHaveBeenCalledWith("TASK-1"));
  });
});
