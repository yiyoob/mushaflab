//! SVG backend. Numbers are written from integers (26.6 and 16.16 have exact decimal
//! expansions), so the same `DrawPage` always gives the same bytes.

use std::fmt::Write;

use crate::{DrawItem, DrawPage, PathOp, Role};

fn fmt_fixed(v: i64, bits: u32) -> String {
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
        // frac / 2^bits has at most `bits` decimal digits: frac * 10^bits / 2^bits is exact
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

/// A 26.6 value as an exact decimal.
pub fn fmt_26_6(v: i64) -> String {
    fmt_fixed(v, 6)
}

/// A 16.16 value as an exact decimal.
pub fn fmt_16_16(v: i64) -> String {
    fmt_fixed(v, 16)
}

fn path_d(path: &[PathOp]) -> String {
    let f = |v: i32| fmt_26_6(i64::from(v));
    let mut d = String::new();
    for op in path {
        if !d.is_empty() {
            d.push(' ');
        }
        match *op {
            PathOp::Move { x, y } => write!(d, "M{} {}", f(x), f(y)),
            PathOp::Line { x, y } => write!(d, "L{} {}", f(x), f(y)),
            PathOp::Cubic {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                write!(
                    d,
                    "C{} {} {} {} {} {}",
                    f(x1),
                    f(y1),
                    f(x2),
                    f(y2),
                    f(x),
                    f(y)
                )
            }
            PathOp::Close => write!(d, "Z"),
        }
        .expect("writing to a String");
    }
    d
}

pub fn to_svg(page: &DrawPage) -> String {
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
        writeln!(s, "<path id=\"g{gid}\" d=\"{}\"/>", path_d(path)).expect("string");
    }
    s.push_str("</defs>\n<g fill=\"#000000\">\n");
    for item in &page.items {
        if let DrawItem::Glyph {
            glyph,
            x,
            y,
            sx,
            sy,
            ..
        } = item
        {
            writeln!(
                s,
                "<use xlink:href=\"#g{glyph}\" transform=\"matrix({} 0 0 -{} {} {})\"/>",
                fmt_16_16(*sx),
                fmt_16_16(*sy),
                fmt_26_6(*x),
                fmt_26_6(*y)
            )
            .expect("string");
        }
    }
    s.push_str("</g>\n");
    for item in &page.items {
        if let DrawItem::Stroke { role, width, path } = item {
            let color = match role {
                Role::Frame => "#000000",
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
