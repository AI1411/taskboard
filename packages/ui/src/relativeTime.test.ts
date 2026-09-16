import { describe, expect, it } from "vitest";

import { relativeTime } from "./relativeTime";

describe("relativeTime", () => {
  const now = Date.parse("2026-09-16T12:02:00Z");

  it("formats minutes, hours, and days", () => {
    expect(relativeTime("2026-09-16T12:00:00Z", now)).toBe("2m ago");
    expect(relativeTime("2026-09-16T10:02:00Z", now)).toBe("2h ago");
    expect(relativeTime("2026-09-14T12:02:00Z", now)).toBe("2d ago");
  });
});
