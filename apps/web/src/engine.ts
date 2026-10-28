// A promise interface to the engine worker. Calls are queued in order by the worker.
export type Timed<T> = { result: T; ms: number };

const worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
let next = 1;
const pending = new Map<number, { ok: (v: Timed<unknown>) => void; fail: (e: Error) => void }>();
worker.onmessage = (e: MessageEvent<{ id: number; result?: unknown; error?: string; ms: number }>) => {
  const p = pending.get(e.data.id);
  if (!p) return;
  pending.delete(e.data.id);
  if (e.data.error !== undefined) p.fail(new Error(e.data.error));
  else p.ok({ result: e.data.result, ms: e.data.ms });
};

export function callTimed<T>(method: string, ...args: unknown[]): Promise<Timed<T>> {
  const id = next++;
  return new Promise((ok, fail) => {
    pending.set(id, { ok: ok as (v: Timed<unknown>) => void, fail });
    worker.postMessage({ id, method, args });
  });
}

export async function call<T>(method: string, ...args: unknown[]): Promise<T> {
  return (await callTimed<T>(method, ...args)).result;
}

/** A JSON-returning engine method, parsed. */
export async function callJson<T>(method: string, ...args: unknown[]): Promise<T> {
  return JSON.parse(await call<string>(method, ...args)) as T;
}
