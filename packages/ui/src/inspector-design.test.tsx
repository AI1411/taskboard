import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

describe("design-review regressions", () => {
  it("New task creates a card through transport", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Untitled" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByRole("list", { name: "Todo" });
    await userEvent.click(screen.getByRole("button", { name: "New task" }));
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
    await userEvent.click(screen.getByRole("button", { name: "Move to Trash" }));
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

  it("closes the inspector when dismiss is clicked", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Untitled" });
    await transport.taskCreate(project.slug, { title: "Closable", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Closable/ }));
    expect(await screen.findByLabelText("Title")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Dismiss details" }));
    expect(screen.queryByLabelText("Title")).toBeNull();
    expect(screen.getByText("Select a card")).toBeTruthy();
  });
});
