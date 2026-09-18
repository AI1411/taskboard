import { useRef, type MutableRefObject } from "react";

/** Always-current ref for read-only snapshots in effects/handlers. */
export function useLatestRef<T>(value: T): MutableRefObject<T> {
  const ref = useRef(value);
  ref.current = value;
  return ref;
}
