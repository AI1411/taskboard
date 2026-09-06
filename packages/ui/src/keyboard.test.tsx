import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

describe("keyboard", () => {
  it("n creates a task through transport", async () => {
    const transport = fakeTransport();
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(screen.getByRole("button", { name: "New project" }));
    // fakeTransport.projectAdd resolves; press n
    await userEvent.keyboard("n");
    await waitFor(() => expect(transport.taskCreate).toHaveBeenCalled());
  });

  it("p creates a project through transport", async () => {
    const transport = fakeTransport();
    render(<TaskboardApp transport={transport} />);
    await userEvent.keyboard("p");
    expect(transport.projectAdd).toHaveBeenCalledWith({ name: "Untitled" });
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
    await waitFor(() => expect(transport.projectAdd).toHaveBeenCalled());
    await screen.findByRole("list", { name: "Todo" });
    await userEvent.keyboard("2");
    await userEvent.keyboard("n");
    await waitFor(() =>
      expect(transport.taskCreate).toHaveBeenCalledWith("untitled", {
        title: "Untitled",
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

  it("meta+z calls undo", async () => {
    const transport = fakeTransport();
    render(<TaskboardApp transport={transport} />);
    await userEvent.keyboard("{Meta>}z{/Meta}");
    expect(transport.undo).toHaveBeenCalled();
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
