//! `just golden` (release; ignored by the debug `core-test` because it fits all 604 pages).
//!
//! T3.5a, rule 6: on every page, every shaped word has no `.notdef`, no inserted dotted circle,
//! every mark placed by an anchor, and a valid cluster map. The 33 lossy words (G-040, owner
//! decision) are the only documented exception and must match the encoding table's list.
//! T3.5: every justified line's drawn width is within 0.1% of its target; squeezed lines are
//! listed exceptions. Golden pages: exact draw-list hash plus PNG within a pixel tolerance.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use mushaf_draw::DrawPage;
use mushaf_layout::fit::{FitKind, FitNote};
use mushaf_render::Renderer;
use mushaf_shape::check_word;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Drawn width (page units) against target x page scale, both 26.6.
fn line_fit(d: &DrawPage) -> Vec<(u32, i64, i64)> {
    d.lines
        .iter()
        .filter(|l| l.kind == FitKind::Text && l.width > 0)
        .map(|l| {
            (
                l.line,
                l.right - l.left,
                l.target * d.scale / mushaf_justify::SCALE_ONE,
            )
        })
        .collect()
}

#[test]
#[ignore = "run by `just golden` (release)"]
fn shaping_integrity_and_line_fit_on_every_page() {
    let r = Renderer::load(&repo(), "hafs").unwrap();
    let resolved = r.resolve(None).unwrap();
    let mapping = r.map(&resolved).unwrap();
    let shaper = r.font.shaper();
    let (mut violations, mut misfit, mut lossy, mut squeezed) =
        (Vec::new(), Vec::new(), BTreeSet::new(), 0);
    let (mut words, mut lines) = (0usize, 0usize);
    for page in 1..=604 {
        let fit = r.fit(&resolved, &mapping, page).unwrap();
        for note in &fit.notes {
            if let FitNote::Lossy { item } = note {
                lossy.insert(item.clone());
            }
        }
        for l in &fit.lines {
            for w in &l.words {
                words += 1;
                let v = check_word(&r.font, &shaper, &w.chars, &w.features, &w.glyphs).unwrap();
                if !v.is_empty() {
                    let at = w
                        .item
                        .map(|i| resolved.items[i].id.to_string())
                        .unwrap_or_else(|| format!("{page}:{} header", l.line));
                    violations.push(format!("{at}: {v:?}"));
                }
            }
        }
        let d = r.draw(&fit).unwrap();
        for (line, drawn, want) in line_fit(&d) {
            lines += 1;
            let squeezed_line = d
                .lines
                .iter()
                .any(|l| l.line == line && l.squeeze.is_some());
            squeezed += usize::from(squeezed_line);
            if (drawn - want).abs() * 1000 > want {
                misfit.push(format!("{page}:{line} drawn {drawn} vs {want}"));
            }
        }
    }
    eprintln!(
        "integrity: {words} words on 604 pages; {lines} text lines, {squeezed} squeezed (D15 exceptions)"
    );
    assert!(
        violations.is_empty(),
        "{} words violate rule 6: {:?}",
        violations.len(),
        &violations[..violations.len().min(20)]
    );
    let expected: BTreeSet<String> = r.table.lossy_items.iter().cloned().collect();
    assert_eq!(
        lossy, expected,
        "lossy words (G-040) differ from the encoding table's list"
    );
    assert!(
        misfit.is_empty(),
        "{} lines outside 0.1%: {:?}",
        misfit.len(),
        &misfit[..misfit.len().min(20)]
    );
}

/// Pages for the owner's GATE G1 review and the regression goldens (T3.5). Chosen from the
/// data: 106 is the first page with a surah header below line 1, 255 also has a short line
/// inside the text, 278 holds 16:101 (G-015), 507 has the strongest squeeze (D15).
const GOLDEN_PAGES: [u32; 9] = [1, 2, 3, 106, 255, 278, 302, 507, 604];
const PNG_WIDTH: u32 = 1000;

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// What a golden pins exactly: the draw list's hash and its readable summary.
fn summary(d: &DrawPage) -> serde_json::Value {
    serde_json::json!({
        "page": d.page,
        "layout_id": d.layout_id,
        "riwayah": d.riwayah,
        "draw_sha256": sha256(serde_json::to_string(d).unwrap().as_bytes()),
        "scale": d.scale,
        "canonical": d.canonical,
        "lines": d.lines,
        "notes": d.notes,
    })
}

/// Pixels whose channels differ by more than `tol` (0-255), and the pixel count.
fn png_diff(a: &[u8], b: &[u8], tol: u8) -> (usize, usize) {
    let pa = resvg::tiny_skia::Pixmap::decode_png(a).unwrap();
    let pb = resvg::tiny_skia::Pixmap::decode_png(b).unwrap();
    assert_eq!(
        (pa.width(), pa.height()),
        (pb.width(), pb.height()),
        "image size changed"
    );
    let n = pa.data().len() / 4;
    let bad = pa
        .data()
        .chunks(4)
        .zip(pb.data().chunks(4))
        .filter(|(x, y)| x.iter().zip(*y).any(|(p, q)| p.abs_diff(*q) > tol))
        .count();
    (bad, n)
}

