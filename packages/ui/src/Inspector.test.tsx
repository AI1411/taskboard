import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

describe("Inspector links", () => {
  it("adds a URL and a path then removes the URL", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Linked", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Linked"));
    await userEvent.type(screen.getByPlaceholderText("https:// or /path"), "https://example.com");
    await userEvent.click(screen.getByRole("button", { name: "Add link" }));
    await waitFor(() =>
      expect(transport.linkAdd).toHaveBeenCalledWith("TASK-1", {
        kind: "url",
        value: "https://example.com",
      }),
    );
    expect(await screen.findByText("https://example.com")).toBeTruthy();
    await userEvent.type(screen.getByPlaceholderText("https:// or /path"), "/tmp/notes.md");
    await userEvent.click(screen.getByRole("button", { name: "Add link" }));
    await waitFor(() =>
      expect(transport.linkAdd).toHaveBeenCalledWith("TASK-1", {
        kind: "path",
        value: "/tmp/notes.md",
      }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Remove https://example.com" }));
    await waitFor(() => expect(transport.linkRemove).toHaveBeenCalledWith("l1"));
    expect(screen.queryByText("https://example.com")).toBeNull();
  });

  it("opens URL and file links and copies a path", async () => {
    const writeText = vi.fn();
    Object.assign(navigator, { clipboard: { writeText } });
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Linked", column: "todo" });
    task.links = [
      { id: "l1", taskId: task.id, kind: "url", value: "https://example.com", sortOrder: 0 },
      { id: "l2", taskId: task.id, kind: "url", value: "file:///tmp/log.txt", sortOrder: 1 },
      { id: "l3", taskId: task.id, kind: "path", value: "/tmp/notes.md", sortOrder: 2 },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Linked"));
    const url = await screen.findByRole("link", { name: "https://example.com" });
    expect(url.getAttribute("href")).toBe("https://example.com");
    expect(url.getAttribute("target")).toBe("_blank");
    expect(screen.getByRole("link", { name: "file:///tmp/log.txt" }).getAttribute("href")).toBe(
      "file:///tmp/log.txt",
    );
    await userEvent.click(screen.getByRole("button", { name: "/tmp/notes.md" }));
    expect(writeText).toHaveBeenCalledWith("/tmp/notes.md");
    expect(await screen.findByText("Copied")).toBeTruthy();
  });
});

describe("Inspector workspace", () => {
  it("shows recorded worktree and branch", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Wt", column: "todo" });
    task.worktreePath = "/tmp/wt";
    task.branch = "cursor/foo-88ba";
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Wt"));
    expect(await screen.findByText("/tmp/wt · cursor/foo-88ba")).toBeTruthy();
  });
});

describe("Inspector checklists", () => {
  it("shows checklist items as checkboxes without changing the note", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "DoD", column: "todo" });
    task.noteMarkdown = "# Spec";
    task.checks = [
      {
        id: "k1",
        displayId: "CHECK-1",
        taskId: task.id,
        text: "Add CLI JSON contract",
        done: true,
        sortOrder: 0,
      },
      {
        id: "k2",
        displayId: "CHECK-2",
        taskId: task.id,
        text: "Write tests",
        done: false,
        sortOrder: 1,
      },
    ];
    task.checklistDone = 1;
    task.checklistTotal = 2;
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("DoD"));
    const done = await screen.findByRole("checkbox", { name: "Add CLI JSON contract" });
    const open = screen.getByRole("checkbox", { name: "Write tests" });
    expect((done as HTMLInputElement).checked).toBe(true);
    expect((open as HTMLInputElement).checked).toBe(false);
    expect((screen.getByLabelText("Note") as HTMLTextAreaElement).value).toBe("# Spec");
  });
});

describe("Inspector history", () => {
  it("shows readable run and activity rows", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "History", column: "todo" });
    const startedAt = new Date(Date.now() - 2 * 60 * 1000).toISOString();
    task.runs = [
      {
        id: "r1",
        displayId: "RUN-3",
        taskId: task.id,
        agent: "cursor",
        sessionId: null,
        status: "running",
        message: "implementing",
        waitingReason: "Need spec",
        summary: "halfway",
        startedAt,
        endedAt: null,
        revision: 1,
        createdAt: startedAt,
        updatedAt: startedAt,
      },
    ];
    task.recentActivities = [
      {
        id: "a1",
        sequence: 2,
        actorKind: "cli",
        actorLabel: "cursor",
        operation: "task.move",
        entityType: "task",
        entityId: task.id,
        previousRevision: 1,
        beforeJson: null,
        afterJson: null,
        createdAt: startedAt,
      },
      {
        id: "a2",
        sequence: 1,
        actorKind: "cli",
        actorLabel: "cursor",
        operation: "mystery.zap",
        entityType: "task",
        entityId: task.id,
        previousRevision: null,
        beforeJson: null,
        afterJson: null,
        createdAt: startedAt,
      },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("History"));
    const runId = await screen.findByText(/RUN-3/);
    expect(runId.parentElement?.textContent?.replace(/\s+/g, " ")).toMatch(
      /RUN-3 · cursor · Running · 2m ago/,
    );
    expect(screen.getByText("implementing")).toBeTruthy();
    expect(screen.getByText("Need spec")).toBeTruthy();
    expect(screen.getByText("halfway")).toBeTruthy();
    expect(screen.getByText(/Moved · cursor · 2m ago/)).toBeTruthy();
    expect(screen.getByText(/Zap · cursor · 2m ago/)).toBeTruthy();
    expect(screen.queryByText("mystery.zap")).toBeNull();
  });
});

describe("Inspector comments", () => {
  it("shows a plain comment thread", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Talk", column: "todo" });
    task.comments = [
      {
        id: "c1",
        taskId: task.id,
        actorKind: "cli",
        actorLabel: "alice",
        body: "use TDD",
        createdAt: "2026-09-16T12:00:00Z",
      },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Talk"));
    expect(await screen.findByText(/2026-09-16T12:00:00Z · alice · use TDD/)).toBeTruthy();
    expect(screen.queryByText(/marked/i)).toBeNull();
  });
});
