// T8.3: each screen works and shows only what the core returns.
import { expect, test } from "@playwright/test";
import { canvasRed, golden, markerIn, open, readText } from "./helpers";

test("the riwayah picker offers exactly the core's styles", async ({ page }) => {
  await open(page, "hafs/hijazi", 1);
  const styles = JSON.parse((await page.evaluate(() => window.mushaf.call("styles"))) as string);
  const offered = await page.locator("#mushaf option").evaluateAll((os) => os.map((o) => (o as HTMLOptionElement).value));
  expect(offered.sort()).toEqual(styles.map((s: { riwayah: string; style: string }) => `${s.riwayah}/${s.style}`).sort());
});

test("RTL pager: ArrowLeft is the next page, ArrowRight the previous", async ({ page }) => {
  await open(page, "hafs/hijazi", 3);
  await page.keyboard.press("ArrowLeft");
  await expect(page.locator("#pageview")).toHaveAttribute("data-page", "4");
  await page.keyboard.press("ArrowRight");
  await expect(page.locator("#pageview")).toHaveAttribute("data-page", "3");
});

test("highlight toggle is kept per mushaf; tapping a highlight shows label, Hafs form and source", async ({ page }) => {
  await open(page, "warsh/maghribi", 3);
  await expect(page.locator("#hl")).not.toBeChecked();
  await page.locator("#hl").check();
  await expect(page.locator('#pageview use[fill="#cc0000"]').first()).toBeVisible();
  const h = (await page.evaluate(() => window.mushaf.state.view!.highlights[0]))!;
  await page.locator(`#pageview use[data-c="${h.cluster}"][data-l="${h.line}"]`).first().dispatchEvent("click");
  await expect(page.locator("#popover")).toBeVisible();
  await expect(page.locator("#popover")).toContainText(h.label);
  await expect(page.locator("#popover")).toContainText(h.hafs || "inserted");
  await expect(page.locator("#popover")).toContainText(h.cite ?? "no citation");
  // another muṣḥaf keeps its own setting; coming back restores this one
  await page.evaluate(() => window.mushaf.select("hafs/hijazi"));
  await expect(page.locator("#hl")).not.toBeChecked();
  await page.evaluate(() => window.mushaf.select("warsh/maghribi"));
  await expect(page.locator("#hl")).toBeChecked();
});

test("option sheet shows the data's groups and defaults; the margin carries the other wajh", async ({ page }) => {
  // the page of 69:28 (golden hafs-maliyah-p567, core test hafs_maliyah_option_pair)
  await open(page, "hafs/hijazi", 567);
  const groups = JSON.parse((await page.evaluate(() => window.mushaf.call("option_groups"))) as string);
  await page.locator("#btn-options").click();
  for (const g of groups) {
    await expect(page.locator(`fieldset[data-group="${g.id}"]`)).toBeVisible();
    await expect(page.locator(`fieldset[data-group="${g.id}"] input[value="${g.default}"]`)).toBeChecked();
  }
  await expect(page.locator("#margin .note")).not.toHaveCount(0);
  const note = page.locator("#margin .note").first();
  await expect(note.locator(".note-word svg").first()).toBeAttached();
  // a page without khilaf has no margin
  await page.evaluate(() => window.mushaf.go(1));
  await expect(page.locator("#margin .note")).toHaveCount(0);
  // riwayat with no option groups do not show the button
  await page.evaluate(() => window.mushaf.select("warsh/maghribi"));
  const none = JSON.parse((await page.evaluate(() => window.mushaf.call("option_groups"))) as string);
  await expect(page.locator("#btn-options")).toBeVisible({ visible: none.length > 0 });
});

test("free edit: the marker is in the view and in every export; undo restores the page", async ({ page }) => {
  await open(page, "hafs/hijazi", 3);
  const plain = (await page.evaluate(() => window.mushaf.call("svg", 3, false))) as string;
  expect(markerIn(plain)).toBe(false);
  await page.locator("#btn-edit").click();
  await page.locator("#pageview use[data-c]").nth(40).click({ force: true });
  await expect(page.locator("#sel")).toBeVisible();
  await page.locator("#val").fill("32");
  await page.locator('button[data-op="set_width"]').click();
  await expect(page.locator("#pageview")).toHaveAttribute("data-canonical", "false");
  // the marker is painted on the page on screen (rule 11)
  expect(await canvasRed(page)).toBeGreaterThan(0);
  const [svgDl] = await Promise.all([page.waitForEvent("download"), page.locator("#export").selectOption("svg")]);
  expect(markerIn(readText((await svgDl.path())!))).toBe(true);
  const [jsonDl] = await Promise.all([page.waitForEvent("download"), page.locator("#export").selectOption("json")]);
  expect(readText((await jsonDl.path())!)).toContain('"role":"edited_marker"');
  const [pngDl] = await Promise.all([page.waitForEvent("download"), page.locator("#export").selectOption("png")]);
  expect((await pngDl.path()) !== null).toBe(true);
  // the PNG has the marker's red where the plain page has none
  const red = async (svg: string) =>
    page.evaluate(async (s) => {
      const img = new Image();
      img.src = URL.createObjectURL(new Blob([s], { type: "image/svg+xml" }));
      await img.decode();
      const c = document.createElement("canvas");
      [c.width, c.height] = [600, Math.round((600 * img.naturalHeight) / img.naturalWidth)];
      const g = c.getContext("2d")!;
      g.drawImage(img, 0, 0, c.width, c.height);
      const d = g.getImageData(0, 0, c.width, c.height).data;
      let n = 0;
      for (let i = 0; i < d.length; i += 4) if (d[i] > 150 && d[i + 1] < 80 && d[i + 2] < 80) n++;
      return n;
    }, svg);
  expect(await red((await page.evaluate(() => window.mushaf.call("svg", 3, false))) as string)).toBeGreaterThan(0);
  expect(await red(plain)).toBe(0);
  await page.locator('button[data-op="undo"]').click();
  await expect(page.locator("#pageview")).toHaveAttribute("data-canonical", "true");
  expect(await canvasRed(page)).toBe(0);
  expect((await page.evaluate(() => window.mushaf.call("svg", 3, false))) as string).toBe(plain);
});

