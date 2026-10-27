//! SVG backend. Numbers are written from integers (26.6 and 16.16 have exact decimal
//! expansions), so the same `DrawPage` always gives the same bytes.

use std::fmt::Write;

use mushaf_model::ClusterRef;

use crate::{DrawItem, DrawPage, PathOp, Role};

/// Appends `v / 2^bits` as an exact decimal: no trailing zeros, no `.` for an integer.
fn push_fixed(s: &mut String, v: i64, bits: u32) {
    let a = v.unsigned_abs();
    if v < 0 {
        s.push('-');
    }
    write!(s, "{}", a >> bits).expect("string");
    let frac = a & ((1 << bits) - 1);
    if frac != 0 {
        // frac / 2^bits has at most `bits` decimal digits: frac * 10^bits / 2^bits is exact
        let mut digits = (u128::from(frac) * 10u128.pow(bits)) >> bits;
        let mut buf = [b'0'; 16];
        let mut n = bits as usize;
        while digits.is_multiple_of(10) {
            digits /= 10;
            n -= 1;
        }
        for i in (0..n).rev() {
            buf[i] = b'0' + (digits % 10) as u8;
            digits /= 10;
        }
        s.push('.');
        s.push_str(std::str::from_utf8(&buf[..n]).expect("ascii"));
    }
}

fn fmt_fixed(v: i64, bits: u32) -> String {
    let mut s = String::new();
    push_fixed(&mut s, v, bits);
    s
}

/// A 26.6 value as an exact decimal.
pub fn fmt_26_6(v: i64) -> String {
    fmt_fixed(v, 6)
}

/// A 16.16 value as an exact decimal.
pub fn fmt_16_16(v: i64) -> String {
    fmt_fixed(v, 16)
}

/// A path's `d` attribute (26.6 font or page units, exact decimals).
pub fn path_d(path: &[PathOp]) -> String {
    let mut d = String::new();
    push_path_d(&mut d, path);
    d
}

