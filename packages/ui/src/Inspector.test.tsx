import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { TaskDetail } from "@taskboard/types";
import { describe, expect, it, vi } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { Inspector } from "./Inspector";
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
  it("shows recorded worktree and branch in editable fields", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Wt", column: "todo" });
    task.worktreePath = "/tmp/wt";
    task.branch = "cursor/foo-88ba";
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Wt"));
    expect((await screen.findByLabelText("Worktree") as HTMLInputElement).value).toBe("/tmp/wt");
    expect((screen.getByLabelText("Branch") as HTMLInputElement).value).toBe("cursor/foo-88ba");
  });

  it("writes worktree and branch through taskUpdate and clears with empty string", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Wt", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Wt"));
    const worktree = await screen.findByLabelText("Worktree");
    await userEvent.clear(worktree);
    await userEvent.type(worktree, "/tmp/wt");
    await userEvent.tab();
    await waitFor(() =>
      expect(transport.taskUpdate).toHaveBeenCalledWith(
        "TASK-1",
        { worktreePath: "/tmp/wt" },
        expect.anything(),
      ),
    );
    const branch = screen.getByLabelText("Branch");
    await userEvent.clear(branch);
    await userEvent.type(branch, "cursor/foo-88ba");
    await userEvent.tab();
    await waitFor(() =>
      expect(transport.taskUpdate).toHaveBeenCalledWith(
        "TASK-1",
        { branch: "cursor/foo-88ba" },
        expect.anything(),
      ),
    );
    await userEvent.clear(worktree);
    await userEvent.tab();
    await waitFor(() =>
      expect(transport.taskUpdate).toHaveBeenCalledWith(
        "TASK-1",
        { worktreePath: "" },
        expect.anything(),
      ),
    );
    expect((worktree as HTMLInputElement).value).toBe("");
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

  it("removes a check from the inspector", async () => {
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
    await userEvent.click(await screen.findByRole("button", { name: "Remove CHECK-1" }));
    await waitFor(() => expect(transport.checkRemove).toHaveBeenCalledWith("CHECK-1"));
    expect(screen.queryByRole("checkbox", { name: "Write tests" })).toBeNull();
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
        sequence: 8,
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
        sequence: 7,
        actorKind: "cli",
        actorLabel: "cursor",
        operation: "comment.add",
        entityType: "comment",
        entityId: task.id,
        previousRevision: null,
        beforeJson: null,
        afterJson: null,
        createdAt: startedAt,
      },
      {
        id: "a3",
        sequence: 6,
        actorKind: "cli",
        actorLabel: "cursor",
        operation: "check.add",
        entityType: "check",
        entityId: task.id,
        previousRevision: null,
        beforeJson: null,
        afterJson: null,
        createdAt: startedAt,
      },
      {
        id: "a4",
        sequence: 5,
        actorKind: "cli",
        actorLabel: "cursor",
        operation: "check.toggle",
        entityType: "check",
        entityId: task.id,
        previousRevision: null,
        beforeJson: null,
        afterJson: null,
        createdAt: startedAt,
      },
      {
        id: "a5",
        sequence: 4,
        actorKind: "cli",
        actorLabel: "cursor",
        operation: "task.review",
        entityType: "task",
        entityId: task.id,
        previousRevision: null,
        beforeJson: null,
        afterJson: null,
        createdAt: startedAt,
      },
      {
        id: "a6",
        sequence: 3,
        actorKind: "cli",
        actorLabel: "cursor",
        operation: "task.spawn",
        entityType: "task",
        entityId: task.id,
        previousRevision: null,
        beforeJson: null,
        afterJson: null,
        createdAt: startedAt,
      },
      {
        id: "a7",
        sequence: 2,
        actorKind: "cli",
        actorLabel: "cursor",
        operation: "run.cancel",
        entityType: "run",
        entityId: task.id,
        previousRevision: null,
        beforeJson: null,
        afterJson: null,
        createdAt: startedAt,
      },
      {
        id: "a8",
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
    expect(screen.getByText(/Commented · cursor · 2m ago/)).toBeTruthy();
    expect(screen.getByText(/Check added · cursor · 2m ago/)).toBeTruthy();
    expect(screen.getByText(/Check toggled · cursor · 2m ago/)).toBeTruthy();
    expect(screen.getByText(/Review · cursor · 2m ago/)).toBeTruthy();
    expect(screen.getByText(/Spawned · cursor · 2m ago/)).toBeTruthy();
    expect(screen.getByText(/Run canceled · cursor · 2m ago/)).toBeTruthy();
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
    const createdAt = new Date(Date.now() - 2 * 60 * 1000).toISOString();
    task.comments = [
      {
        id: "c1",
        taskId: task.id,
        actorKind: "cli",
        actorLabel: "alice",
        body: "use TDD",
        createdAt,
      },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Talk"));
    expect(await screen.findByText(/2m ago · alice · use TDD/)).toBeTruthy();
    expect(screen.queryByText(/2026-09-16T12:00:00Z/)).toBeNull();
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

  it("removes only the latest comment", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Talk", column: "todo" });
    const createdAt = new Date(Date.now() - 2 * 60 * 1000).toISOString();
    task.comments = [
      {
        id: "c1",
        taskId: task.id,
        actorKind: "cli",
        actorLabel: "alice",
        body: "first",
        createdAt,
      },
      {
        id: "c2",
        taskId: task.id,
        actorKind: "cli",
        actorLabel: "bob",
        body: "latest",
        createdAt,
      },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Talk"));
    expect(screen.getByRole("button", { name: "Remove latest comment" })).toBeTruthy();
    expect(screen.getAllByRole("button", { name: /Remove/ }).filter((btn) =>
      btn.getAttribute("aria-label") === "Remove latest comment",
    )).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "Remove latest comment" }));
    await waitFor(() => expect(transport.commentRemoveLatest).toHaveBeenCalledWith("TASK-1"));
    expect(screen.queryByText(/bob · latest/)).toBeNull();
    expect(screen.getByText(/alice · first/)).toBeTruthy();
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

  it("spawns a todo child and blocks the parent without moving columns", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Parent", column: "in-progress" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Parent"));
    await userEvent.type(screen.getByPlaceholderText("Spawn title"), "Child");
    await userEvent.click(screen.getByRole("button", { name: "Spawn" }));
    await waitFor(() => expect(transport.taskSpawn).toHaveBeenCalledWith("TASK-1", ["Child"]));
    expect(await screen.findByText("Child")).toBeTruthy();
    await userEvent.click(await screen.findByText("Parent"));
    expect(await screen.findByRole("button", { name: "TASK-2" })).toBeTruthy();
    expect((screen.getByLabelText("Column") as HTMLSelectElement).value).toBe("in-progress");
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

function detail(noteMarkdown: string, revision: number): TaskDetail {
  return {
    id: "t1",
    displayId: "TASK-1",
    projectId: "p1",
    title: "Spec",
    column: "in-progress",
    urgent: false,
    revision,
    displayStatus: "running",
    runMessage: null,
    waitingReason: null,
    reply: null,
    blockedBy: [],
    blocks: [],
    stale: false,
    checklistDone: 0,
    checklistTotal: 0,
    worktreePath: null,
    branch: null,
    noteMarkdown,
    links: [],
    runs: [],
    comments: [],
    checks: [],
    recentActivities: [],
  };
}

function renderInspector(task: TaskDetail, onNoteChange: (markdown: string) => void) {
  return render(
    <Inspector
      task={task}
      open
      onTitleCommit={() => {}}
      onColumnChange={() => {}}
      onUrgentChange={() => {}}
      onNoteChange={onNoteChange}
      onDelete={() => {}}
    />,
  );
}

describe("Inspector note drafts", () => {
  it("keeps an agent note when the server value changes under a clean draft", async () => {
    const onNoteChange = vi.fn();
    const view = renderInspector(detail("", 1), onNoteChange);
    view.rerender(
      <Inspector
        task={detail("# written by agent", 2)}
        open
        onTitleCommit={() => {}}
        onColumnChange={() => {}}
        onUrgentChange={() => {}}
        onNoteChange={onNoteChange}
        onDelete={() => {}}
      />,
    );
    expect((screen.getByLabelText("Note") as HTMLTextAreaElement).value).toBe(
      "# written by agent",
    );
    vi.useFakeTimers();
    try {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(400);
      });
    } finally {
      vi.useRealTimers();
    }
    expect(onNoteChange).not.toHaveBeenCalled();
  });

  it("stops autosave and offers keep-mine or load-server when a dirty note is stale", async () => {
    const onNoteChange = vi.fn();
    const view = renderInspector(detail("", 1), onNoteChange);
    fireEvent.change(screen.getByLabelText("Note"), { target: { value: "my draft" } });
    view.rerender(
      <Inspector
        task={detail("# written by agent", 2)}
        open
        onTitleCommit={() => {}}
        onColumnChange={() => {}}
        onUrgentChange={() => {}}
        onNoteChange={onNoteChange}
        onDelete={() => {}}
      />,
    );
    expect(screen.getByRole("status").textContent).toContain("Updated elsewhere");
    vi.useFakeTimers();
    try {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(400);
      });
    } finally {
      vi.useRealTimers();
    }
    expect(onNoteChange).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Load server" }));
    expect((screen.getByLabelText("Note") as HTMLTextAreaElement).value).toBe(
      "# written by agent",
    );
  });
});
