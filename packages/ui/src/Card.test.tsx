import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

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
});
