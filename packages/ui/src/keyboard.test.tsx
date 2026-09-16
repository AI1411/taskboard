import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

describe("keyboard", () => {
  it("n opens composer and Enter creates in the current column", async () => {
    const transport = fakeTransport();
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(screen.getByRole("button", { name: "New project" }));
    await userEvent.type(screen.getByPlaceholderText("Project name"), "Alpha{Enter}");
    await screen.findByRole("list", { name: "Todo" });
    await userEvent.keyboard("2");
    await userEvent.keyboard("n");
    expect(transport.taskCreate).not.toHaveBeenCalled();
    await userEvent.type(screen.getByPlaceholderText("Task title"), "Ship it{Enter}");
    await waitFor(() =>
      expect(transport.taskCreate).toHaveBeenCalledWith("alpha", {
        title: "Ship it",
        column: "in-progress",
      }),
    );
  });

  it("p opens project composer and does not insert Untitled", async () => {
    const transport = fakeTransport();
    render(<TaskboardApp transport={transport} />);
    await userEvent.keyboard("p");
    expect(transport.projectAdd).not.toHaveBeenCalled();
    await userEvent.type(screen.getByPlaceholderText("Project name"), "Beta{Enter}");
    await waitFor(() => expect(transport.projectAdd).toHaveBeenCalledWith({ name: "Beta" }));
  });

  it("u toggles urgent on the selected card", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Urgent me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Urgent me"));
    await userEvent.keyboard("u");
    await waitFor(() => expect(transport.taskUrgent).toHaveBeenCalled());
  });

  it("slash focuses search and Escape clears it", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Untitled" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByRole("list", { name: "Todo" });
    await userEvent.keyboard("/");
    const search = screen.getByRole("searchbox", { name: "Search" });
    expect(document.activeElement).toBe(search);
    await userEvent.keyboard("hello");
    expect((search as HTMLInputElement).value).toBe("hello");
    await userEvent.keyboard("{Escape}");
    expect((search as HTMLInputElement).value).toBe("");
  });

  it("1-4 jump columns so n creates in the current column", async () => {
    const transport = fakeTransport();
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(screen.getByRole("button", { name: "New project" }));
    await userEvent.type(screen.getByPlaceholderText("Project name"), "Alpha{Enter}");
    await screen.findByRole("list", { name: "Todo" });
    await userEvent.keyboard("2");
    await userEvent.keyboard("n");
    expect(transport.taskCreate).not.toHaveBeenCalled();
    await userEvent.type(screen.getByPlaceholderText("Task title"), "Ship it{Enter}");
    await waitFor(() =>
      expect(transport.taskCreate).toHaveBeenCalledWith("alpha", {
        title: "Ship it",
        column: "in-progress",
      }),
    );
  });

  it("j and k move selection within the current column", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "First", column: "todo" });
    await transport.taskCreate(project.slug, { title: "Second", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByText("First");
    await userEvent.keyboard("j");
    expect(screen.getByRole("listitem", { name: /First/ }).getAttribute("aria-selected")).toBe("true");
    await userEvent.keyboard("j");
    expect(screen.getByRole("listitem", { name: /Second/ }).getAttribute("aria-selected")).toBe("true");
    await userEvent.keyboard("k");
    expect(screen.getByRole("listitem", { name: /First/ }).getAttribute("aria-selected")).toBe("true");
  });

  it("l moves the selected card to the next column", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Move me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Move me"));
    await userEvent.keyboard("l");
    await waitFor(() => expect(transport.taskMove).toHaveBeenCalled());
  });

  it("shift+l changes current column without moving", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Stay", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Stay"));
    await userEvent.keyboard("{Shift>}l{/Shift}");
    expect(transport.taskMove).not.toHaveBeenCalled();
    await userEvent.keyboard("n");
    await userEvent.type(screen.getByPlaceholderText("Task title"), "Next{Enter}");
    await waitFor(() =>
      expect(transport.taskCreate).toHaveBeenCalledWith("alpha", {
        title: "Next",
        column: "in-progress",
      }),
    );
  });

  it("l move shows undo toast", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Move me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Move me"));
    await userEvent.keyboard("l");
    await waitFor(() => expect(transport.taskMove).toHaveBeenCalled());
    expect(await screen.findByText("Moved to In Progress")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Undo" }));
    await waitFor(() => expect(transport.undo).toHaveBeenCalled());
    expect(await screen.findByText("Undone")).toBeTruthy();
  });

  it("delete toast offers undo", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Drop me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Drop me"));
    await userEvent.keyboard("{Delete}");
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(await screen.findByText("Deleted TASK-1")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Undo" }));
    await waitFor(() => expect(transport.undo).toHaveBeenCalled());
  });

  it("opens the last live project from uiState", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Alpha" });
    await transport.projectAdd({ name: "Beta" });
    vi.mocked(transport.uiState).mockResolvedValue({ lastProjectSlug: "beta" });
    render(<TaskboardApp transport={transport} />);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Beta" }).getAttribute("aria-current")).toBe(
        "true",
      ),
    );
  });

  it("persists the selected project slug", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Alpha" });
    await transport.projectAdd({ name: "Beta" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("button", { name: "Beta" }));
    await waitFor(() => expect(transport.uiStateSet).toHaveBeenCalledWith("beta"));
  });

  it("toasts taskShow errors that are not not_found", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Broken", column: "todo" });
    vi.mocked(transport.taskShow).mockRejectedValueOnce(new Error("nope"));
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Broken"));
    expect(await screen.findByText("nope")).toBeTruthy();
  });

  it("meta+z calls undo", async () => {
    const transport = fakeTransport();
    render(<TaskboardApp transport={transport} />);
    await userEvent.keyboard("{Meta>}z{/Meta}");
    expect(transport.undo).toHaveBeenCalled();
  });

  it("delete key confirms and deletes the selected card", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Drop me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Drop me"));
    await userEvent.keyboard("{Delete}");
    expect(transport.taskDelete).not.toHaveBeenCalled();
    expect(screen.getByText("Delete Drop me?")).toBeTruthy();
    await userEvent.keyboard("{Escape}");
    expect(transport.taskDelete).not.toHaveBeenCalled();
    await userEvent.keyboard("{Backspace}");
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(transport.taskDelete).toHaveBeenCalled());
  });

  it("question mark toggles shortcut legend", async () => {
    const transport = fakeTransport();
    render(<TaskboardApp transport={transport} />);
    await userEvent.keyboard("?");
    const dialog = screen.getByRole("dialog", { name: "Keyboard shortcuts" });
    expect(dialog).toBeTruthy();
    expect(dialog.textContent).toMatch(/New task/);
    await userEvent.keyboard("?");
    expect(screen.queryByRole("dialog", { name: "Keyboard shortcuts" })).toBeNull();
  });

  it("question mark in a field does not open the legend", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Alpha" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByRole("list", { name: "Todo" });
    await userEvent.keyboard("/");
    await userEvent.keyboard("?");
    expect(screen.queryByRole("dialog", { name: "Keyboard shortcuts" })).toBeNull();
  });

  it("Escape closes the inspector and clears the selection", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Inspect me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Inspect me/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByLabelText("Title")).toBeNull();
    expect(screen.queryByText("Select a card")).toBeNull();
    expect(screen.getByRole("listitem", { name: /Inspect me/ }).getAttribute("aria-selected")).toBe(
      "false",
    );
  });

  it("Enter opens the inspector for the selected card", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Inspect me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByText("Inspect me");
    await userEvent.keyboard("j");
    await userEvent.keyboard("{Enter}");
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    expect(screen.queryByText("Select a card")).toBeNull();
  });
});
