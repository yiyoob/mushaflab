// The web app's UI (T8.3). It draws only what the core returns (rule 7): the page is the core's
// draw list painted in the worker (D63), with the core's SVG on top for taps (D59); highlights, margin notes, options, inspector and provenance are core JSON. Nothing
// here resolves, shapes, justifies or decides what differs from Ḥafṣ.
import { call, callJson, callTimed } from "./engine";

type StyleOption = { riwayah: string; style: string; default: boolean };
type Highlight = { line: number; cluster: string; label: string; hafs: string; cite: string | null; cited: boolean; annotation: boolean; owner?: string };
type View = { page: number; riwayah: string; canonical: boolean; height: number; baselines: [number, number][]; svg: string; highlights: Highlight[] };
type OptionGroup = { id: string; title_ar: string; choices: string[]; default: string | null; current: string | null };
type MarginNote = { group: string; title_ar: string; choice: string; line: number; cluster: string; text: string; current_text: string; annotations: Record<string, string>; current_annotations: Record<string, string>; word: string | null; word_error: string | null };
type EditState = { edits: number; can_undo: boolean; can_redo: boolean };

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const ui = {
  mushaf: $<HTMLSelectElement>("mushaf"),
  page: $<HTMLInputElement>("page"),
  of: $("of"),
  prev: $<HTMLButtonElement>("prev"),
  next: $<HTMLButtonElement>("next"),
  hl: $<HTMLInputElement>("hl"),
  options: $<HTMLButtonElement>("btn-options"),
  edit: $<HTMLButtonElement>("btn-edit"),
  inspect: $<HTMLButtonElement>("btn-inspect"),
  prov: $<HTMLButtonElement>("btn-prov"),
  exportSel: $<HTMLSelectElement>("export"),
  status: $("status"),
  margin: $("margin"),
  view: $("pageview"),
  popover: $("popover"),
  panel: $("panel"),
  panelTitle: $("panel-title"),
  panelBody: $("panel-body"),
};

const state = {
  mushaf: "", // riwayah/style
  pages: 604,
  page: 1,
  view: null as View | null,
  panel: "" as "" | "options" | "edit" | "inspect" | "prov",
  selected: null as string | null, // cluster id selected in the editor
  renderMs: [] as number[], // engine time per page (resolve/fit/draw/SVG in the worker)
  turnMs: [] as number[], // page turn: request to SVG on screen
};

// Rule 9: nothing is fetched after load. The page is also exposed for the Playwright tests.
declare global {
  interface Window {
    mushaf: {
      ready: boolean;
      state: typeof state;
      go: (p: number) => Promise<void>;
      select: (m: string) => Promise<void>;
      /** For the Playwright tests: the page as the app paints it, as a PNG data URL. */
      paint: (page: number, width: number) => Promise<string>;
      /** For the Playwright tests: a raw engine call, then a redraw. */
      call: (method: string, ...args: unknown[]) => Promise<unknown>;
    };
  }
}

const store = {
  get: (k: string) => { try { return localStorage.getItem(k); } catch { return null; } },
  set: (k: string, v: string) => { try { localStorage.setItem(k, v); } catch { /* private mode */ } },
};

const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);
const status = (s: string) => (ui.status.textContent = s);

// ---- page view -------------------------------------------------------------------------

// Pages are painted in the worker into a bitmap (D63); a page turn only shows a bitmap that is
// already there. Neighbouring pages are painted ahead while the reader reads (T8.4). Any change
// of riwayah, choices, edits, highlighting or width starts a new generation and drops them.
type Prepared = { view: View; bitmap: ImageBitmap; ms: number; paintMs: number };
const ahead = new Map<string, Promise<Prepared>>();
const widthPx = () => Math.round(Math.max(200, ui.view.clientWidth) * (window.devicePixelRatio || 1));
const prepare = async (p: number, w: number): Promise<Prepared> => {
  const { result, ms } = await callTimed<{ view: string; bitmap: ImageBitmap; paintMs: number }>("page", p, ui.hl.checked, w);
  return { view: JSON.parse(result.view) as View, bitmap: result.bitmap, ms, paintMs: result.paintMs };
};
let gen = 0;
const invalidate = () => {
  gen++;
  for (const v of ahead.values()) void v.then((x) => x.bitmap.close()).catch(() => undefined);
  ahead.clear();
};
const viewOf = (p: number) => {
  const w = widthPx();
  const k = `${gen}:${p}:${ui.hl.checked}:${w}`;
  if (!ahead.has(k)) ahead.set(k, prepare(p, w));
  return ahead.get(k)!;
};
let resizeTimer = 0;
addEventListener("resize", () => {
  clearTimeout(resizeTimer);
  resizeTimer = window.setTimeout(() => {
    if (!state.view) return;
    invalidate();
    void render();
  }, 200);
});