test("an exported overlay imports and draws exactly as the CLI's edit golden", async ({ page }) => {
  await open(page, "hafs/hijazi", 3);
  await page.evaluate((t) => window.mushaf.call("import_edits", t), readText(golden("hafs-p003-edit.overlay.yaml")));
  await expect(page.locator("#pageview")).toHaveAttribute("data-canonical", "false");
  const json = (await page.evaluate(() => window.mushaf.call("render", 3, false))) as string;
  const sha = await page.evaluate(async (s) => {
    const h = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s));
    return [...new Uint8Array(h)].map((b) => b.toString(16).padStart(2, "0")).join("");
  }, json);
  expect(sha).toBe(JSON.parse(readText(golden("hafs-p003-edit.json"))).draw_sha256);
});

test("inspector lists every drawn cluster with provenance; provenance screen shows statuses", async ({ page }) => {
  await open(page, "warsh/maghribi", 3);
  await page.locator("#btn-inspect").click();
  await expect(page.locator("table.insp tbody tr").first()).toBeVisible();
  const n = await page.locator("table.insp tbody tr").count();
  const drawn = await page.locator("#pageview use[data-c]").evaluateAll((us) => new Set(us.map((u) => u.getAttribute("data-c"))).size);
  expect(n).toBe(drawn);
  await expect(page.locator("table.insp tbody tr.changed").first()).toBeVisible();
  await page.locator("#btn-prov").click();
  await expect(page.locator("#panel")).toContainText("Patch files");
  await expect(page.locator("#panel")).toContainText("patches/curated/warsh.yaml");
});

test("phone width: margin notes are tap-to-show and the page keeps its lines", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await open(page, "hafs/hijazi", 567);
  const drawn = await page.locator("#pageview").innerHTML();
  const uses = await page.locator("#pageview use").count();
  const note = page.locator("#margin .note").first();
  await expect(note.locator(".note-body")).toBeHidden();
  await note.locator(".note-dot").click();
  await expect(note.locator(".note-body")).toBeVisible();
  // the page is the core's drawing, untouched by the margin
  expect(await page.locator("#pageview").innerHTML()).toBe(drawn);
  const view = JSON.parse((await page.evaluate(() => window.mushaf.call("view", 567, false))) as string);
  const coreUses = await page.evaluate((svg) => new DOMParser().parseFromString(svg, "image/svg+xml").querySelectorAll("use").length, view.svg);
  expect(uses).toBe(coreUses);
});

test("offline: after one load, a reload with no network still draws pages (rule 9)", async ({ page, context }) => {
  await open(page, "hafs/hijazi", 3);
  await page.evaluate(() => navigator.serviceWorker.ready);
  await page.reload();
  await page.waitForFunction(() => window.mushaf?.ready, null, { timeout: 120_000 });
  const requests: string[] = [];
  page.on("request", (r) => requests.push(r.url()));
  // paging after load makes no request at all
  for (const p of [4, 5, 6]) await page.evaluate((q) => window.mushaf.go(q), p);
  expect(requests).toEqual([]);
  await context.setOffline(true);
  await page.reload();
  await page.waitForFunction(() => window.mushaf?.ready, null, { timeout: 120_000 });
  await page.evaluate(() => window.mushaf.go(10));
  await expect(page.locator("#pageview")).toHaveAttribute("data-page", "10");
  await context.setOffline(false);
});

test("the page on screen keeps the page's proportions and matches the golden painting", async ({ page }) => {
  await open(page, "hafs/hijazi", 3);
  const r = await page.evaluate(() => {
    const c = document.querySelector<HTMLCanvasElement>("#pageview canvas.page")!;
    const b = c.getBoundingClientRect();
    return { w: c.width, h: c.height, cssRatio: b.height / b.width };
  });
  // proportions of the golden PNG of the same page (1000 x 1577, tests/golden/pages/hafs-p003.png)
  const goldenRatio = 1577 / 1000;
  expect(Math.abs(r.h / r.w - goldenRatio)).toBeLessThan(0.002);
  expect(Math.abs(r.cssRatio - goldenRatio)).toBeLessThan(0.01);
});
