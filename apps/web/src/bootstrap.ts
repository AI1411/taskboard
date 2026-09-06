import { HttpTransport } from "@taskboard/client";

export type Bootstrap = {
  session: string;
  activitySequence?: number;
};

function defaultFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  return globalThis.fetch(input, init);
}

export async function fetchBootstrap(
  fetchImpl: typeof fetch = defaultFetch,
): Promise<Bootstrap> {
  const res = await fetchImpl("/api/v1/bootstrap", { credentials: "include" });
  if (!res.ok) {
    throw new Error(`bootstrap ${res.status}`);
  }
  return (await res.json()) as Bootstrap;
}

export function transportFromBootstrap(
  json: { session: string },
  baseUrl: string,
  fetchImpl: typeof fetch = defaultFetch,
): HttpTransport {
  return new HttpTransport(baseUrl, json.session, fetchImpl);
}

export function refetchInterval(
  visibility: DocumentVisibilityState,
): number | false {
  return visibility === "visible" ? 1000 : false;
}
