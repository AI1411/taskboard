import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

describe("design-review regressions", () => {
  it("New task creates a card through transport", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Untitled" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByRole("list", { name: "Todo" });
    await userEvent.click(screen.getByRole("button", { name: "New task" }));
    expect(transport.taskCreate).not.toHaveBeenCalled();
    await userEvent.type(screen.getByPlaceholderText("Task title"), "Named{Enter}");
    await waitFor(() => expect(transport.taskCreate).toHaveBeenCalled());
  });

  it("Delete asks for confirmation before removing a card", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Keep me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Keep me/ }));
    await userEvent.click(await screen.findByRole("button", { name: "Delete" }));
    expect(transport.taskDelete).not.toHaveBeenCalled();
    expect(screen.getByText("Delete Keep me?")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(transport.taskDelete).toHaveBeenCalled());
  });

  it("uses a dark color-scheme on the document", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Untitled" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByRole("list", { name: "Todo" });
    const scheme = getComputedStyle(document.documentElement).colorScheme;
    expect(scheme).toMatch(/dark/);
  });

  it("keeps four columns visible while the inspector is open", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Keep columns", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Keep columns/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    expect(screen.getByRole("list", { name: "Todo" })).toBeTruthy();
    expect(screen.getByRole("list", { name: "In Progress" })).toBeTruthy();
    expect(screen.getByRole("list", { name: "In Review" })).toBeTruthy();
    expect(screen.getByRole("list", { name: "Done" })).toBeTruthy();
  });

  it("does not render a full-board dismiss backdrop while a card is open", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Open me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Open me/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Dismiss details" })).toBeNull();
    expect(screen.getByRole("list", { name: "Todo" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Taskboard" })).toBeTruthy();
  });

  it("lets the sidebar switch projects while the inspector is open", async () => {
    const transport = fakeTransport();
    const alpha = await transport.projectAdd({ name: "Alpha" });
    await transport.projectAdd({ name: "Beta" });
    await transport.taskCreate(alpha.slug, { title: "Alpha card", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Alpha card/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Beta" }));
    expect(screen.queryByLabelText("Title")).toBeNull();
    expect(screen.queryByText("Select a card")).toBeNull();
    expect(screen.getByRole("button", { name: "Beta" }).getAttribute("aria-current")).toBe("true");
  });

  it("swaps inspector content when a second card is clicked", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "First card", column: "todo" });
    await transport.taskCreate(project.slug, { title: "Second card", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /First card/ }));
    expect((await screen.findByLabelText("Title") as HTMLInputElement).value).toBe("First card");
    expect(screen.getByRole("complementary", { name: "Task details" }).textContent).toMatch(
      /TASK-1/,
    );
    await userEvent.click(screen.getByRole("listitem", { name: /Second card/ }));
    await waitFor(() =>
      expect((screen.getByLabelText("Title") as HTMLInputElement).value).toBe("Second card"),
    );
    expect(screen.getByRole("complementary", { name: "Task details" }).textContent).toMatch(
      /TASK-2/,
    );
    expect(screen.queryByRole("button", { name: "Dismiss details" })).toBeNull();
  });

  it("closes the inspector when empty board space is clicked", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Closable", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Closable/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    await userEvent.click(screen.getByRole("list", { name: "In Progress" }));
    expect(screen.queryByLabelText("Title")).toBeNull();
    expect(screen.queryByText("Select a card")).toBeNull();
    expect(screen.getByRole("listitem", { name: /Closable/ }).getAttribute("aria-selected")).toBe(
      "false",
    );
  });

  it("moves focus into the inspector on open and back to the board on close", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Focus me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Focus me/ }));
    const panel = await screen.findByRole("complementary", { name: "Task details" });
    expect(panel.contains(document.activeElement)).toBe(true);
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("complementary", { name: "Task details" })).toBeNull();
    expect(document.activeElement).toBe(screen.getByRole("region", { name: "Board" }));
  });

  it("lets Inbox expand while the inspector is open", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const created = await transport.taskCreate(project.slug, { title: "Blocked", column: "todo" });
    created.displayStatus = "waiting";
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Blocked/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    const inbox = screen.getByRole("region", { name: "Inbox" });
    await userEvent.click(screen.getByRole("button", { name: /Inbox/ }));
    expect(inbox.textContent).toMatch(/Blocked/);
  });

  it("copies TASK-n from the card and inspector and toasts Copied", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Copy me", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("button", { name: "Copy TASK-1" }));
    expect(writeText).toHaveBeenCalledWith("TASK-1");
    expect(await screen.findByText("Copied")).toBeTruthy();
    expect(screen.getByRole("listitem", { name: /Copy me/ }).getAttribute("aria-selected")).toBe(
      "false",
    );
    await userEvent.click(screen.getByRole("listitem", { name: /Copy me/ }));
    const panel = await screen.findByRole("complementary", { name: "Task details" });
    await userEvent.click(within(panel).getByRole("button", { name: "Copy TASK-1" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledTimes(2));
    expect(screen.getByText("Copied")).toBeTruthy();
  });
});