let overlayToken = 0;
const pageCanvas = document.createElement("canvas");
pageCanvas.className = "page";
const overlay = document.createElement("div");
overlay.className = "overlay";

async function render() {
  const p = state.page;
  const t = performance.now();
  let prepared: Prepared;
  try {
    prepared = await viewOf(p);
  } catch (err) {
    if (p !== state.page) return;
    // The core cannot draw this page in this style (e.g. a word with no conversion row, G-102):
    // say so, and keep paging around it.
    ahead.delete(`${gen}:${p}:${ui.hl.checked}:${widthPx()}`);
    const note = document.createElement("div");
    note.className = "blocked";
    note.textContent = `Page ${p} cannot be drawn in this style yet: ${(err as Error).message ?? err}`;
    ui.view.replaceChildren(note);
    ui.view.dataset.page = String(p);
    ui.view.dataset.blocked = "1";
    ui.page.value = String(p);
    state.view = null;
    ui.margin.replaceChildren();
    status("page not drawn");
    for (const q of [p + 1, p - 1]) if (q >= 1 && q <= state.pages) void viewOf(q).catch(() => undefined);
    return;
  }
  delete ui.view.dataset.blocked;
  const { view: v, bitmap, ms } = prepared;
  if (p !== state.page) return; // a later page was asked for meanwhile
  state.view = v;
  state.renderMs.push(ms);
  state.turnMs.push(performance.now() - t);
  // Shown without a copy: the bitmap is handed to the canvas (and so used once; a later visit
  // asks the worker again, which keeps the drawn page).
  ahead.delete(`${gen}:${p}:${ui.hl.checked}:${bitmap.width}`);
  // The canvas takes the bitmap's size; otherwise it keeps its default 300x150 and stretches it.
  [pageCanvas.width, pageCanvas.height] = [bitmap.width, bitmap.height];
  pageCanvas.getContext("bitmaprenderer")!.transferFromImageBitmap(bitmap);
  overlay.replaceChildren();
  if (pageCanvas.parentElement !== ui.view) ui.view.replaceChildren(pageCanvas, overlay);
  ui.view.dataset.page = String(v.page);
  ui.view.dataset.canonical = String(v.canonical);
  ui.page.value = String(p);
  status(`${ms.toFixed(1)} ms${v.canonical ? "" : " · edited"}`);
  // A turn ends when the page has been painted: the frame after the swap. Then, out of the way
  // of the turn, the core's SVG goes on top, invisible, so a tap can name its cluster (D63).
  const token = ++overlayToken;
  void new Promise((ok) => requestAnimationFrame(() => setTimeout(ok, 0))).then(() => {
    state.turnMs[state.turnMs.length - 1] = performance.now() - t;
    if (p === state.page)
      status(`turn ${(performance.now() - t).toFixed(0)} ms · worker ${ms.toFixed(0)} ms${v.canonical ? "" : " · edited"}`);
    setTimeout(() => {
      if (token !== overlayToken) return;
      overlay.innerHTML = v.svg;
      ui.view.dataset.overlay = String(v.page);
      markSelection();
    }, 0);
  });
  delete ui.view.dataset.overlay;
  for (const k of [...ahead.keys()]) if (!k.startsWith(`${gen}:`) || Math.abs(Number(k.split(":")[1]) - p) > 2) ahead.delete(k);
  for (const q of [p + 1, p - 1]) if (q >= 1 && q <= state.pages) void viewOf(q).catch(() => undefined);
  await renderMargin();
  if (state.panel === "inspect") await showInspector();
  if (state.panel === "edit") await showEditor();
}

async function go(page: number) {
  state.page = Math.min(Math.max(1, page), state.pages);
  hidePopover();
  await render();
}