fn push_path_d(d: &mut String, path: &[PathOp]) {
    let start = d.len();
    let nums = |d: &mut String, cmd: char, vs: &[i32]| {
        if d.len() > start {
            d.push(' ');
        }
        d.push(cmd);
        for (k, v) in vs.iter().enumerate() {
            if k > 0 {
                d.push(' ');
            }
            push_fixed(d, i64::from(*v), 6);
        }
    };
    for op in path {
        match *op {
            PathOp::Move { x, y } => nums(d, 'M', &[x, y]),
            PathOp::Line { x, y } => nums(d, 'L', &[x, y]),
            PathOp::Quad { x1, y1, x, y } => nums(d, 'Q', &[x1, y1, x, y]),
            PathOp::Cubic {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => nums(d, 'C', &[x1, y1, x2, y2, x, y]),
            PathOp::Close => nums(d, 'Z', &[]),
        }
    }
}

pub fn to_svg(page: &DrawPage) -> String {
    svg(page, false)
}

/// `to_svg` for the apps (D59): the same drawing, with each glyph's cluster and line as
/// `data-c` and `data-l` attributes so a tap can name the cluster. Nothing else differs.
pub fn to_svg_interactive(page: &DrawPage) -> String {
    svg(page, true)
}

fn svg(page: &DrawPage, interactive: bool) -> String {
    let mut s = String::new();
    let (w, h) = (fmt_26_6(page.width), fmt_26_6(page.height));
    writeln!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
         viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">"
    )
    .expect("string");
    writeln!(
        s,
        "<desc>{} {} page {} ({}, {}){}</desc>",
        page.layout_id,
        page.riwayah,
        page.page,
        page.script_id,
        page.font,
        if page.canonical { "" } else { " EDITED" }
    )
    .expect("string");
    writeln!(s, "<rect width=\"{w}\" height=\"{h}\" fill=\"#ffffff\"/>").expect("string");
    s.push_str("<defs>\n");
    for (gid, path) in &page.glyphs {
        write!(s, "<path id=\"g{gid}\" d=\"").expect("string");
        push_path_d(&mut s, path);
        s.push_str("\"/>\n");
    }
    s.push_str("</defs>\n<g fill=\"#000000\">\n");
    // Highlighted clusters are drawn in red, as in the owner's print (D45, 2026-10-26).
    let marked: std::collections::BTreeSet<(u32, &ClusterRef)> = page
        .highlights
        .iter()
        .map(|h| (h.line, &h.cluster))
        .collect();
    for item in &page.items {
        if let DrawItem::Glyph {
            glyph,
            x,
            y,
            sx,
            sy,
            line,
            cluster,
        } = item
        {
            write!(s, "<use xlink:href=\"#g{glyph}\"").expect("string");
            if let (Some(c), true) = (cluster, interactive) {
                write!(s, " data-c=\"{c}\" data-l=\"{line}\"").expect("string");
            }
            if cluster
                .as_ref()
                .is_some_and(|c| marked.contains(&(*line, c)))
            {
                s.push_str(" fill=\"#cc0000\"");
            }
            s.push_str(" transform=\"matrix(");
            push_fixed(&mut s, *sx, 16);
            s.push_str(" 0 0 -");
            push_fixed(&mut s, *sy, 16);
            s.push(' ');
            push_fixed(&mut s, *x, 6);
            s.push(' ');
            push_fixed(&mut s, *y, 6);
            s.push_str(")\"/>\n");
        }
    }
    for item in &page.items {
        if let DrawItem::Fill { path, .. } = item {
            writeln!(s, "<path d=\"{}\"/>", path_d(path)).expect("string");
        }
    }
    s.push_str("</g>\n");
    for item in &page.items {
        if let DrawItem::Stroke { role, width, path } = item {
            let color = match role {
                Role::Frame | Role::SajdahOverline => "#000000",
                Role::EditedMarker => "#cc0000",
            };
            writeln!(
                s,
                "<path d=\"{}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{}\" \
                 stroke-linejoin=\"round\" stroke-linecap=\"round\"/>",
                path_d(path),
                fmt_26_6(*width)
            )
            .expect("string");
        }
    }
    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The formatter before T8.4 (allocating), kept as the reference for `push_fixed`.
    fn reference(v: i64, bits: u32) -> String {
        let neg = v < 0;
        let a = v.unsigned_abs();
        let int = a >> bits;
        let frac = a & ((1 << bits) - 1);
        let mut s = if neg {
            format!("-{int}")
        } else {
            int.to_string()
        };
        if frac != 0 {
            let digits = (u128::from(frac) * 10u128.pow(bits)) >> bits;
            let mut d = format!("{digits:0width$}", width = bits as usize);
            while d.ends_with('0') {
                d.pop();
            }
            s.push('.');
            s.push_str(&d);
        }
        s
    }

    #[test]
    fn fast_formatter_writes_the_same_bytes() {
        let mut vals: Vec<i64> = (-70_000..70_000).collect();
        vals.extend([
            i64::from(i32::MAX),
            i64::from(i32::MIN),
            1 << 40,
            -(1 << 40) - 7,
        ]);
        vals.extend((0..20_000).map(|k| k * 7919 + 65_535));
        for v in vals {
            for bits in [6, 16] {
                assert_eq!(fmt_fixed(v, bits), reference(v, bits), "{v} / 2^{bits}");
            }
        }
    }

    #[test]
    fn fixed_point_decimals_are_exact() {
        assert_eq!(fmt_26_6(64), "1");
        assert_eq!(fmt_26_6(1), "0.015625");
        assert_eq!(fmt_26_6(-96), "-1.5");
        assert_eq!(fmt_16_16(1), "0.0000152587890625");
        assert_eq!(fmt_16_16(65536 * 9 / 10), "0.899993896484375");
        assert_eq!(fmt_16_16(-65536), "-1");
    }
}
