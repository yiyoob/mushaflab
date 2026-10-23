//! Ink collisions between adjacent lines (host-only check for D15's line pitch).
//!
//! Each line's ink (its glyph outlines, plus a surah-header frame) is rasterised without
//! anti-aliasing at `RES` page units per pixel. For every column of pixels the ink of the upper
//! line and of the lower line are lists of runs; moving the lower line up by `d` pixels makes a
//! run pair overlap exactly when `d` lies in `[b0 - a1, b1 - a0]`. From those ranges each pair of
//! lines gets: whether it collides at the drawn pitch, and the tightest pitch at which it does
//! not collide (every pitch between that and the drawn one is free for this pair).
//!
//! Floating point is used here, in a host test tool after the core's output is final (D10).

use mushaf_draw::{DrawItem, DrawPage, PathOp, Role};
use resvg::tiny_skia::{self, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// Page units (font units at page scale 1) per pixel.
pub const RES: f32 = 10.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairClearance {
    pub page: u32,
    /// Line numbers of the upper and lower line.
    pub upper: u32,
    pub lower: u32,
    /// The two lines' ink touches or overlaps at the drawn pitch.
    pub collides: bool,
    /// The tightest line pitch (page units) at which this pair is free of collisions, with
    /// every pitch from there up to the drawn one also free (or, if it collides now, the
    /// smallest larger pitch that is free).
    pub tightest_pitch: i64,
}

/// Ink runs per pixel column: `runs[x]` = list of `[y0, y1]` (inclusive, absolute pixels).
struct Ink {
    runs: Vec<Vec<(i32, i32)>>,
}

const F: f32 = 64.0;

fn glyph_path(pb: &mut PathBuilder, outline: &[PathOp], (x, y, sx, sy): (i64, i64, i64, i64)) {
    let (sx, sy) = (sx as f32 / 65536.0, sy as f32 / 65536.0);
    let px = |gx: i32| (x as f32 + gx as f32 * sx) / F / RES;
    let py = |gy: i32| (y as f32 - gy as f32 * sy) / F / RES;
    for op in outline {
        match *op {
            PathOp::Move { x, y } => pb.move_to(px(x), py(y)),
            PathOp::Line { x, y } => pb.line_to(px(x), py(y)),
            PathOp::Quad { x1, y1, x, y } => pb.quad_to(px(x1), py(y1), px(x), py(y)),
            PathOp::Cubic {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => pb.cubic_to(px(x1), py(y1), px(x2), py(y2), px(x), py(y)),
            PathOp::Close => pb.close(),
        }
    }
}

fn page_path(path: &[PathOp]) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let p = |v: i32| v as f32 / F / RES;
    for op in path {
        match *op {
            PathOp::Move { x, y } => pb.move_to(p(x), p(y)),
            PathOp::Line { x, y } => pb.line_to(p(x), p(y)),
            PathOp::Quad { x1, y1, x, y } => pb.quad_to(p(x1), p(y1), p(x), p(y)),
            PathOp::Cubic {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => pb.cubic_to(p(x1), p(y1), p(x2), p(y2), p(x), p(y)),
            PathOp::Close => pb.close(),
        }
    }
    pb.finish()
}

/// Rasterise only the band of rows the ink can reach (its control-point bounds), then report
/// runs in absolute page pixels.
fn rasterise(
    width: u32,
    fill: Option<&tiny_skia::Path>,
    strokes: &[(tiny_skia::Path, f32)],
) -> Ink {
    let mut top = f32::MAX;
    let mut bottom = f32::MIN;
    if let Some(p) = fill {
        top = top.min(p.bounds().top());
        bottom = bottom.max(p.bounds().bottom());
    }
    for (p, w) in strokes {
        top = top.min(p.bounds().top() - w);
        bottom = bottom.max(p.bounds().bottom() + w);
    }
    if top > bottom {
        return Ink {
            runs: vec![Vec::new(); width as usize],
        };
    }
    let band_top = top.floor() as i32 - 1;
    let height = (bottom.ceil() as i32 + 1 - band_top).max(1) as u32;
    let shift = Transform::from_translate(0.0, -band_top as f32);
    let mut pm = Pixmap::new(width, height).expect("band size");
    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 0, 0, 255);
    paint.anti_alias = false;
    if let Some(p) = fill {
        pm.fill_path(p, &paint, FillRule::Winding, shift, None);
    }
    for (p, w) in strokes {
        let stroke = Stroke {
            width: *w,
            ..Stroke::default()
        };
        pm.stroke_path(p, &paint, &stroke, shift, None);
    }
    let mut runs = vec![Vec::new(); width as usize];
    let data = pm.data();
    for (x, col) in runs.iter_mut().enumerate() {
        let mut start: Option<i32> = None;
        for y in 0..height as usize {
            let ink = data[(y * width as usize + x) * 4 + 3] > 0;
            match (ink, start) {
                (true, None) => start = Some(band_top + y as i32),
                (false, Some(s)) => {
                    col.push((s, band_top + y as i32 - 1));
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(s) = start {
            col.push((s, band_top + height as i32 - 1));
        }
    }
    Ink { runs }
}

/// Ink of every line on the page, in line order.
fn line_inks(d: &DrawPage) -> Vec<(u32, Ink)> {
    let width = (d.width as f32 / F / RES).ceil() as u32 + 1;
    let mut out = Vec::new();
    for l in &d.lines {
        let mut pb = PathBuilder::new();
        for item in &d.items {
            if let DrawItem::Glyph {
                glyph,
                x,
                y,
                sx,
                sy,
                line,
                ..
            } = item
                && *line == l.line
            {
                glyph_path(&mut pb, &d.glyphs[glyph], (*x, *y, *sx, *sy));
            }
            if let DrawItem::Fill { line, path, .. } = item
                && *line == l.line
            {
                // page units, y down: the identity transform of `glyph_path`
                glyph_path(&mut pb, path, (0, 0, 65536, -65536));
            }
        }
        // a frame belongs to the line whose baseline is nearest its middle
        let mut strokes = Vec::new();
        for item in &d.items {
            if let DrawItem::Stroke {
                role: Role::Frame,
                width: w,
                path,
            } = item
            {
                let ys: Vec<i64> = path
                    .iter()
                    .filter_map(|op| match *op {
                        PathOp::Move { y, .. } | PathOp::Line { y, .. } => Some(i64::from(y)),
                        _ => None,
                    })
                    .collect();
                let mid = (ys.iter().min().unwrap() + ys.iter().max().unwrap()) / 2;
                let nearest = d
                    .lines
                    .iter()
                    .min_by_key(|m| (m.baseline - mid).abs())
                    .unwrap();
                if nearest.line == l.line
                    && let Some(p) = page_path(path)
                {
                    strokes.push((p, *w as f32 / F / RES));
                }
            }
        }
        out.push((l.line, rasterise(width, pb.finish().as_ref(), &strokes)));
    }
    out
}

fn clearance(page: u32, upper: (u32, &Ink), lower: (u32, &Ink), pitch: i64) -> PairClearance {
    // forbidden shifts d (pixels the lower line moves up) per run pair
    let mut forbidden = Vec::new();
    for (a, b) in upper.1.runs.iter().zip(&lower.1.runs) {
        for &(a0, a1) in a {
            for &(b0, b1) in b {
                forbidden.push((b0 - a1, b1 - a0));
            }
        }
    }
    let hit = |d: i32| forbidden.iter().any(|&(lo, hi)| lo <= d && d <= hi);
    let collides = hit(0);
    let res = RES as i64;
    let tightest_pitch = if collides {
        // move apart until free
        let mut e = 0;
        while hit(e) {
            e = forbidden
                .iter()
                .filter(|&&(lo, hi)| lo <= e && e <= hi)
                .map(|r| r.0)
                .min()
                .unwrap()
                - 1;
        }
        pitch - i64::from(e) * res
    } else {
        let first = forbidden
            .iter()
            .filter(|r| r.1 >= 0)
            .map(|r| r.0.max(0))
            .min();
        match first {
            Some(d) => pitch - i64::from(d - 1) * res,
            None => 0,
        }
    };
    PairClearance {
        page,
        upper: upper.0,
        lower: lower.0,
        collides,
        tightest_pitch,
    }
}

/// Every adjacent pair of lines on a page. `pitch` is the drawn line pitch in page units.
pub fn line_pairs(d: &DrawPage, pitch: i64) -> Vec<PairClearance> {
    let inks = line_inks(d);
    inks.windows(2)
        .map(|w| clearance(d.page, (w[0].0, &w[0].1), (w[1].0, &w[1].1), pitch))
        .collect()
}