// RTL pager: the next page is to the left.
ui.next.onclick = () => go(state.page + 1);
ui.prev.onclick = () => go(state.page - 1);
ui.page.onchange = () => go(Number(ui.page.value) || 1);
document.addEventListener("keydown", (e) => {
  if ((e.target as HTMLElement).closest("input, textarea, select")) return;
  if (e.key === "ArrowLeft") go(state.page + 1);
  if (e.key === "ArrowRight") go(state.page - 1);
});
let touchX: number | null = null;
ui.view.addEventListener("touchstart", (e) => (touchX = e.touches[0].clientX), { passive: true });
ui.view.addEventListener("touchend", (e) => {
  if (touchX === null) return;
  const dx = e.changedTouches[0].clientX - touchX;
  touchX = null;
  if (Math.abs(dx) > 60) go(state.page + (dx > 0 ? 1 : -1)); // swipe right = next (RTL)
});

// ---- riwayah picker, highlight toggle --------------------------------------------------

const hlKey = () => `mushaf.highlight.${state.mushaf}`;

async function select(mushaf: string) {
  const [riwayah, style] = mushaf.split("/");
  status(`opening ${riwayah} (${style})…`);
  const { ms } = await callTimed("select", riwayah, style);
  invalidate();
  state.mushaf = mushaf;
  state.selected = null;
  ui.mushaf.value = mushaf;
  store.set("mushaf.current", mushaf);
  // The toggle is kept per muṣḥaf (D37); plain by default.
  ui.hl.checked = store.get(hlKey()) === "1";
  state.pages = await call<number>("pages");
  ui.page.max = String(state.pages);
  ui.of.textContent = `/ ${state.pages}`;
  const groups = await callJson<OptionGroup[]>("option_groups");
  ui.options.hidden = groups.length === 0;
  if (state.panel === "options" && groups.length === 0) closePanel();
  status(`opened in ${ms.toFixed(0)} ms`);
  await go(state.page);
  if (state.panel === "prov") await showProvenance();
  if (state.panel === "options") await showOptions();
}

ui.mushaf.onchange = () => select(ui.mushaf.value);
ui.hl.onchange = () => {
  store.set(hlKey(), ui.hl.checked ? "1" : "0");
  invalidate();
  render();
};

// ---- tap for detail (D37) and editor selection ----------------------------------------

function hidePopover() {
  ui.popover.hidden = true;
}

ui.view.addEventListener("click", (e) => {
  const use = (e.target as Element).closest("use[data-c]");
  if (!use || !state.view) return hidePopover();
  const cluster = use.getAttribute("data-c")!;
  const line = Number(use.getAttribute("data-l"));
  if (state.panel === "edit") {
    state.selected = cluster;
    markSelection();
    showEditor();
    return;
  }
  const hs = state.view.highlights.filter((h) => h.cluster === cluster && h.line === line);
  if (hs.length === 0) return hidePopover();
  ui.popover.innerHTML = hs
    .map(
      (h) => `<div class="hl ${h.cited ? "cited" : "hypothesis"}">
        <div><b>${esc(h.label)}</b> <code>${esc(h.cluster)}</code></div>
        <div>Ḥafṣ: <code>${h.hafs ? esc(h.hafs) : "(inserted, no Ḥafṣ cluster)"}</code></div>
        <div>${h.owner !== undefined ? "Marked by you, not by a rule" : h.cite ? `Source: ${esc(h.cite)}` : "Hypothesis: no citation yet"}</div>
        ${h.annotation ? "<div>Pronunciation only (annotation)</div>" : ""}
        ${h.owner !== undefined ? `<div><b>Your mark</b>${h.owner ? `: ${esc(h.owner)}` : ""}</div>` : ""}
      </div>`,
    )
    .join("");
  const r = (e as MouseEvent);
  ui.popover.style.left = `${Math.min(r.clientX, innerWidth - 320)}px`;
  ui.popover.style.top = `${r.clientY + 12}px`;
  ui.popover.hidden = false;
});

function markSelection() {
  ui.view.querySelectorAll("use.selected").forEach((u) => u.classList.remove("selected"));
  if (state.selected && state.panel === "edit")
    ui.view.querySelectorAll(`use[data-c="${CSS.escape(state.selected)}"]`).forEach((u) => u.classList.add("selected"));
}

// ---- margin khilaf notes (D60) ---------------------------------------------------------

