import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

describe("TaskboardApp route", () => {
  it("selects project and task from route props", async () => {
    const onRouteChange = vi.fn();
    const transport = fakeTransport();
    const alpha = await transport.projectAdd({ name: "Alpha" });
    await transport.projectAdd({ name: "Beta" });
    await transport.taskCreate(alpha.slug, { title: "Routed", column: "todo" });
    render(
      <TaskboardApp
        transport={transport}
        route={{ project: "alpha", task: "TASK-1" }}
        onRouteChange={onRouteChange}
      />,
    );
    expect(await screen.findByDisplayValue("Routed")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Alpha" }).getAttribute("aria-current")).toBe("true");
    await waitFor(() =>
      expect(onRouteChange).toHaveBeenCalledWith({ project: "alpha", task: "TASK-1" }),
    );
  });

  it("shows the board without a card when the route is invalid", async () => {
    const onRouteChange = vi.fn();
    const transport = fakeTransport();
    const alpha = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(alpha.slug, { title: "Hidden", column: "todo" });
    render(
      <TaskboardApp
        transport={transport}
        route={{ project: "missing", task: "TASK-99" }}
        onRouteChange={onRouteChange}
      />,
    );
    expect(await screen.findByRole("listitem", { name: /Hidden/ })).toBeTruthy();
    expect(screen.queryByLabelText("Title")).toBeNull();
    await waitFor(() =>
      expect(onRouteChange).toHaveBeenCalledWith({ project: "alpha", task: null }),
    );
  });

  it("writes the route when a card is selected", async () => {
    const onRouteChange = vi.fn();
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Pick me", column: "todo" });
    render(<TaskboardApp transport={transport} onRouteChange={onRouteChange} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Pick me/ }));
    await waitFor(() =>
      expect(onRouteChange).toHaveBeenCalledWith({ project: "alpha", task: "TASK-1" }),
    );
  });
});
