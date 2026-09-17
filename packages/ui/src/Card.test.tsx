import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { Card } from "./Card";
import { summary } from "./summary";

describe("Card", () => {
  it("shows running badge and hides idle badge", () => {
    const { getByText, queryByText, rerender } = render(
      <Card task={summary({ displayStatus: "idle" })} selected={false} />,
    );
    expect(queryByText("Running")).toBeNull();
    rerender(<Card task={summary({ displayStatus: "running" })} selected={false} />);
    expect(getByText("Running")).toBeTruthy();
  });

  it("shows waiting, failed, and completed badges", () => {
    const { getByText, queryByText, rerender } = render(
      <Card task={summary({ displayStatus: "waiting" })} selected={false} />,
    );
    expect(getByText("Waiting")).toBeTruthy();
    expect(queryByText("Running")).toBeNull();
    rerender(<Card task={summary({ displayStatus: "failed" })} selected={false} />);
    expect(getByText("Failed")).toBeTruthy();
    rerender(<Card task={summary({ displayStatus: "completed" })} selected={false} />);
    expect(getByText("Done")).toBeTruthy();
    expect(queryByText("Completed")).toBeNull();
  });

  it("renders title, display id, listitem role, and urgent pip", () => {
    const { getByText, getByRole, queryByLabelText, rerender } = render(
      <Card task={summary({ title: "Fix login", displayId: "TASK-7" })} selected={false} />,
    );
    expect(getByText("Fix login")).toBeTruthy();
    expect(getByText("TASK-7")).toBeTruthy();
    expect(getByRole("listitem")).toBeTruthy();
    expect(queryByLabelText("Urgent")).toBeNull();
    rerender(
      <Card task={summary({ title: "Fix login", displayId: "TASK-7", urgent: true })} selected={false} />,
    );
    expect(getByRole("listitem", { name: /Urgent/ })).toBeTruthy();
    expect(getByRole("listitem").querySelector('[aria-label="Urgent"]')).toBeTruthy();
  });

  it("shows a lifted pickup clone and a dim placeholder while grabbed", () => {
    const { getByRole, rerender } = render(
      <Card task={summary({ title: "Fix login" })} selected={false} grabbed />,
    );
    const placeholder = getByRole("listitem");
    expect(placeholder.getAttribute("aria-grabbed")).toBe("true");
    expect(placeholder.getAttribute("data-placeholder")).toBe("true");
    expect(placeholder.getAttribute("data-lifted")).toBeNull();
    rerender(<Card task={summary({ title: "Fix login" })} selected={false} lifted />);
    const overlay = getByRole("listitem", { hidden: true });
    expect(overlay.getAttribute("data-lifted")).toBe("true");
    expect(overlay.getAttribute("aria-hidden")).toBe("true");
    expect(overlay.getAttribute("data-placeholder")).toBeNull();
  });

  it("shows waitingReason when runMessage is empty", () => {
    const { getByText } = render(
      <Card
        task={summary({ displayStatus: "waiting", runMessage: null, waitingReason: "Need spec" })}
        selected={false}
      />,
    );
    expect(getByText("Need spec")).toBeTruthy();
  });

  it("copy button calls onCopyId and does not require a card click", async () => {
    const onCopyId = vi.fn();
    const onClick = vi.fn();
    const { getByRole } = render(
      <Card
        task={summary({ title: "Fix login", displayId: "TASK-7" })}
        selected={false}
        onCopyId={onCopyId}
        onClick={onClick}
      />,
    );
    await userEvent.click(getByRole("button", { name: "Copy TASK-7" }));
    expect(onCopyId).toHaveBeenCalledWith("TASK-7");
    expect(onClick).not.toHaveBeenCalled();
  });

  it("marks attention faces and mutes the completed badge", () => {
    const { getByRole, getByText, rerender } = render(
      <Card task={summary({ displayStatus: "waiting" })} selected={false} />,
    );
    expect(getByRole("listitem").getAttribute("data-face")).toBe("waiting");
    rerender(
      <Card
        task={summary({ displayStatus: "failed", waitingReason: "boom" })}
        selected={false}
      />,
    );
    expect(getByRole("listitem").getAttribute("data-face")).toBe("failed");
    expect(getByText("boom")).toBeTruthy();
    rerender(<Card task={summary({ displayStatus: "running" })} selected={false} />);
    expect(getByRole("listitem").getAttribute("data-face")).toBe("running");
    rerender(<Card task={summary({ displayStatus: "running", stale: true })} selected={false} />);
    expect(getByRole("listitem").getAttribute("data-face")).toBeNull();
    rerender(<Card task={summary({ displayStatus: "completed" })} selected={false} />);
    expect(getByRole("listitem").getAttribute("data-face")).toBe("completed");
    expect(getByText("Done").className).toMatch(/muted/);
  });

  it("shows a Stale badge instead of Running", () => {
    const { getByText, queryByText } = render(
      <Card task={summary({ displayStatus: "running", stale: true })} selected={false} />,
    );
    expect(getByText("Stale")).toBeTruthy();
    expect(queryByText("Running")).toBeNull();
    expect(queryByText("Waiting")).toBeNull();
  });

  it("shows recorded worktree and branch", () => {
    const { getByText, queryByText } = render(
      <Card
        task={summary({ worktreePath: "/tmp/wt", branch: "cursor/foo-88ba" })}
        selected={false}
      />,
    );
    expect(getByText("/tmp/wt")).toBeTruthy();
    expect(getByText("cursor/foo-88ba")).toBeTruthy();
    expect(queryByText(/shared/i)).toBeNull();
  });

  it("shows checklist progress on the card face", () => {
    const { getByText, rerender, queryByText } = render(
      <Card task={summary({ checklistDone: 2, checklistTotal: 5 })} selected={false} />,
    );
    expect(getByText("2/5")).toBeTruthy();
    rerender(<Card task={summary({ checklistDone: 0, checklistTotal: 0 })} selected={false} />);
    expect(queryByText("0/0")).toBeNull();
  });

  it("shows Blocked by and Blocks labels", () => {
    const { getByText, rerender } = render(
      <Card task={summary({ blockedBy: ["TASK-8"] })} selected={false} />,
    );
    expect(getByText("Blocked by TASK-8")).toBeTruthy();
    rerender(<Card task={summary({ blocks: ["TASK-14"] })} selected={false} />);
    expect(getByText("Blocks TASK-14")).toBeTruthy();
  });
});