async function renderMargin() {
  const notes = await callJson<MarginNote[]>("margin_notes", state.page);
  const v = state.view!;
  ui.margin.innerHTML = "";
  ui.margin.hidden = notes.length === 0;
  ui.view.classList.toggle("has-notes", notes.length > 0);
  const base = new Map(v.baselines);
  const ann = (a: Record<string, string>) => Object.entries(a).map(([k, x]) => `${k}: ${x}`).join(", ");
  const byLine = new Map<number, MarginNote[]>();
  for (const n of notes) byLine.set(n.line, [...(byLine.get(n.line) ?? []), n]);
  for (const [line, ns] of byLine) {
    const el = document.createElement("div");
    el.className = "note";
    el.dataset.line = String(line);
    // Placed beside its line; the page itself is not touched, so lines never move.
    el.style.top = `${(100 * (base.get(line) ?? 0)) / v.height}%`;
    el.innerHTML = `<button class="note-dot" aria-label="Khilaf note, line ${line}">◆</button>
      <div class="note-body">${ns
        .map(
          (n) => `<div class="note-item" data-c="${esc(n.cluster)}">
            <div class="note-title">${esc(n.title_ar)} — ${esc(n.choice)}</div>
            ${n.word ? `<div class="note-word">${n.word}</div>` : `<div class="note-err">${esc(n.word_error ?? "deleted under this wajh")}</div>`}
            ${n.text !== n.current_text ? `<div><code>${esc(n.text || "(none)")}</code></div>` : ""}
            ${ann(n.annotations) !== ann(n.current_annotations) ? `<div>${esc(ann(n.annotations) || "no annotation")}</div>` : ""}
          </div>`,
        )
        .join("")}</div>`;
    el.querySelector(".note-dot")!.addEventListener("click", () => el.classList.toggle("open"));
    ui.margin.append(el);
  }
}

// ---- panels ----------------------------------------------------------------------------

function openPanel(kind: typeof state.panel, title: string) {
  state.panel = kind;
  ui.panel.hidden = false;
  ui.panelTitle.textContent = title;
  document.body.classList.toggle("editing", kind === "edit");
  document.body.classList.add("panel-open");
  markSelection();
}
function closePanel() {
  state.panel = "";
  ui.panel.hidden = true;
  document.body.classList.remove("editing", "panel-open");
  markSelection();
}
$("panel-close").onclick = closePanel;
const toggle = (kind: typeof state.panel, show: () => Promise<void>) => () =>
  state.panel === kind ? closePanel() : show();

// Options: the core's groups for this riwayah, defaults from the data.
async function showOptions() {
  openPanel("options", "Reading options");
  const groups = await callJson<OptionGroup[]>("option_groups");
  ui.panelBody.innerHTML = groups
    .map(
      (g) => `<fieldset data-group="${esc(g.id)}"><legend dir="rtl">${esc(g.title_ar)}</legend>
        ${g.choices
          .map(
            (c) => `<label><input type="radio" name="${esc(g.id)}" value="${esc(c)}" ${c === g.current ? "checked" : ""}/>
              ${esc(c)}${c === g.default ? " (default)" : ""}</label>`,
          )
          .join("<br/>")}</fieldset>`,
    )
    .join("");
  ui.panelBody.querySelectorAll("input[type=radio]").forEach((i) =>
    i.addEventListener("change", async () => {
      const choices: Record<string, string> = {};
      ui.panelBody.querySelectorAll<HTMLInputElement>("input[type=radio]:checked").forEach((x) => (choices[x.name] = x.value));
      await call("set_choices", JSON.stringify(choices));
      invalidate();
      await render();
    }),
  );
}
ui.options.onclick = toggle("options", showOptions);

