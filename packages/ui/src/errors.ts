import { TransportError } from "@taskboard/client";

export function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}

export function errorCode(err: unknown): string | undefined {
  if (err instanceof TransportError) return err.code;
  return undefined;
}

export function errorField(err: unknown): string | undefined {
  if (err instanceof TransportError) return err.field;
  return undefined;
}

export function isNotFound(err: unknown): boolean {
  if (errorCode(err) === "not_found") return true;
  return errorMessage(err).toLowerCase().includes("not found");
}
