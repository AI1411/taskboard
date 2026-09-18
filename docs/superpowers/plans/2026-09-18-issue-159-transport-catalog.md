# Catalog Http/Tauri transports (#159) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Http and Tauri transports share one declarative method catalog / factory so adding an endpoint does not require hand-duplicating full method bodies in both files.

**Architecture:** Introduce `packages/client/src/createTransport.ts` that builds a `Transport` from a single `dispatch(spec)` callback. Each method maps once to `{ cmd, args, http: { method, path, body?, revision?, unwrap? } }`. `HttpTransport` and `TauriTransport` only implement envelope/`request` vs `invoke`+`snakeArgs` and pass that into `createTransport`.

**Tech Stack:** TypeScript, Vitest, `@taskboard/client`.

## Global Constraints

- Behavior-preserving — do not rename public client methods
- No new product endpoints
- `fakeTransport` stays aligned (same `Transport` interface; no need to use catalog)
- Client tests pass

User already chose sequential inline execution — do not ask Subagent-Driven vs Inline.

## File map

- Create: `packages/client/src/createTransport.ts`
- Modify: `packages/client/src/http.ts` — thin class + `request`; methods from factory
- Modify: `packages/client/src/tauri.ts` — thin class + `call`; methods from factory
- Unchanged API: `transport.ts` interface, `index.ts` exports
- Test: `http.test.ts`, `tauri.test.ts`

---

### Task 1: `createTransport` + wire backends

**Interfaces:**

```ts
export type TransportDispatch = <T>(spec: {
  cmd: string;
  args?: Record<string, unknown>;
  http: {
    method: string;
    path: string;
    body?: unknown;
    revision?: number;
    unwrap?: "entity" | "entities" | "raw";
  };
}) => Promise<T>;

export function createTransport(dispatch: TransportDispatch): Transport;
```

Method bodies (examples):

```ts
projectAdd: (input) =>
  dispatch({
    cmd: "project_add",
    args: input,
    http: { method: "POST", path: "/api/v1/projects", body: input },
  }),
projectList: (includeArchived) =>
  dispatch({
    cmd: "project_list",
    args: { includeArchived },
    http: {
      method: "GET",
      path: `/api/v1/projects?archived=${includeArchived}`,
      unwrap: "entities",
    },
  }),
// …all 37 methods; preserve special cases:
// projectUpdate newSlug remap, commentAdd continueWaiting→continue body key for HTTP,
// runPatch spreads op, inbox query params, etc.
```

`HttpTransport`: keep `request` private; `constructor` assigns `Object.assign(this, createTransport((spec) => this.request(...)))` or stores impl and implements via composition returning the factory result.

`TauriTransport`: `dispatch` → `this.call(spec.cmd, spec.args)`.

- [ ] Implement + `pnpm --filter @taskboard/client test` PASS
- [ ] Commit + PR

## Self-review

One catalog/factory — yes. No hand-duplicated method bodies in http+tauri — yes. Public names unchanged — yes.