// Free-edit editor (Phase 7 EditSession). Never refuses a non-canonical edit (rule 11).
async function showEditor() {
  openPanel("edit", "Free edit");
  const st = await callJson<EditState>("edit_state");
  const lint = await callJson<{ item: string; change: string; matches: string[] }[]>("lint");
  const sel = state.selected;
  ui.panelBody.innerHTML = `
    <p>${sel ? `Selected cluster <code id="sel">${esc(sel)}</code>` : "Tap a letter on the page, or pick a word below."}</p>
    <div class="ops"><label>Cluster or word id <input id="pick-id" placeholder="2:5:3:1 or 2:5:3" size="16"/></label>
      <button id="pick-go">Select</button></div>
    <details id="words" open><summary>Words on this page</summary><div id="word-list">loading…</div></details>
    <div class="ops" ${sel ? "" : "hidden"}>
      <button data-op="delete-cluster">Delete cluster</button>
      <button data-op="delete-word">Delete word</button>
      <label>Mark <input id="cp" placeholder="U+0652" size="8"/></label>
      <button data-op="add_mark">Add mark</button>
      <button data-op="remove_mark">Remove mark</button>
      <label>Codepoints <input id="cps" placeholder="U+0645 U+064E" size="20"/></label>
      <button data-op="replace_cluster">Replace cluster</button>
      <button data-op="insert-word">Insert word after</button>
      <button data-op="insert-cluster">Insert letter after</button>
      <label>Value <input id="val" type="number" placeholder="1/64 em"/></label>
      <button data-op="set_width">Set width</button>
      <button data-op="set_stretch">Set stretch</button>
    </div>
    <div class="ops">
      <button data-op="undo" ${st.can_undo ? "" : "disabled"}>Undo</button>
      <button data-op="redo" ${st.can_redo ? "" : "disabled"}>Redo</button>
      <button data-op="clear" ${st.edits ? "" : "disabled"}>Clear all</button>
      <button id="exp-yaml" ${st.edits ? "" : "disabled"}>Export edits</button>
      <label class="file">Import edits <input id="imp-yaml" type="file" accept=".yaml,.yml"/></label>
    </div>
    <p>${st.edits} edit(s). ${st.edits ? "The page carries the “edited” marker." : ""}</p>
    <div id="edit-error" class="error"></div>
    <h3>Highlights</h3>
    <div class="ops" ${sel ? "" : "hidden"}>
      <button id="hm-off">Not a difference</button>
      <button id="hm-on">Should be highlighted</button>
      <button id="hm-clear">Clear my mark</button>
      <label>Note <input id="hm-note" size="24" placeholder="why (optional)"/></label>
    </div>
    <div id="hm-list"></div>
    <div class="ops">
      <button id="hm-exp">Export highlight marks</button>
      <label class="file">Import highlight marks <input id="hm-imp" type="file" accept=".yaml,.yml"/></label>
    </div>
    ${lint.length ? `<h3>Lint (advisory)</h3><ul>${lint.map((l) => `<li><code>${esc(String(l.item))}</code> ${esc(l.change)}${l.matches.length ? ` — matches ${esc(l.matches.join(", "))}` : ""}</li>`).join("")}</ul>` : ""}`;
  ui.panelBody.querySelectorAll<HTMLButtonElement>("button[data-op]").forEach((b) => (b.onclick = () => applyEdit(b.dataset.op!)));
  const pick = (id: string) => {
    state.selected = id;
    markSelection();
    void showEditor();
  };
  $("pick-go").onclick = () => {
    const v = $<HTMLInputElement>("pick-id").value.trim();
    if (!v) return;
    // a word id (s:a:w) selects its first cluster
    pick(v.split(":").length === 3 ? `${v}:1` : v);
  };
  // The page's words, from the core's inspector: line by line, each word's clusters. The
  // labels are the core's codepoints shown as characters, only to help pick one.
  const insp = await callJson<{ clusters: { line: number; id: string; text: string }[] }>("inspect", state.page);
  const chars = (u: string) => u.split(" ").map((x) => String.fromCodePoint(parseInt(x.slice(2), 16))).join("");
  const byWord = new Map<string, { line: number; clusters: { id: string; text: string }[] }>();
  for (const c of insp.clusters) {
    const w = c.id.replace(/:\d+(\+ins\.\d+)*$/, "");
    if (!byWord.has(w)) byWord.set(w, { line: c.line, clusters: [] });
    byWord.get(w)!.clusters.push({ id: c.id, text: c.text });
  }
  const lines = new Map<number, string[]>();
  for (const [w, x] of byWord) lines.set(x.line, [...(lines.get(x.line) ?? []), w]);
  $("word-list").innerHTML = [...lines]
    .map(
      ([l, ws]) => `<div class="wl"><span class="wl-n">${l}</span>${ws
        .map((w) => {
          const x = byWord.get(w)!;
          const open = sel !== null && x.clusters.some((c) => c.id === sel);
          return `<span class="w${open ? " open" : ""}"><button class="w-btn" data-w="${esc(w)}" title="${esc(w)}" dir="rtl">${esc(x.clusters.map((c) => chars(c.text)).join(""))}</button>${
            open ? x.clusters.map((c) => `<button class="c-btn${c.id === sel ? " on" : ""}" data-c="${esc(c.id)}" title="${esc(c.id)} ${esc(c.text)}">${esc(chars(c.text))}</button>`).join("") : ""
          }</span>`;
        })
        .join("")}</div>`,
    )
    .join("");
  $("word-list").querySelectorAll<HTMLButtonElement>(".w-btn").forEach((b) => (b.onclick = () => pick(byWord.get(b.dataset.w!)!.clusters[0].id)));
  $("word-list").querySelectorAll<HTMLButtonElement>(".c-btn").forEach((b) => (b.onclick = () => pick(b.dataset.c!)));
  // The owner's highlight marks (D64): display and feedback only, computed by the core.
  const marks = await callJson<{ cluster: string; show: boolean; note: string }[]>("highlight_marks");
  $("hm-list").innerHTML = marks.length
    ? `<ul>${marks.map((m) => `<li><code>${esc(m.cluster)}</code> ${m.show ? "should be highlighted" : "not a difference"}${m.note ? ` — ${esc(m.note)}` : ""}</li>`).join("")}</ul>`
    : "<p>No highlight marks.</p>";
  const mark = async (m: { show: boolean; note: string } | null) => {
    await call("set_highlight_mark", sel, JSON.stringify(m));
    if (!ui.hl.checked) {
      ui.hl.checked = true;
      store.set(hlKey(), "1");
    }
    invalidate();
    await render();
  };
  const note = () => $<HTMLInputElement>("hm-note").value;
  $("hm-off").onclick = () => mark({ show: false, note: note() });
  $("hm-on").onclick = () => mark({ show: true, note: note() });
  $("hm-clear").onclick = () => mark(null);
  $("hm-exp").onclick = async () => download(`highlights-${state.mushaf.replace("/", "-")}.yaml`, await call<string>("export_highlight_marks"), "text/yaml");
  $<HTMLInputElement>("hm-imp").onchange = async (e) => {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (!f) return;
    try {
      await call("import_highlight_marks", await f.text());
      invalidate();
      await render();
    } catch (err) {
      $("edit-error").textContent = String(err);
    }
  };
  $("exp-yaml").onclick = async () => download(`edits-${state.mushaf.replace("/", "-")}.yaml`, await call<string>("export_edits"), "text/yaml");
  $<HTMLInputElement>("imp-yaml").onchange = async (e) => {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (!f) return;
    try {
      await call("import_edits", await f.text());
      invalidate();
      await render();
    } catch (err) {
      $("edit-error").textContent = String(err);
    }
  };
}

