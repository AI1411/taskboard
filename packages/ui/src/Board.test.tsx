import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

vi.mock("./Board", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./Board")>();
  return {
    ...actual,
    Board: (props: ComponentProps<typeof actual.Board>) => (
      <>
        <actual.Board {...props} />
        <button type="button" onClick={() => props.onReorder("TASK-1", "TASK-2")}>
          Test reorder
        </button>
      </>
    ),
  };
});

describe("Board", () => {
  it("shows empty copy when there are no projects", async () => {
    render(<TaskboardApp transport={fakeTransport()} />);
    expect(await screen.findByText("Create a project to start a board")).toBeTruthy();
  });

  it("New project composer commits a named project", async () => {
    const transport = fakeTransport();
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(screen.getByRole("button", { name: "New project" }));
    expect(transport.projectAdd).not.toHaveBeenCalled();
    await userEvent.type(screen.getByPlaceholderText("Project name"), "Renai Sim{Enter}");
    await waitFor(() => expect(transport.projectAdd).toHaveBeenCalledWith({ name: "Renai Sim" }));
  });

  it("renders four columns with exact aria-labels after a project exists", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Untitled" });
    render(<TaskboardApp transport={transport} />);
    expect(await screen.findByRole("list", { name: "Todo" })).toBeTruthy();
    expect(screen.getByRole("list", { name: "In Progress" })).toBeTruthy();
    expect(screen.getByRole("list", { name: "In Review" })).toBeTruthy();
    expect(screen.getByRole("list", { name: "Done" })).toBeTruthy();
  });

  it("loads first project tasks on mount as listitems", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Ship board", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    expect(transport.projectList).toHaveBeenCalledWith(false);
    expect(await screen.findByText("Ship board")).toBeTruthy();
    expect(screen.getByText("TASK-1")).toBeTruthy();
    expect(screen.getByRole("listitem", { name: /Ship board/ })).toBeTruthy();
    await waitFor(() => expect(transport.taskList).toHaveBeenCalledWith(project.slug));
  });

  it("does not show inspector empty copy until a card is selected", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Untitled" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByRole("list", { name: "Todo" });
    expect(screen.queryByText("Select a card")).toBeNull();
    expect(screen.queryByLabelText("Title")).toBeNull();
  });

  it("filters by TASK-n, status badge, and run text", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Alpha task", column: "todo" });
    await transport.taskCreate(project.slug, { title: "Beta item", column: "todo" });
    const listed = await transport.taskList(project.slug);
    const alpha = listed.find((task) => task.title === "Alpha task");
    const beta = listed.find((task) => task.title === "Beta item");
    if (alpha) {
      alpha.displayStatus = "waiting";
      alpha.waitingReason = "Need spec";
    }
    if (beta) beta.runMessage = "compiling kernel";
    render(<TaskboardApp transport={transport} />);
    expect(await screen.findByText("Alpha task")).toBeTruthy();
    expect(screen.getByPlaceholderText("Search title, TASK-n, status")).toBeTruthy();
    await userEvent.keyboard("/");
    await userEvent.keyboard("task-2");
    expect(screen.queryByText("Alpha task")).toBeNull();
    expect(screen.getByText("Beta item")).toBeTruthy();
    await userEvent.keyboard("{Escape}");
    await userEvent.keyboard("/");
    await userEvent.keyboard("waiting");
    expect(screen.getByText("Alpha task")).toBeTruthy();
    expect(screen.queryByText("Beta item")).toBeNull();
    await userEvent.keyboard("{Escape}");
    await userEvent.keyboard("/");
    await userEvent.keyboard("kernel");
    expect(screen.getByText("Beta item")).toBeTruthy();
    expect(screen.queryByText("Alpha task")).toBeNull();
  });

  it("filters cards by title substring", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Alpha task", column: "todo" });
    await transport.taskCreate(project.slug, { title: "Beta item", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    expect(await screen.findByText("Alpha task")).toBeTruthy();
    await userEvent.keyboard("/");
    await userEvent.keyboard("alpha");
    expect(screen.getByText("Alpha task")).toBeTruthy();
    expect(screen.queryByText("Beta item")).toBeNull();
  });

  it("taskNoteSet after title commit uses the updated revision", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Old title", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Old title"));

    const titleInput = await screen.findByLabelText("Title");
    await userEvent.clear(titleInput);
    await userEvent.type(titleInput, "New title");
    fireEvent.blur(titleInput);
    await waitFor(() => expect(transport.taskUpdate).toHaveBeenCalled());
    const afterTitle = await vi.mocked(transport.taskUpdate).mock.results.at(-1)!.value;
    const titleRevision = afterTitle.revision;
    expect(titleRevision).toBeGreaterThan(1);

    vi.useFakeTimers();
    try {
      fireEvent.change(screen.getByLabelText("Note"), { target: { value: "updated note" } });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(400);
      });
      expect(transport.taskNoteSet).toHaveBeenCalledWith("TASK-1", "updated note", titleRevision);
    } finally {
      vi.useRealTimers();
    }
  });

  it("taskNoteSet after reorder uses the updated revision", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "First", column: "todo" });
    await transport.taskCreate(project.slug, { title: "Second", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("First"));
    await screen.findByLabelText("Note");

    await userEvent.click(screen.getByRole("button", { name: "Test reorder" }));
    await waitFor(() => expect(transport.taskReorder).toHaveBeenCalled());
    const afterReorder = await vi.mocked(transport.taskReorder).mock.results.at(-1)!.value;
    const reorderRevision = afterReorder.revision;
    expect(reorderRevision).toBeGreaterThan(1);

    vi.useFakeTimers();
    try {
      fireEvent.change(screen.getByLabelText("Note"), { target: { value: "after reorder" } });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(400);
      });
      expect(transport.taskNoteSet).toHaveBeenCalledWith("TASK-1", "after reorder", reorderRevision);
    } finally {
      vi.useRealTimers();
    }
  });
});
