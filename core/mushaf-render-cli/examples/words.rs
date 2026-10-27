//! Review aid: draw single words in a font, one row per word, to a PNG.
//!
//!     cargo run --release --example words -- <font> <out.png> < rows.tsv
//!
//! Each input row is `label<TAB>U+XXXX U+XXXX ...` (the font's own encoding). Labels are
//! not drawn; rows are drawn top to bottom in input order.

use std::io::Read;

use mushaf_draw::{Outlines, PathOp};
use mushaf_model::Cps;
use mushaf_shape::FontData;

/// A row: its characters and per-glyph offsets (index, dx, dy).
type Row = (Vec<char>, Vec<(usize, i64, i64)>);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [font, out] = args.as_slice() else {
        eprintln!("usage: words <font> <out.png> < rows.tsv");
        std::process::exit(2);
    };
    let bytes = std::fs::read(font).unwrap();
    let data = FontData::new(bytes.clone()).unwrap();
    let outlines = Outlines::new(&bytes).unwrap();
    let shaper = data.shaper();
    let upem = i64::from(data.units_per_em());
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    // Optional third column: per-glyph offsets in font units, `index:dx:dy;...` (visual order).
    let rows: Vec<Row> = input
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut f = l.split('\t').skip(1);
            let cps = f.next().unwrap().parse::<Cps>().unwrap().0;
            let offs = f
                .next()
                .unwrap_or("")
                .split(';')
                .filter(|s| !s.is_empty())
                .map(|s| {
                    let v: Vec<i64> = s.split(':').map(|n| n.parse().unwrap()).collect();
                    (v[0] as usize, v[1], v[2])
                })
                .collect();
            (cps, offs)
        })
        .collect();
    let row_h = upem * 2 * 64;
    let width = upem * 6 * 64;
    let mut svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {width} {}'><rect width='100%' height='100%' fill='white'/>",
        row_h * rows.len() as i64
    );
    for (i, (chars, offs)) in rows.iter().enumerate() {
        let glyphs = shaper.shape(chars, &[]).unwrap();
        let total: i64 = glyphs.iter().map(|g| i64::from(g.x_advance)).sum::<i64>() * 64;
        let mut x = (width + total) / 2;
        let base = row_h * i as i64 + row_h * 6 / 10;
        // Glyphs come in visual order (left to right); lay them out from the left edge.
        x -= total;
        for (k, g) in glyphs.iter().enumerate() {
            let (dx, dy) = offs
                .iter()
                .find(|o| o.0 == k)
                .map_or((0, 0), |o| (o.1, o.2));
            if std::env::var_os("WORDS_DUMP").is_some() {
                eprintln!(
                    "{i} {k} glyph={} cl={} adv={}",
                    g.glyph, g.cluster, g.x_advance
                );
            }
            let gx = x + (i64::from(g.x_offset) + dx) * 64;
            let gy = base - (i64::from(g.y_offset) + dy) * 64;
            let mut d = String::new();
            for op in outlines.get(g.glyph).unwrap() {
                match *op {
                    PathOp::Move { x, y } => d += &format!("M{} {} ", x, -y),
                    PathOp::Line { x, y } => d += &format!("L{} {} ", x, -y),
                    PathOp::Quad { x1, y1, x, y } => d += &format!("Q{} {} {} {} ", x1, -y1, x, -y),
                    PathOp::Cubic {
                        x1,
                        y1,
                        x2,
                        y2,
                        x,
                        y,
                    } => d += &format!("C{} {} {} {} {} {} ", x1, -y1, x2, -y2, x, -y),
                    PathOp::Close => d += "Z ",
                }
            }
            svg += &format!("<path transform='translate({gx} {gy})' d='{d}'/>");
            x += i64::from(g.x_advance) * 64;
        }
    }
    svg += "</svg>";
    std::fs::write(out, mushaf_render::png(&svg, 700).unwrap()).unwrap();
}
