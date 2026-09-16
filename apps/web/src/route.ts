import type { BoardRoute } from "@taskboard/ui";

export function readBoardRoute(search = window.location.search): BoardRoute {
  const query = new URLSearchParams(search);
  return {
    project: query.get("project"),
    task: query.get("task"),
  };
}

export function replaceBoardRoute(
  route: { project: string | null; task: string | null },
  loc: Pick<Location, "href" | "pathname" | "search" | "hash"> = window.location,
  historyApi: Pick<History, "replaceState"> = window.history,
): string {
  const url = new URL(loc.href);
  if (route.project) url.searchParams.set("project", route.project);
  else url.searchParams.delete("project");
  if (route.task) url.searchParams.set("task", route.task);
  else url.searchParams.delete("task");
  const next = `${url.pathname}${url.search}${url.hash}`;
  const current = `${loc.pathname}${loc.search}${loc.hash}`;
  if (next !== current) historyApi.replaceState(null, "", next);
  return next;
}
