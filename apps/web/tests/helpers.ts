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
}

export const readText = (path: string) => readFileSync(path, "utf8");

/** The edited marker as the core draws it: red strokes (rule 11). */
export const markerIn = (svg: string) => svg.includes('stroke="#cc0000"');
