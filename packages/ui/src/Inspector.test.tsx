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
  it("toggles a check and adds a row", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "DoD", column: "todo" });
    task.checks = [
      {
        id: "k1",
        displayId: "CHECK-1",
        taskId: task.id,
        text: "Write tests",
        done: false,
        sortOrder: 0,
      },
    ];
    task.checklistDone = 0;
    task.checklistTotal = 1;
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("DoD"));
    await userEvent.click(await screen.findByRole("checkbox", { name: "Write tests" }));
    await waitFor(() => expect(transport.checkToggle).toHaveBeenCalledWith("CHECK-1"));
    await userEvent.type(screen.getByPlaceholderText("Add a check"), "Ship it");
    await userEvent.click(screen.getByRole("button", { name: "Add check" }));
    await waitFor(() => expect(transport.checkAdd).toHaveBeenCalledWith("TASK-1", "Ship it"));
    expect(await screen.findByRole("checkbox", { name: "Ship it" })).toBeTruthy();
  });

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

  it("cancels a running run from the inspector", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Stuck", column: "todo" });
    const startedAt = "2026-09-16T12:00:00Z";
    task.displayStatus = "running";
    task.runs = [
      {
        id: "r1",
        displayId: "RUN-1",
        taskId: task.id,
        agent: "cursor",
        sessionId: null,
        status: "running",
        message: "implementing",
        waitingReason: null,
        summary: null,
        startedAt,
        endedAt: null,
        revision: 1,
        createdAt: startedAt,
        updatedAt: startedAt,
        worktreePath: null,
        branch: null,
      },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Stuck"));
    await userEvent.click(screen.getByRole("button", { name: "Cancel RUN-1" }));
    await waitFor(() =>
      expect(transport.runPatch).toHaveBeenCalledWith("RUN-1", { op: "cancel" }),
    );
  });

  it("cancels a stale-badged running run from the inspector", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Heartbeat", column: "todo" });
    const startedAt = "2026-09-16T12:00:00Z";
    task.displayStatus = "running";
    task.stale = true;
    task.runs = [
      {
        id: "r1",
        displayId: "RUN-2",
        taskId: task.id,
        agent: "cursor",
        sessionId: null,
        status: "running",
        message: null,
        waitingReason: null,
        summary: null,
        startedAt,
        endedAt: null,
        revision: 1,
        createdAt: startedAt,
        updatedAt: startedAt,
        worktreePath: null,
        branch: null,
      },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Heartbeat"));
    await userEvent.click(screen.getByRole("button", { name: "Cancel RUN-2" }));
    await waitFor(() =>
      expect(transport.runPatch).toHaveBeenCalledWith("RUN-2", { op: "cancel" }),
    );
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

  it("adds a comment on Enter without changing the note", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Talk", column: "todo" });
    task.noteMarkdown = "# Spec";
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Talk"));
    await userEvent.type(screen.getByPlaceholderText("Add a comment"), "use TDD{Enter}");
    await waitFor(() =>
      expect(transport.commentAdd).toHaveBeenCalledWith("TASK-1", "use TDD", false),
    );
    expect(await screen.findByText(/local-ui · use TDD/)).toBeTruthy();
    expect((screen.getByLabelText("Note") as HTMLTextAreaElement).value).toBe("# Spec");
  });

  it("can continue a waiting run when sending a comment", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Wait", column: "todo" });
    const startedAt = "2026-09-16T12:00:00Z";
    task.displayStatus = "waiting";
    task.waitingReason = "Need spec";
    task.runs = [
      {
        id: "r1",
        displayId: "RUN-1",
        taskId: task.id,
        agent: "cursor",
        sessionId: null,
        status: "waiting",
        message: null,
        waitingReason: "Need spec",
        summary: null,
        startedAt,
        endedAt: null,
        revision: 1,
        createdAt: startedAt,
        updatedAt: startedAt,
        worktreePath: null,
        branch: null,
      },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Wait"));
    await userEvent.click(screen.getByLabelText("If Waiting, Continue"));
    await userEvent.type(screen.getByPlaceholderText("Add a comment"), "here is spec{Enter}");
    await waitFor(() =>
      expect(transport.commentAdd).toHaveBeenCalledWith("TASK-1", "here is spec", true),
    );
  });
});

describe("Inspector blocked-by", () => {
  it("adds a TASK-n blocked-by link from a separate input", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Blocker", column: "todo" });
    await transport.taskCreate(project.slug, { title: "Blocked", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Blocked"));
    await userEvent.type(screen.getByPlaceholderText("TASK-n"), "TASK-1");
    await userEvent.click(screen.getByRole("button", { name: "Add blocked-by" }));
    await waitFor(() =>
      expect(transport.linkAdd).toHaveBeenCalledWith("TASK-2", {
        kind: "blocked_by",
        value: "TASK-1",
      }),
    );
    expect(await screen.findByRole("button", { name: "TASK-1" })).toBeTruthy();
    expect(screen.getByPlaceholderText("https:// or /path")).toBeTruthy();
  });

  it("toasts validation_error on a blocked-by cycle", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "A", column: "todo" });
    await transport.taskCreate(project.slug, { title: "B", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("B"));
    await userEvent.type(screen.getByPlaceholderText("TASK-n"), "TASK-1");
    await userEvent.click(screen.getByRole("button", { name: "Add blocked-by" }));
    await waitFor(() => expect(transport.linkAdd).toHaveBeenCalled());
    await userEvent.click(await screen.findByText("A"));
    await userEvent.type(screen.getByPlaceholderText("TASK-n"), "TASK-2");
    await userEvent.click(screen.getByRole("button", { name: "Add blocked-by" }));
    expect(await screen.findByText("blocked-by cycle")).toBeTruthy();
  });
});

describe("Inspector review", () => {
  it("approves an in-review card", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Ship", column: "in-review" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Ship"));
    await userEvent.type(screen.getByPlaceholderText("Review comment"), "lgtm");
    await userEvent.click(screen.getByRole("button", { name: "Approve" }));
    await waitFor(() =>
      expect(transport.review).toHaveBeenCalledWith("TASK-1", { action: "approve", text: "lgtm" }),
    );
  });

  it("hides review verbs when the card is not in review", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Todo", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Todo"));
    expect(screen.queryByRole("button", { name: "Approve" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Request changes" })).toBeNull();
  });
});
