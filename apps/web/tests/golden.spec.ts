// T8.5: every core golden page, as the app draws it in Chromium, against the core's golden PNG
// (tests/golden/pages, rasterised by resvg at 1000 px). The app's SVG is the core's (D59), so
// the drawing is byte-identical (mushaf-draw tests, `just web-parity`); what this checks is the
// browser's painting of it. Two rasterisers anti-alias edges differently (D61), so both images
// are box-blurred 3x3, and a pixel differs only when no pixel within 1 px of it in the other
// image is within 32 on every channel (both ways). At most 0.1 % may differ, as in golden.rs.
// A control (the plain page against the edit golden, which moves one line) must fail.
// `hafs-p003-lint` is skipped: its overlay is built inside the Rust test, not stored.
import { readdirSync, readFileSync } from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { golden, open, readText } from "./helpers";

const re = /^([a-z_]+?)(?:-(hijazi|maliyah))?-p(\d{3})(?:-(edit))?\.png$/;
const cases = readdirSync(golden(""))
  .map((f) => f.match(re))
  .filter((m): m is RegExpMatchArray => m !== null)
  .sort((a, b) => a[0].localeCompare(b[0]));

const maghribi = new Set(["warsh", "qalun"]); // default styles (D36), as `styles()` reports them

async function compare(page: Page, file: string) {
  const svg = await page.locator("#pageview").innerHTML();
  const png = readFileSync(golden(file)).toString("base64");
  return page.evaluate(
    async ({ svg, png }) => {
      const TOL = 32;
      const load = async (src: string) => {
        const i = new Image();
        i.src = src;
        await i.decode();
        return i;
      };
      const want = await load(`data:image/png;base64,${png}`);
      const [w, h] = [want.naturalWidth, want.naturalHeight];
      // Paint the vector at the golden's pixel size (as resvg does), not a resample of it.
      const root = new DOMParser().parseFromString(svg, "image/svg+xml").documentElement;
      const [vw, vh] = root.getAttribute("viewBox")!.split(" ").slice(2).map(Number);
      if (Math.round((vh * w) / vw) !== h) return { size: false, bad: 0, n: 0 };
      root.setAttribute("width", String(w));
      root.setAttribute("height", String(h));
      const sized = new XMLSerializer().serializeToString(root);
      const got = await load(URL.createObjectURL(new Blob([sized], { type: "image/svg+xml" })));
      const blurred = (img: HTMLImageElement) => {
        const c = document.createElement("canvas");
        [c.width, c.height] = [w, h];
        const g = c.getContext("2d")!;
        g.fillStyle = "#fff";
        g.fillRect(0, 0, w, h);
        g.drawImage(img, 0, 0, w, h);
        const d = g.getImageData(0, 0, w, h).data;
        const out = new Float32Array(w * h * 3);
        for (let y = 0; y < h; y++)
          for (let x = 0; x < w; x++)
            for (let k = 0; k < 3; k++) {
              let s = 0, n = 0;
              for (let dy = -1; dy <= 1; dy++)
                for (let dx = -1; dx <= 1; dx++) {
                  const [qx, qy] = [x + dx, y + dy];
                  if (qx < 0 || qy < 0 || qx >= w || qy >= h) continue;
                  s += d[(qy * w + qx) * 4 + k];
                  n++;
                }
              out[(y * w + x) * 3 + k] = s / n;
            }
        return out;
      };
      const [a, b] = [blurred(want), blurred(got)];
      const near = (x: Float32Array, y: Float32Array, px: number, py: number) => {
        const i = (py * w + px) * 3;
        for (let dy = -1; dy <= 1; dy++)
          for (let dx = -1; dx <= 1; dx++) {
            const [qx, qy] = [px + dx, py + dy];
            if (qx < 0 || qy < 0 || qx >= w || qy >= h) continue;
            const j = (qy * w + qx) * 3;
            if (Math.abs(x[i] - y[j]) <= TOL && Math.abs(x[i + 1] - y[j + 1]) <= TOL && Math.abs(x[i + 2] - y[j + 2]) <= TOL) return true;
          }
        return false;
      };
      let bad = 0;
      for (let py = 0; py < h; py++) for (let px = 0; px < w; px++) if (!near(a, b, px, py) || !near(b, a, px, py)) bad++;
      return { size: true, bad, n: w * h };
    },
    { svg, png },
  );
}

test.describe.configure({ mode: "serial" });

for (const m of cases) {
  const [file, riwayah, variant, pageStr, kind] = m;
  test(`golden ${file}`, async ({ page }) => {
    const style = variant === "hijazi" ? "hijazi" : maghribi.has(riwayah) ? "maghribi" : "hijazi";
    await open(page, `${riwayah}/${style}`, Number(pageStr));
    if (kind === "edit")
      await page.evaluate((t) => window.mushaf.call("import_edits", t), readText(golden(file.replace(".png", ".overlay.yaml"))));
    const r = await compare(page, file);
    expect(r.size, "image size").toBe(true);
    console.log(`${file}: ${r.bad} of ${r.n} pixels differ (${((1000 * r.bad) / r.n).toFixed(3)} per mille)`);
    expect(r.bad * 1000).toBeLessThanOrEqual(r.n);
  });
}

test("control: the check fails on a page that differs by one edited line", async ({ page }) => {
  await open(page, "hafs/hijazi", 3);
  const r = await compare(page, "hafs-p003-edit.png");
  console.log(`control: ${r.bad} of ${r.n} pixels differ`);
  expect(r.bad * 1000).toBeGreaterThan(r.n);
});
