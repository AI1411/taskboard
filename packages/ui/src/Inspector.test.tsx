import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

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
