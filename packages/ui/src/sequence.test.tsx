import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, type Mock } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

describe("TaskboardApp sequence refresh", () => {
  it("keeps selection when sequence advances and the task still exists", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Keep me", column: "todo" });
    const { rerender } = render(<TaskboardApp transport={transport} sequence={1} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Keep me/ }));
    expect(await screen.findByDisplayValue("Keep me")).toBeTruthy();

    (transport.projectList as Mock).mockClear();
    (transport.taskList as Mock).mockClear();

    rerender(<TaskboardApp transport={transport} sequence={2} />);
    await waitFor(() => expect(transport.projectList).toHaveBeenCalled());
    expect(await screen.findByDisplayValue("Keep me")).toBeTruthy();
    expect(screen.getByRole("listitem", { name: /Keep me/ }).getAttribute("aria-selected")).toBe(
      "true",
    );
  });

  it("closes inspector and keeps the project when the selected task is gone", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Gone soon", column: "todo" });
    const { rerender } = render(<TaskboardApp transport={transport} sequence={1} />);
    await userEvent.click(await screen.findByRole("listitem", { name: /Gone soon/ }));
    expect(await screen.findByDisplayValue("Gone soon")).toBeTruthy();

    await transport.taskDelete("TASK-1");
    rerender(<TaskboardApp transport={transport} sequence={2} />);

    await waitFor(() => {
      expect(screen.queryByDisplayValue("Gone soon")).toBeNull();
    });
    expect(screen.queryByLabelText("Title")).toBeNull();
    expect(screen.getByRole("button", { name: "Alpha" }).getAttribute("aria-current")).toBe("true");
  });
});
