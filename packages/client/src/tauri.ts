import { createTransport } from "./createTransport";
import { TransportError, type Transport } from "./transport";

export { TransportError };

export type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

type AppErrorDto = {
  code: string;
  message: string;
  field?: string;
  current?: unknown;
};

function camelToSnake(key: string): string {
  return key.replace(/[A-Z]/g, (ch) => `_${ch.toLowerCase()}`);
}

function snakeArgs(obj: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(obj)) {
    if (value === undefined) continue;
    out[camelToSnake(key)] = value;
  }
  return out;
}

function isAppErrorDto(value: unknown): value is AppErrorDto {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as { code?: unknown }).code === "string"
  );
}

export interface TauriTransport extends Transport {}

export class TauriTransport {
  constructor(private readonly invokeFn: InvokeFn) {
    Object.assign(
      this,
      createTransport((spec) => this.call(spec.cmd, spec.args)),
    );
  }

  private async call<T>(cmd: string, raw?: Record<string, unknown>): Promise<T> {
    const args = raw === undefined ? undefined : snakeArgs(raw);
    const invokeArgs = args && Object.keys(args).length > 0 ? args : undefined;
    try {
      return await this.invokeFn<T>(cmd, invokeArgs);
    } catch (err) {
      if (isAppErrorDto(err)) {
        throw new TransportError(err);
      }
      throw err;
    }
  }
}