const cpsList = (s: string) => s.trim().split(/\s+/).filter(Boolean);

async function applyEdit(kind: string) {
  const sel = state.selected!;
  // A cluster id is `<item id>:<n>` plus any `+ins.<k>` (CLAUDE.md conventions).
  const itemOf = (c: string) => c.replace(/:\d+(\+ins\.\d+)*$/, "");
  const val = $<HTMLInputElement>("val")?.value;
  const value = val === "" || val === undefined ? null : Number(val);
  const cps = $<HTMLInputElement>("cps")?.value ?? "";
  const cp = ($<HTMLInputElement>("cp")?.value ?? "").trim();
  const ops: Record<string, () => object> = {
    "delete-cluster": () => ({ op: "delete", target: sel }),
    "delete-word": () => ({ op: "delete", target: itemOf(sel) }),
    add_mark: () => ({ op: "add_mark", target: sel, cp, at: null }),
    remove_mark: () => ({ op: "remove_mark", target: sel, cp }),
    replace_cluster: () => ({ op: "replace_cluster", target: sel, cluster: { kind: "letter", cps: cpsList(cps).join(" ") } }),
    "insert-word": () => ({
      op: "insert", anchor: itemOf(sel), after: true,
      item: { kind: "word", sep_after: "U+0020", clusters: [{ kind: "letter", cps: cpsList(cps).join(" ") }] },
    }),
    "insert-cluster": () => ({ op: "insert", anchor: sel, after: true, cluster: { kind: "letter", cps: cpsList(cps).join(" ") } }),
    set_width: () => ({ op: "set_width", target: sel, value }),
    set_stretch: () => ({ op: "set_stretch", target: sel, value }),
    undo: () => ({ op: "undo" }),
    redo: () => ({ op: "redo" }),
    clear: () => ({ op: "clear" }),
  };
  try {
    await call("edit", JSON.stringify(ops[kind]()));
    invalidate();
    await render();
  } catch (err) {
    $("edit-error").textContent = String((err as Error).message ?? err);
  }
}
ui.edit.onclick = toggle("edit", showEditor);

