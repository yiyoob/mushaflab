// The core runs here, off the UI thread (D58). This file only forwards calls to the WASM
// engine and times them; every answer is the core's (rule 7).
/// <reference lib="webworker" />

type Engine = Record<string, (...args: unknown[]) => unknown> & { free(): void };
let engine: Engine | null = null;

async function init(): Promise<{ bytes: number; sha256: string; parseMs: number }> {
  const manifest = await (await fetch("/data/manifest.json")).json();
  const pkg = await import(/* @vite-ignore */ `${location.origin}/pkg/mushaf_wasm.js`);
  await pkg.default({ module_or_path: "/pkg/mushaf_wasm_bg.wasm" });
  const res = await fetch(`/data/${manifest.file}?v=${manifest.sha256}`);
  if (!res.ok) throw new Error(`bundle: HTTP ${res.status}`);
  const bytes = new Uint8Array(await res.arrayBuffer());
  const t = performance.now();
  engine = new pkg.Engine(bytes) as Engine;
  return { bytes: bytes.length, sha256: manifest.sha256, parseMs: performance.now() - t };
}

self.onmessage = async (e: MessageEvent<{ id: number; method: string; args: unknown[] }>) => {
  const { id, method, args } = e.data;
  const t = performance.now();
  try {
    let result: unknown;
    if (method === "init") result = await init();
    else {
      if (!engine) throw new Error("engine not loaded");
      const f = engine[method];
      if (typeof f !== "function") throw new Error(`no engine method ${method}`);
      result = f.apply(engine, args);
    }
    self.postMessage({ id, result, ms: performance.now() - t });
  } catch (err) {
    self.postMessage({ id, error: String((err as Error)?.message ?? err), ms: performance.now() - t });
  }
};
