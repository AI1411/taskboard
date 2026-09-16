import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { TaskDetail } from "@taskboard/types";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./TaskboardApp";

describe("InboxStrip", () => {
  it("hides inbox when empty and shows waiting row when present", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Idle", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByText("Idle");
    expect(screen.queryByText(/Inbox ·/)).toBeNull();
  });

  it("lists waiting cards and jump selects them", async () => {
    const transport = fakeTransport();
    const a = await transport.projectAdd({ name: "Alpha" });
    const b = await transport.projectAdd({ name: "Beta" });
    const t1 = await transport.taskCreate(a.slug, { title: "Wait A", column: "todo" });
    const t2 = await transport.taskCreate(b.slug, { title: "Wait B", column: "todo" });
    (t1 as TaskDetail).displayStatus = "waiting";
    (t2 as TaskDetail).displayStatus = "waiting";
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("button", { name: "Alpha" }));
    expect(await screen.findByText(/Inbox · 1/)).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: /Inbox/ }));
    await userEvent.click(screen.getByRole("button", { name: "All projects" }));
    expect(await screen.findByText(/Inbox · 2/)).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: /Wait B/ }));
    expect(await screen.findByDisplayValue("Wait B")).toBeTruthy();
  });

  it("i toggles the inbox strip expanded", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Wait A", column: "todo" });
    (task as TaskDetail).displayStatus = "waiting";
    render(<TaskboardApp transport={transport} />);
    await screen.findByText(/Inbox · 1/);
    expect(screen.queryByRole("button", { name: /Wait A/ })).toBeNull();
    await userEvent.keyboard("i");
    expect(await screen.findByRole("button", { name: /Wait A/ })).toBeTruthy();
    await userEvent.keyboard("i");
    await waitFor(() => expect(screen.queryByRole("button", { name: /Wait A/ })).toBeNull());
  });
});