// Engine inspector: stacks, choices, report, per-cluster provenance.
async function showInspector() {
  openPanel("inspect", `Engine inspector — page ${state.page}`);
  const i = await callJson<any>("inspect", state.page);
  const prov = (p: any) => `<span class="st st-${esc(p.effective_status)}" title="${esc(p.source ?? "")}">${esc(p.id)} · ${esc(p.effective_status)}${p.single_source ? " · single-source" : ""}</span>`;
  ui.panelBody.innerHTML = `
    <dl>
      <dt>Riwāyah / style</dt><dd>${esc(i.riwayah)} / ${esc(i.style)} — ${i.canonical ? "canonical" : "<b class='error'>edited</b>"}</dd>
      <dt>Profile</dt><dd>${esc(i.profile)}</dd>
      <dt>Layout · font · spine</dt><dd>${esc(i.layout_id)} · ${esc(i.font)} · ${esc(i.spine)}</dd>
      <dt>Patch stack</dt><dd>${i.patch_stack.map(esc).join(" → ")}${i.user_patches.length ? ` → user (${i.user_patches.length})` : ""}</dd>
      <dt>Rule stack</dt><dd>${i.rule_stack.map(esc).join(", ")}</dd>
      <dt>Choices</dt><dd>${Object.entries(i.choices).map(([k, v]) => `${esc(k)}=${esc(String(v))}`).join(", ") || "—"}</dd>
      <dt>Report</dt><dd>${i.report.length ? `<pre>${esc(JSON.stringify(i.report, null, 1))}</pre>` : "none"}</dd>
      <dt>Fit notes</dt><dd>${i.notes.length ? `<pre>${esc(JSON.stringify(i.notes))}</pre>` : "none"}</dd>
    </dl>
    <input id="insp-filter" placeholder="Filter by id, rule or status"/>
    <table class="insp"><thead><tr><th>Line</th><th>Cluster</th><th>Text</th><th>Ḥafṣ</th><th>Provenance</th></tr></thead><tbody>
    ${i.clusters
      .map(
        (c: any) => `<tr data-c="${esc(c.id)}" class="${c.provenance.length ? "changed" : ""}"><td>${c.line}</td><td><code>${esc(c.id)}</code></td>
          <td><code>${esc(c.text)}</code></td><td><code>${c.hafs === c.text ? "=" : esc(c.hafs || "(inserted)")}</code></td>
          <td>${c.provenance.map(prov).join(" ")}${c.attrs ? ` <code>${esc(JSON.stringify(c.attrs))}</code>` : ""}</td></tr>`,
      )
      .join("")}</tbody></table>`;
  const f = $<HTMLInputElement>("insp-filter");
  f.oninput = () =>
    ui.panelBody.querySelectorAll<HTMLTableRowElement>("table.insp tbody tr").forEach((r) => (r.hidden = !r.textContent!.includes(f.value)));
  ui.panelBody.querySelectorAll<HTMLTableRowElement>("table.insp tbody tr").forEach(
    (r) => (r.onmouseenter = () => ui.view.querySelectorAll(`use[data-c="${CSS.escape(r.dataset.c!)}"]`).forEach((u) => u.classList.add("peek"))),
  );
  ui.panelBody.querySelector("table.insp")!.addEventListener("mouseleave", () => ui.view.querySelectorAll("use.peek").forEach((u) => u.classList.remove("peek")));
}
ui.inspect.onclick = toggle("inspect", showInspector);

// Provenance and verification status.
async function showProvenance() {
  openPanel("prov", "Provenance and verification status");
  const p = await callJson<any>("provenance");
  ui.panelBody.innerHTML = `
    <dl>
      <dt>Riwāyah / style</dt><dd>${esc(p.riwayah)} / ${esc(p.style)}</dd>
      <dt>Text source</dt><dd>${esc(p.text_source)}${p.single_source ? " (single-source)" : ""}</dd>
      <dt>Layout</dt><dd>${esc(p.layout_id)}: ${Object.entries(p.layout_sources).map(([k, v]) => `${esc(k)}=${esc(String(v))}`).join(", ")}</dd>
      <dt>Font</dt><dd>${esc(p.font)}</dd>
    </dl>
    <h3>Patch files</h3>
    <table><thead><tr><th>File</th><th>Patches</th><th>By status</th><th>Single-source</th><th>Owner-approved</th></tr></thead><tbody>
      ${p.patch_files.map((f: any) => `<tr><td>${esc(f.path)}</td><td>${f.patches}</td><td>${Object.entries(f.by_status).map(([k, v]) => `${esc(k)}: ${v}`).join(", ")}</td><td>${f.single_source}</td><td>${f.approved}</td></tr>`).join("")}
    </tbody></table>
    <h3>Rules (${p.rules.length})</h3>
    <table><thead><tr><th>Rule</th><th>Status</th><th>Source</th><th>Option</th><th>Description</th></tr></thead><tbody>
      ${p.rules.map((r: any) => `<tr><td><code>${esc(r.id)}</code></td><td class="st-${esc(r.effective_status)}">${esc(r.effective_status)}${r.single_source ? " · single-source" : ""}</td><td>${esc(r.source ?? "—")}</td><td>${esc(r.option ?? "")}</td><td>${esc(r.description)}</td></tr>`).join("")}
    </tbody></table>
    <h3>Owner approvals (${p.approvals.length})</h3>
    <table><thead><tr><th>Target</th><th>Status</th><th>By</th><th>Date</th><th>Note</th></tr></thead><tbody>
      ${p.approvals.map((a: any) => `<tr><td><code>${esc(a.target_ref)}</code></td><td>${esc(a.status)}</td><td>${esc(a.by)}</td><td>${esc(a.date)}</td><td>${esc(a.note)}</td></tr>`).join("")}
    </tbody></table>`;
}
ui.prov.onclick = toggle("prov", showProvenance);

