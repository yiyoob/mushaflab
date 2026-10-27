// Paints a core DrawPage into an OffscreenCanvas (D63). It replays the core's draw commands
// as they are: each glyph's outline under its transform, then fills, then strokes, in the
// colours of the core's SVG backend (mushaf-draw/src/svg.rs). It decides nothing (rule 7).

type PathOp =
  | { op: "move" | "line"; x: number; y: number }
  | { op: "quad"; x1: number; y1: number; x: number; y: number }
  | { op: "cubic"; x1: number; y1: number; x2: number; y2: number; x: number; y: number }
  | { op: "close" };
type Item =
  | { kind: "glyph"; glyph: number; x: number; y: number; sx: number; sy: number; line: number; cluster?: string }
  | { kind: "fill"; role: string; line: number; path: PathOp[] }
  | { kind: "stroke"; role: string; width: number; path: PathOp[] };
export type DrawPage = {
  width: number;
  height: number;
  glyphs: Record<string, PathOp[]>;
  items: Item[];
  highlights?: { line: number; cluster: string }[];
};

const U = 64; // 26.6
const S = 65536; // 16.16

function path(ops: PathOp[]): Path2D {
  const p = new Path2D();
  for (const o of ops) {
    if (o.op === "move") p.moveTo(o.x / U, o.y / U);
    else if (o.op === "line") p.lineTo(o.x / U, o.y / U);
    else if (o.op === "quad") p.quadraticCurveTo(o.x1 / U, o.y1 / U, o.x / U, o.y / U);
    else if (o.op === "cubic") p.bezierCurveTo(o.x1 / U, o.y1 / U, o.x2 / U, o.y2 / U, o.x / U, o.y / U);
    else p.closePath();
  }
  return p;
}

/** The page at `widthPx` pixels wide (height keeps the page's proportions). */
export function paint(d: DrawPage, widthPx: number): ImageBitmap {
  const [w, h] = [d.width / U, d.height / U];
  const heightPx = Math.round((h * widthPx) / w);
  const c = new OffscreenCanvas(widthPx, heightPx);
  const g = c.getContext("2d")!;
  const k = widthPx / w;
  g.fillStyle = "#ffffff";
  g.fillRect(0, 0, widthPx, heightPx);
  const outlines = new Map<string, Path2D>();
  const marked = new Set((d.highlights ?? []).map((x) => `${x.line} ${x.cluster}`));
  for (const it of d.items) {
    if (it.kind !== "glyph") continue;
    const key = String(it.glyph);
    if (!outlines.has(key)) outlines.set(key, path(d.glyphs[key]));
    // SVG: matrix(sx 0 0 -sy x y) under the page scale
    g.setTransform((k * it.sx) / S, 0, 0, (-k * it.sy) / S, (k * it.x) / U, (k * it.y) / U);
    g.fillStyle = it.cluster && marked.has(`${it.line} ${it.cluster}`) ? "#cc0000" : "#000000";
    g.fill(outlines.get(key)!);
  }
  g.setTransform(k, 0, 0, k, 0, 0);
  g.fillStyle = "#000000";
  for (const it of d.items) if (it.kind === "fill") g.fill(path(it.path));
  g.lineJoin = "round";
  g.lineCap = "round";
  for (const it of d.items) {
    if (it.kind !== "stroke") continue;
    g.strokeStyle = it.role === "edited_marker" ? "#cc0000" : "#000000";
    g.lineWidth = it.width / U;
    g.stroke(path(it.path));
  }
  return c.transferToImageBitmap();
}
