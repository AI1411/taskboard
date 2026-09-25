import { describe, expect, it } from "vitest";

import { editDraft, followServer, freshDraft, keepMine, shouldCommit } from "./fieldDraft";

describe("fieldDraft", () => {
  it("follows the server while the draft is clean", () => {
    const next = followServer(freshDraft(""), "# written by agent");
    expect(next.value).toBe("# written by agent");
    expect(shouldCommit(next)).toBe(false);
  });

  it("marks a dirty draft as a conflict when the server moves", () => {
    const dirty = editDraft(freshDraft(""), "my draft");
    const next = followServer(dirty, "# written by agent");
    expect(next.conflict).toBe(true);
    expect(next.value).toBe("my draft");
    expect(shouldCommit(next)).toBe(false);
  });

  it("clears dirty when the server echoes the draft", () => {
    const dirty = editDraft(freshDraft(""), "/tmp/wt");
    const next = followServer(dirty, "/tmp/wt");
    expect(next).toEqual(freshDraft("/tmp/wt"));
    expect(shouldCommit(next)).toBe(false);
  });

  it("keepMine retargets the base so the local value can be saved", () => {
    const conflict = followServer(editDraft(freshDraft(""), "my draft"), "# agent");
    const kept = keepMine(conflict, "# agent");
    expect(kept.conflict).toBe(false);
    expect(kept.value).toBe("my draft");
    expect(shouldCommit(kept)).toBe(true);
  });
});
