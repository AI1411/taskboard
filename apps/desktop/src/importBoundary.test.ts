import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    if (!path.endsWith(".ts") && !path.endsWith(".tsx")) return [];
    if (path.endsWith(".test.ts") || path.endsWith(".test.tsx")) return [];
    return [path];
  });
}

describe("desktop import boundary", () => {
  it("does not import HttpTransport", () => {
    const src = dirname(fileURLToPath(import.meta.url));
    const offenders = sourceFiles(src).filter((path) =>
      readFileSync(path, "utf8").includes("HttpTransport"),
    );
    expect(offenders).toEqual([]);
  });
});
