import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, type Page } from "@playwright/test";

export const repo = join(import.meta.dirname, "../../..");
export const golden = (name: string) => join(repo, "tests/golden/pages", name);

/** Opens the app on a muṣḥaf and page and waits for the core's page to be on screen. */
export async function open(page: Page, mushaf: string, p: number) {
  await page.goto(`/?mushaf=${mushaf}&page=${p}`);
  await page.waitForFunction(() => window.mushaf?.ready, null, { timeout: 120_000 });
  await expect(page.locator("#pageview")).toHaveAttribute("data-page", String(p));
  await expect(page.locator("#pageview")).toHaveAttribute("data-overlay", String(p));
}

export const readText = (path: string) => readFileSync(path, "utf8");

/** The edited marker as the core draws it: red strokes (rule 11). */
export const markerIn = (svg: string) => svg.includes('stroke="#cc0000"');

/** Red pixels (the edited marker's colour) in the page canvas on screen. */
export const canvasRed = (page: Page) =>
  page.evaluate(() => {
    const src = document.querySelector<HTMLCanvasElement>("#pageview canvas.page")!;
    const c = document.createElement("canvas");
    [c.width, c.height] = [src.width, src.height];
    const g = c.getContext("2d")!;
    g.drawImage(src, 0, 0);
    const d = g.getImageData(0, 0, c.width, c.height).data;
    let n = 0;
    for (let i = 0; i < d.length; i += 4) if (d[i] > 150 && d[i + 1] < 80 && d[i + 2] < 80) n++;
    return n;
  });
