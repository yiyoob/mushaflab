// T8.1 acceptance (`just web-parity`): the WASM engine, fed the static bundle, draws every
// golden page byte-identically to the CLI. A plain page's draw list must hash to the golden's
// `draw_sha256` (sha256 of the CLI's serde_json output); a highlights golden must equal the
// page's highlights in the CLI's pretty JSON. `hafs-p003-lint` is skipped: its overlay is built
// inside the Rust test, not stored (its edit counterpart `hafs-p003-edit` is checked).
import { readFileSync, readdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const repo = join(dirname(fileURLToPath(import.meta.url)), "../../..");
const web = join(repo, "build/web");
const { initSync, Engine } = await import(join(web, "pkg/mushaf_wasm.js"));
initSync({ module: readFileSync(join(web, "pkg/mushaf_wasm_bg.wasm")) });

let t = performance.now();
const engine = new Engine(readFileSync(join(web, "data/mushaf.bundle")));
console.log(`bundle parsed in ${Math.round(performance.now() - t)} ms`);
const styles = JSON.parse(engine.styles());

const dir = join(repo, "tests/golden/pages");
const re = /^([a-z_]+?)(?:-(hijazi|maliyah))?-p(\d{3})(?:-(highlights|edit|lint))?\.json$/;
const cases = readdirSync(dir)
  .map((f) => [f, f.match(re)])
  .filter(([f, m]) => {
    if (!m) console.log(`skip ${f} (not a page golden)`);
    else if (m[4] === "lint") console.log(`skip ${f} (overlay built in the Rust test)`);
    return m && m[4] !== "lint";
  })
  .sort(([a], [b]) => a.localeCompare(b));

const sha = (s) => createHash("sha256").update(s, "utf8").digest("hex");
let current = "", fails = 0, n = 0;
for (const [file, m] of cases) {
  const [, riwayah, variant, pageStr, kind] = m;
  const page = Number(pageStr);
  const style = variant === "hijazi" ? "hijazi" : styles.find((s) => s.riwayah === riwayah && s.default).style;
  if (current !== `${riwayah}/${style}`) {
    t = performance.now();
    engine.select(riwayah, style);
    console.log(`select ${riwayah} ${style}: ${Math.round(performance.now() - t)} ms`);
    current = `${riwayah}/${style}`;
  }
  const want = readFileSync(join(dir, file), "utf8");
  let ok;
  if (kind === "highlights") {
    const got = JSON.parse(engine.render(page, true)).highlights ?? [];
    ok = JSON.stringify(got, null, 2) + "\n" === want;
  } else {
    if (kind === "edit") engine.import_edits(readFileSync(join(dir, file.replace(".json", ".overlay.yaml")), "utf8"));
    const got = engine.render(page, false);
    if (kind === "edit") engine.edit(JSON.stringify({ op: "clear" }));
    ok = sha(got) === JSON.parse(want).draw_sha256;
  }
  n++;
  if (!ok) fails++;
  console.log(`${ok ? "ok  " : "FAIL"} ${file}`);
}
console.log(`${n - fails} of ${n} golden pages byte-identical`);
process.exit(fails ? 1 : 0);
