import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { Composer } from "./Composer";

describe("Composer", () => {
  it("submits trimmed title on Enter and cancels empty Enter", async () => {
    const onSubmit = vi.fn();
    const onCancel = vi.fn();
    render(<Composer placeholder="Task title" onSubmit={onSubmit} onCancel={onCancel} />);
    const input = screen.getByPlaceholderText("Task title");
    await userEvent.type(input, "  Fix login  {Enter}");
    expect(onSubmit).toHaveBeenCalledWith("Fix login");
    onSubmit.mockClear();
    await userEvent.clear(input);
    await userEvent.type(input, "{Enter}");
    expect(onSubmit).not.toHaveBeenCalled();
    expect(onCancel).toHaveBeenCalled();
  });
});
