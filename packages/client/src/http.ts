import { createTransport } from "./createTransport";
import { TransportError, type Transport } from "./transport";

type Envelope = {
  ok?: boolean;
  entity?: unknown;
  entities?: unknown;
  error?: { message?: string; code?: string };
};

function defaultFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  return globalThis.fetch(input, init);
}

export interface HttpTransport extends Transport {}

export class HttpTransport {
  constructor(
    private readonly baseUrl: string,
    private readonly session: string,
    private readonly fetchImpl: typeof fetch = defaultFetch,
  ) {
    Object.assign(
      this,
      createTransport((spec) =>
        this.request(spec.http.method, spec.http.path, {
          body: spec.http.body,
          revision: spec.http.revision,
          unwrap: spec.http.unwrap,
        }),
      ),
    );
  }

  private url(path: string): string {
    return `${this.baseUrl.replace(/\/$/, "")}${path}`;
  }

  private async request<T>(
    method: string,
    path: string,
    opts: {
      body?: unknown;
      revision?: number;
      unwrap?: "entity" | "entities" | "raw";
    } = {},
  ): Promise<T> {
    const headers = new Headers();
    headers.set("X-Taskboard-Session", this.session);
    if (opts.body !== undefined) {
      headers.set("Content-Type", "application/json");
    }
    if (opts.revision !== undefined) {
      headers.set("If-Match", String(opts.revision));
    }

    const res = await this.fetchImpl(this.url(path), {
      method,
      headers,
      body: opts.body === undefined ? undefined : JSON.stringify(opts.body),
    });

    const json = (await res.json()) as Envelope;
    if (!res.ok || json.error) {
      throw new TransportError({
        code: json.error?.code ?? `http_${res.status}`,
        message: json.error?.message ?? `HTTP ${res.status}`,
      });
    }

    const unwrap = opts.unwrap ?? "entity";
    if (unwrap === "raw") {
      return json as T;
    }
    if (unwrap === "entities") {
      return json.entities as T;
    }
    return json.entity as T;
  }
}
