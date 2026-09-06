import { HttpTransport } from "@taskboard/client";

export type Bootstrap = {
  session: string;
  activitySequence?: number;
};

export async function fetchBootstrap(
  fetchImpl: typeof fetch = fetch,
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
  fetchImpl: typeof fetch = fetch,
): HttpTransport {
  return new HttpTransport(baseUrl, json.session, fetchImpl);
}

export function refetchInterval(
  visibility: DocumentVisibilityState,
): number | false {
  return visibility === "visible" ? 1000 : false;
}