/// Compare (or, with MUSHAF_UPDATE_GOLDEN=1, write) one golden page.
pub fn check_golden(name: &str, d: &DrawPage) {
    let dir = repo().join("tests/golden/pages");
    let (json_path, png_path) = (
        dir.join(format!("{name}.json")),
        dir.join(format!("{name}.png")),
    );
    let json = serde_json::to_string_pretty(&summary(d)).unwrap() + "\n";
    let png = mushaf_render::png(&mushaf_render::svg(d), PNG_WIDTH).unwrap();
    if std::env::var_os("MUSHAF_UPDATE_GOLDEN").is_some() {
        fs::create_dir_all(&dir).unwrap();
        fs::write(&json_path, &json).unwrap();
        fs::write(&png_path, &png).unwrap();
        return;
    }
    let want = fs::read_to_string(&json_path).unwrap_or_else(|_| panic!("missing {json_path:?}"));
    assert_eq!(json, want, "{name}: draw list or line fit changed");
    let (bad, n) = png_diff(&png, &fs::read(&png_path).unwrap(), 32);
    assert!(bad * 1000 <= n, "{name}: {bad} of {n} pixels differ");
}

#[test]
#[ignore = "run by `just golden` (release)"]
fn golden_pages() {
    let r = Renderer::load(&repo(), "hafs").unwrap();
    for page in GOLDEN_PAGES {
        let d = r.render(page, None).unwrap();
        assert!(d.canonical);
        check_golden(&format!("hafs-p{page:03}"), &d);
    }
}

/// G-015 (owner print check, page 278, 16:101, 2026-10-21): the pause sign that follows a
/// small low meem sits over the end of its word, over the final letter. In KFGQPC fonts it had
/// no anchor. Here every such place in the spine (a pause sign U+06D6..U+06DC after U+06ED,
/// U+06E2 or U+0655 in one cluster) is found from the data, and its pause glyph must be
/// attached and drawn within the horizontal extent of that cluster's base glyph.
#[test]
#[ignore = "run by `just golden` (release)"]
fn g015_pause_signs_after_small_meem_sit_over_their_letter() {
    let r = Renderer::load(&repo(), "hafs").unwrap();
    let spine = &r.data.spines[&r.profile.spine];
    let shaper = r.font.shaper();
    let pause = |c: char| ('\u{06D6}'..='\u{06DC}').contains(&c);
    let carrier = |c: char| matches!(c, '\u{06ED}' | '\u{06E2}' | '\u{0655}');
    let mut places = Vec::new();
    for item in &spine.items {
        for (ci, c) in item.clusters.iter().enumerate() {
            let cps = &c.cps.0;
            let after_carrier = cps
                .iter()
                .position(|&x| carrier(x))
                .is_some_and(|k| cps[k..].iter().any(|&x| pause(x)));
            if after_carrier {
                places.push((item, ci));
            }
        }
    }
    assert!(
        places
            .iter()
            .any(|(i, _)| i.id.surah == 16 && i.id.ayah == 101),
        "16:101 is among them"
    );
    for (item, ci) in &places {
        let cl: Vec<&[char]> = item.clusters.iter().map(|c| c.cps.0.as_slice()).collect();
        let conv = r.table.convert(&cl).unwrap();
        let span = conv.spans.iter().find(|s| s.clusters.contains(ci)).unwrap();
        let glyphs = shaper.shape(&conv.chars, &[]).unwrap();
        assert_eq!(
            check_word(&r.font, &shaper, &conv.chars, &[], &glyphs).unwrap(),
            vec![],
            "{}",
            item.id
        );
        // absolute x of each glyph (visual order, font units) and its outline's x extent
        let mut pen = 0i64;
        let mut boxes = Vec::new();
        for g in &glyphs {
            let pts: Vec<i64> =
                r.outlines
                    .get(g.glyph)
                    .unwrap()
                    .iter()
                    .flat_map(|op| match *op {
                        mushaf_draw::PathOp::Move { x, .. }
                        | mushaf_draw::PathOp::Line { x, .. } => vec![i64::from(x)],
                        mushaf_draw::PathOp::Cubic { x1, x2, x, .. } => {
                            vec![i64::from(x1), i64::from(x2), i64::from(x)]
                        }
                        mushaf_draw::PathOp::Close => vec![],
                    })
                    .collect();
            let x0 = pen * 64 + i64::from(g.x_offset) * 64;
            boxes.push((
                g,
                x0 + pts.iter().min().copied().unwrap_or(0),
                x0 + pts.iter().max().copied().unwrap_or(0),
            ));
            pen += i64::from(g.x_advance);
        }
        // marks keep their own cluster (monotone characters): the pause glyph's cluster is the
        // pause sign's index, the letter's is the span's first character
        let pause_at = span
            .chars
            .clone()
            .rev()
            .find(|&k| pause(conv.chars[k]))
            .unwrap();
        let letter = boxes
            .iter()
            .find(|(g, ..)| {
                g.cluster as usize == span.chars.start && r.font.glyph_class(g.glyph) != 3
            })
            .unwrap();
        let sign = boxes
            .iter()
            .find(|(g, ..)| g.cluster as usize == pause_at)
            .unwrap();
        assert_eq!(
            r.font.glyph_class(sign.0.glyph),
            3,
            "{}: the pause sign is not a mark glyph",
            item.id
        );
        let centre = (sign.1 + sign.2) / 2;
        assert!(
            (letter.1..=letter.2).contains(&centre),
            "{}: pause sign centre {} outside its letter {}..{}",
            item.id,
            centre / 64,
            letter.1 / 64,
            letter.2 / 64
        );
    }
    eprintln!("G-015: {} places checked", places.len());
}
