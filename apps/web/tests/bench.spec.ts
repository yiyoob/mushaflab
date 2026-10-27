// T8.4 (D62): the 16 ms/page budget in Chromium with the CPU throttled 4x, as a proxy for a
// mid-range phone (the real device is the owner's check at G5, G-214). For each muṣḥaf the
// reader pages forward through a sweep; the worker paints neighbours ahead (D62, D63). Reported:
// - engine: the worker's work for a page (core fit, draw, SVG, and canvas painting);
// - turn: from the page request to the painted page on screen. The budget applies here.
import { expect, test } from "@playwright/test";
import { open } from "./helpers";

const THROTTLE = 4;
const SWEEP = 20;
const BUDGET = 16;

const q = (v: number[], p: number) => [...v].sort((a, b) => a - b)[Math.min(v.length - 1, Math.floor((v.length - 1) * p))];

for (const [mushaf, highlighted] of [["hafs/hijazi", false], ["warsh/maghribi", false], ["warsh/maghribi", true]] as const) {
  test(`budget ${mushaf}${highlighted ? " highlighted" : ""}`, async ({ page }) => {
    await open(page, mushaf, 100);
    if (highlighted) {
      await page.locator("#hl").check();
      await expect(page.locator("#pageview")).toHaveAttribute("data-overlay", "100");
    }
    const cdp = await page.context().newCDPSession(page);
    await cdp.send("Emulation.setCPUThrottlingRate", { rate: THROTTLE });
    const r = await page.evaluate(async (n) => {
      const m = window.mushaf;
      const turn: number[] = [], engine: number[] = [];
      for (let p = 101; p < 101 + n; p++) {
        // a reader looks at a page for a while before turning
        await new Promise((ok) => setTimeout(ok, 1500));
        const [e0, t0] = [m.state.renderMs.length, m.state.turnMs.length];
        await m.go(p);
        engine.push(...m.state.renderMs.slice(e0));
        turn.push(...m.state.turnMs.slice(t0));
      }
      return { turn, engine };
    }, SWEEP);
    await cdp.send("Emulation.setCPUThrottlingRate", { rate: 1 });
    const line = (k: string, v: number[]) => `${k} median ${q(v, 0.5).toFixed(1)} ms, p95 ${q(v, 0.95).toFixed(1)} ms, max ${Math.max(...v).toFixed(1)} ms`;
    console.log(`${mushaf}${highlighted ? " highlighted" : ""} (CPU /${THROTTLE}, ${SWEEP} pages): ${line("turn", r.turn)}; ${line("engine", r.engine)}`);
    expect(q(r.turn, 0.95)).toBeLessThanOrEqual(BUDGET);
  });
}
