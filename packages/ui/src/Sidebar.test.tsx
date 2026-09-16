import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { fakeTransport } from "./fakeTransport";
import { TaskboardApp } from "./index";

describe("project admin", () => {
  it("renames the selected project on blur", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Alpha" });
    render(<TaskboardApp transport={transport} />);
    const name = await screen.findByLabelText("Project name");
    await waitFor(() => expect((name as HTMLInputElement).value).toBe("Alpha"));
    fireEvent.change(name, { target: { value: "Renamed" } });
    fireEvent.blur(name);
    await waitFor(() =>
      expect(transport.projectUpdate).toHaveBeenCalledWith("alpha", { name: "Renamed" }, 1),
    );
  });

  it("sets path, archives, reorders, and deletes from the project menu", async () => {
    const transport = fakeTransport();
    await transport.projectAdd({ name: "Alpha" });
    await transport.projectAdd({ name: "Beta" });
    render(<TaskboardApp transport={transport} />);
    await screen.findByRole("button", { name: "Alpha" });
    await userEvent.click(screen.getByRole("button", { name: "Project actions" }));
    await userEvent.click(screen.getByRole("button", { name: "Set repository path" }));
    await userEvent.type(screen.getByPlaceholderText("Repository path"), "/tmp/alpha{Enter}");
    await waitFor(() =>
      expect(transport.projectUpdate).toHaveBeenCalledWith(
        "alpha",
        { repoPath: "/tmp/alpha" },
        1,
      ),
    );
    await userEvent.click(screen.getByRole("button", { name: "Project actions" }));
    await userEvent.click(screen.getByRole("button", { name: "Move down" }));
    await waitFor(() => expect(transport.projectReorder).toHaveBeenCalledWith(["beta", "alpha"]));
    await userEvent.click(screen.getByRole("button", { name: "Project actions" }));
    await userEvent.click(screen.getByRole("button", { name: "Archive" }));
    await waitFor(() => expect(transport.projectArchive).toHaveBeenCalledWith("alpha", true, 2));
    await userEvent.click(await screen.findByRole("button", { name: "Beta" }));
    await userEvent.click(screen.getByRole("button", { name: "Project actions" }));
    await userEvent.click(screen.getByRole("button", { name: "Delete project" }));
    expect(transport.projectDelete).not.toHaveBeenCalled();
    expect(screen.getByText("Delete Beta?")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(transport.projectDelete).toHaveBeenCalledWith("beta", 1));
  });

  it("collapses the project note and marks when it has text", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    project.noteMarkdown = "ship notes";
    render(<TaskboardApp transport={transport} />);
    const toggle = await screen.findByRole("button", { name: "Project note" });
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(toggle.querySelector("[data-filled]")).toBeTruthy();
    expect(screen.queryByLabelText("Project note")).toBeNull();
    await userEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect((screen.getByLabelText("Project note") as HTMLTextAreaElement).value).toBe("ship notes");
    expect(screen.getByRole("list", { name: "Projects" }).getAttribute("data-scroll")).toBe(
      "projects",
    );
    expect(screen.getByRole("button", { name: "Archived projects" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Trash" })).toBeTruthy();
  });
});
