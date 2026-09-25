import type { InboxItem } from "@taskboard/types";

export function inboxItemsForScope(
  all: InboxItem[],
  scope: "this" | "all",
  slug: string | undefined,
): InboxItem[] {
  if (scope === "this" && slug) {
    return all.filter((item) => item.projectSlug === slug);
  }
  return all;
}
