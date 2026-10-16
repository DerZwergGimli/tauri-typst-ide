// src/api.ts
import { commands } from "./generated/bindings.ts";
import { useLogsStore } from "@/stores/logs.ts";

type Commands = typeof commands;

// { status: "ok", data: T } | { status: "error", error: E }  →  T
// Commands that don't return a Result pass through unchanged.
type Unwrap<R> = R extends { status: "ok"; data: infer D }
  ? D
  : R extends { status: "error"; error: unknown }
    ? never
    : R;

type WrappedCommands = {
  [K in keyof Commands]: (
    ...args: Parameters<Commands[K]>
  ) => Promise<Unwrap<Awaited<ReturnType<Commands[K]>>> | undefined>;
};

function isResult(
  value: unknown,
): value is
  { status: "ok"; data: unknown } | { status: "error"; error: unknown } {
  return (
    typeof value === "object" &&
    value !== null &&
    "status" in value &&
    (value.status === "ok" || value.status === "error")
  );
}

function wrap<K extends keyof Commands>(name: K): WrappedCommands[K] {
  const fn = commands[name] as (...args: unknown[]) => Promise<unknown>;

  return (async (...args: unknown[]) => {
    const store = useLogsStore(); // resolved at call time, after Pinia is installed
    try {
      const res = await fn(...args);
      if (isResult(res)) {
        if (res.status === "ok") return res.data;
        store.push({
          level: "error",
          timestamp: new Date().toISOString(),
          message: JSON.stringify(res.error),
          context: "backend-" + name.toString(),
        });
        return undefined;
      }
      return res;
    } catch (e) {
      store.push({
        level: "error",
        timestamp: new Date().toISOString(),
        message: JSON.stringify(e),
        context: name.toString(),
      });
      return undefined;
    }
  }) as WrappedCommands[K];
}

export const ExecuteCommand = Object.fromEntries(
  (Object.keys(commands) as (keyof Commands)[]).map((name) => [
    name,
    wrap(name),
  ]),
) as WrappedCommands;