// ---- export (every export carries the edited marker: the core draws it, rule 11) -----------

function download(name: string, data: BlobPart, type: string) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(new Blob([data], { type }));
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

ui.exportSel.onchange = async () => {
  const kind = ui.exportSel.value;
  ui.exportSel.value = "";
  const base = `${state.mushaf.replace("/", "-")}-p${String(state.page).padStart(3, "0")}`;
  if (kind === "json") download(`${base}.json`, await call<string>("render", state.page, ui.hl.checked), "application/json");
  if (kind === "svg") download(`${base}.svg`, await call<string>("svg", state.page, ui.hl.checked), "image/svg+xml");
  if (kind === "png") download(`${base}.png`, await pngOf(await call<string>("svg", state.page, ui.hl.checked), 1200), "image/png");
};

/** Rasterises the core's SVG (the browser only paints it). */
export async function pngOf(svg: string, width: number): Promise<Blob> {
  // Paint the vector at the target size (not a resample of a smaller raster).
  const root = new DOMParser().parseFromString(svg, "image/svg+xml").documentElement;
  const [vw, vh] = root.getAttribute("viewBox")!.split(" ").slice(2).map(Number);
  const height = Math.round((vh * width) / vw);
  root.setAttribute("width", String(width));
  root.setAttribute("height", String(height));
  const img = new Image();
  img.src = URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(root)], { type: "image/svg+xml" }));
  await img.decode();
  const c = document.createElement("canvas");
  c.width = width;
  c.height = height;
  c.getContext("2d")!.drawImage(img, 0, 0, c.width, c.height);
  URL.revokeObjectURL(img.src);
  return new Promise((ok) => c.toBlob((b) => ok(b!), "image/png"));
}

// ---- start -----------------------------------------------------------------------------

async function start() {
  if ("serviceWorker" in navigator) await navigator.serviceWorker.register("/sw.js").catch(() => undefined);
  status("loading data…");
  const info = await call<{ bytes: number; parseMs: number }>("init");
  status(`data ${(info.bytes / 1e6).toFixed(0)} MB parsed in ${(info.parseMs / 1000).toFixed(1)} s`);
  const styles = await callJson<StyleOption[]>("styles");
  ui.mushaf.innerHTML = styles
    .map((s) => `<option value="${s.riwayah}/${s.style}">${s.riwayah} — ${s.style}${s.default ? "" : " (converted)"}</option>`)
    .join("");
  const saved = store.get("mushaf.current");
  const first = styles.find((s) => `${s.riwayah}/${s.style}` === saved) ?? styles.find((s) => s.riwayah === "hafs" && s.default) ?? styles[0];
  const q = new URLSearchParams(location.search);
  state.page = Number(q.get("page")) || 1;
  await select(q.get("mushaf") ?? `${first.riwayah}/${first.style}`);
  window.mushaf.ready = true;
}

window.mushaf = {
  ready: false,
  state,
  go,
  select,
  paint: async (page, width) => {
    const r = await call<{ bitmap: ImageBitmap }>("page", page, false, width);
    const c = document.createElement("canvas");
    [c.width, c.height] = [r.bitmap.width, r.bitmap.height];
    c.getContext("2d")!.drawImage(r.bitmap, 0, 0);
    return c.toDataURL("image/png");
  },
  call: async (method, ...args) => {
    const r = await call(method, ...args);
    invalidate();
    await render();
    return r;
  },
};
start().catch((e) => status(`error: ${e.message ?? e}`));
