// Dev and preview server, and the static build. The WASM package and the data bundle are build
// outputs in build/web (D57): the servers serve them at /pkg/ and /data/, and `vite build`
// copies them into dist/, so dist/ is a complete static site (G-218). Nothing deploys it.
import { cpSync, createReadStream, existsSync, statSync } from "node:fs";
import { join, normalize, resolve } from "node:path";
import { defineConfig, type Connect, type Plugin } from "vite";

const buildWeb = resolve(__dirname, "../../build/web");
const types: Record<string, string> = {
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".json": "application/json",
  ".bundle": "application/octet-stream",
  ".ts": "text/plain",
};

const serveBuild: Connect.NextHandleFunction = (req, res, next) => {
  const url = (req.url ?? "").split("?")[0];
  if (!url.startsWith("/pkg/") && !url.startsWith("/data/")) return next();
  const file = normalize(join(buildWeb, url));
  if (!file.startsWith(buildWeb) || !existsSync(file) || !statSync(file).isFile()) {
    res.statusCode = 404;
    return res.end(`not built: ${url} (run just web-wasm web-data)`);
  }
  const ext = file.slice(file.lastIndexOf("."));
  res.setHeader("Content-Type", types[ext] ?? "application/octet-stream");
  res.setHeader("Content-Length", String(statSync(file).size));
  res.setHeader("Cache-Control", "no-cache");
  createReadStream(file).pipe(res);
};

const buildOutputs = (): Plugin => ({
  name: "mushaf-build-outputs",
  configureServer: (s) => void s.middlewares.use(serveBuild),
  configurePreviewServer: (s) => void s.middlewares.use(serveBuild),
  closeBundle() {
    for (const d of ["pkg", "data"]) {
      const from = join(buildWeb, d);
      if (!existsSync(from)) throw new Error(`${from} missing: run just web-wasm web-data`);
      cpSync(from, resolve(__dirname, "dist", d), { recursive: true });
    }
  },
});

export default defineConfig({
  plugins: [buildOutputs()],
  server: { host: "127.0.0.1", port: 5173, strictPort: true },
  preview: { host: "127.0.0.1", port: 4173, strictPort: true },
  worker: { format: "es" },
});
