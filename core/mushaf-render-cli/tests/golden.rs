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
                        mushaf_draw::PathOp::Quad { x1, x, .. } => {
                            vec![i64::from(x1), i64::from(x)]
                        }
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

/// T3.7: a synthetic free-mode edit (a test fixture, not Quranic data or a reading). The small
/// high seen (U+06DC) goes on the first cluster, from page 3 on, whose form with U+06DC added
/// has an encoding row and still shapes cleanly in its word (rule 6; U+06DC on a medial nun,
/// for one, has no anchor in DigitalKhatt Madina: G-044). On that line a copy of another word's
/// second letter is inserted after it, a third word is widened by 0.5 em, and a manual stretch
/// of 0.2 em goes on one letter that can reach it and one that cannot (D16: applied, reported). The line must
/// still close, every word must pass the integrity check, and the page carries the marker.
#[test]
#[ignore = "run by `just golden` (release)"]
fn edit_demo_line_still_closes_and_page_is_marked() {
    use mushaf_model::{EditOverlay, LineKind, Patch};
    let r = Renderer::load(&repo(), "hafs").unwrap();
    let spine = &r.data.spines[&r.profile.spine];
    let layout = &r.data.layouts[&r.profile.layout_id];
    let seen = '\u{06DC}';
    let shaper0 = r.font.shaper();
    // does this word still convert and shape cleanly with U+06DC appended to cluster ci?
    let clean = |w: &mushaf_model::Item, ci: usize| -> bool {
        let mut cl: Vec<Vec<char>> = w.clusters.iter().map(|c| c.cps.0.clone()).collect();
        cl[ci].push(seen);
        let refs: Vec<&[char]> = cl.iter().map(Vec::as_slice).collect();
        let Ok(conv) = r.table.convert(&refs) else {
            return false;
        };
        let g = shaper0.shape(&conv.chars, &[]).unwrap();
        check_word(&r.font, &shaper0, &conv.chars, &[], &g)
            .unwrap()
            .is_empty()
    };
    let (page, line, words, wi, ci) = layout
        .lines
        .iter()
        .filter(|l| l.page >= 3)
        .find_map(|l| {
            let LineKind::Text { first, last } = &l.kind else {
                return None;
            };
            let (a, b) = (spine.position(first)?, spine.position(last)?);
            let words: Vec<_> = spine.items[a..=b]
                .iter()
                .filter(|i| i.kind == mushaf_model::ItemKind::Word)
                .collect();
            if words.len() < 3 {
                return None;
            }
            let (wi, ci) = words.iter().enumerate().find_map(|(wi, w)| {
                (0..w.clusters.len())
                    .find(|&ci| !w.clusters[ci].cps.0.contains(&seen) && clean(w, ci))
                    .map(|ci| (wi, ci))
            })?;
            Some((l.page, l.line, words, wi, ci))
        })
        .unwrap();
    let others: Vec<_> = (0..words.len())
        .filter(|&k| k != wi && words[k].clusters.len() >= 2)
        .take(3)
        .collect();
    let (ins_w, wide_w) = (words[others[0]], words[others[1]]);
    // one letter where 0.2 em of stretch is reachable and one where it is not (both from data)
    let upem = i64::from(r.font.units_per_em());
    let request = 200 * upem * 64 / 1000;
    let reach = |want: bool| {
        words
            .iter()
            .enumerate()
            .filter(|(k, _)| ![wi, others[0], others[1]].contains(k))
            .find_map(|(_, w)| {
                let cl: Vec<&[char]> = w.clusters.iter().map(|c| c.cps.0.as_slice()).collect();
                let conv = r.table.convert(&cl).ok()?;
                conv.spans.iter().find_map(|sp| {
                    let ms = mushaf_justify::manual_stretch(
                        &shaper0,
                        &conv.chars,
                        sp.chars.start,
                        request,
                    )
                    .ok()?;
                    (ms.reached == want && sp.clusters.len() == 1)
                        .then(|| w.clusters[sp.clusters.start].id.clone())
                })
            })
    };
    let (stretch_ok, stretch_no) = (reach(true).unwrap(), reach(false).unwrap());
    let yaml = |id: &str, op: &str, target: String, payload: String| -> Patch {
        mushaf_model::from_yaml(&format!(
            "id: '{id}'\nowner: {{level: 'user', id: 'demo'}}\nop: '{op}'\ntarget: '{target}'\npayload: {payload}\nsource: {{kind: 'user', ref: 'T3.7 synthetic edit demo'}}\nstatus: 'user-edit'\n"
        ))
        .unwrap()
    };
    let copied = &ins_w.clusters[1];
    let patches = vec![
        yaml(
            "user.demo.seen",
            "set_marks",
            words[wi].clusters[ci].id.to_string(),
            "{add: ['U+06DC']}".into(),
        ),
        yaml(
            "user.demo.letter",
            "insert_after",
            copied.id.to_string(),
            format!(
                "{{seq: 1, cluster: {{kind: 'letter', cps: '{}'}}}}",
                copied.cps
            ),
        ),
        yaml(
            "user.demo.widen",
            "set_attr",
            wide_w.clusters[0].id.to_string(),
            "{width_em_delta: 500}".into(),
        ),
        yaml(
            "user.demo.stretch",
            "set_attr",
            stretch_ok.to_string(),
            "{stretch: 200}".into(),
        ),
        yaml(
            "user.demo.stretch-unreachable",
            "set_attr",
            stretch_no.to_string(),
            "{stretch: 200}".into(),
        ),
    ];
    let mut overlay = EditOverlay::new(spine.id.clone());
    for p in patches {
        overlay.upsert(p).unwrap();
    }
    let edited = r.render(page, Some(&overlay)).unwrap();
    let plain = r.render(page, None).unwrap();
    assert!(plain.canonical && !edited.canonical);
    let marker = |d: &DrawPage| {
        d.items
            .iter()
            .filter(|i| {
                matches!(
                    i,
                    mushaf_draw::DrawItem::Stroke {
                        role: mushaf_draw::Role::EditedMarker,
                        ..
                    }
                )
            })
            .count()
    };
    assert_eq!((marker(&plain), marker(&edited)), (0, 2));
    let (a, b) = (
        plain.lines.iter().find(|l| l.line == line).unwrap(),
        edited.lines.iter().find(|l| l.line == line).unwrap(),
    );
    assert!(b.natural > a.natural, "the edits widen the line");
    for (l, drawn, want) in line_fit(&edited) {
        assert!(
            (drawn - want).abs() * 1000 <= want,
            "edited page line {l}: {drawn} vs {want}"
        );
    }
    // rule 6 holds for the edited words too
    let resolved = r.resolve(Some(&overlay)).unwrap();
    let mapping = r.map(&resolved).unwrap();
    let fit = r.fit(&resolved, &mapping, page).unwrap();
    let shaper = r.font.shaper();
    for w in fit.lines.iter().flat_map(|l| &l.words) {
        let v = check_word(&r.font, &shaper, &w.chars, &w.features, &w.glyphs).unwrap();
        let at = w.item.map(|i| resolved.items[i].id.to_string());
        let cps: Vec<String> = w
            .chars
            .iter()
            .map(|c| format!("{:04X}", u32::from(*c)))
            .collect();
        assert_eq!(v, vec![], "{at:?} {cps:?}");
    }
    // a reachable manual stretch is applied to its letter; an unreachable one is reported
    let unreachable: Vec<&String> = fit
        .notes
        .iter()
        .filter_map(|n| match n {
            FitNote::StretchUnreachable { cluster, .. } => Some(cluster),
            _ => None,
        })
        .collect();
    assert_eq!(unreachable, vec![&stretch_no.to_string()]);
    let ok_word = fit
        .lines
        .iter()
        .flat_map(|l| &l.words)
        .find(|w| w.spans.iter().any(|sp| sp.clusters.contains(&stretch_ok)))
        .unwrap();
    let at = ok_word
        .spans
        .iter()
        .find(|sp| sp.clusters.contains(&stretch_ok))
        .unwrap()
        .chars
        .start as u32;
    assert!(
        ok_word
            .features
            .iter()
            .any(|f| f.start == at && &f.tag == b"cv01"),
        "manual stretch not applied"
    );
    eprintln!(
        "edit demo: page {page} line {line}; natural {} -> {}, target {}",
        a.natural, b.natural, b.target
    );
    let name = format!("hafs-p{page:03}-edit");
    let overlay_path = repo().join(format!("tests/golden/pages/{name}.overlay.yaml"));
    let text = mushaf_model::to_yaml(&overlay).unwrap();
    if std::env::var_os("MUSHAF_UPDATE_GOLDEN").is_some() {
        fs::write(&overlay_path, &text).unwrap();
    } else {
        assert_eq!(
            fs::read_to_string(&overlay_path).unwrap(),
            text,
            "edit demo overlay changed"
        );
    }
    check_golden(&name, &edited);
}

/// D15 line pitch: no ink of adjacent lines touches on any page (glyph outlines and header
/// frames rasterised without anti-aliasing, `mushaf_render::collide`). The detector must also
/// find a known collision: at a pitch of 1,850 the tail of a stretched letter on page 410 line
/// 14 touches a pause sign on line 15 (checked visually by overlaying the two lines, 2026-10-22).
#[test]
#[ignore = "run by `just golden` (release)"]
fn adjacent_lines_never_collide_at_the_profile_pitch() {
    use mushaf_render::collide::line_pairs;
    let mut r = Renderer::load(&repo(), "hafs").unwrap();
    let pitch = r.profile.geometry.line_pitch;
    let resolved = r.resolve(None).unwrap();
    let mapping = r.map(&resolved).unwrap();
    let mut pairs = Vec::new();
    for page in 1..=604 {
        let d = r.draw(&r.fit(&resolved, &mapping, page).unwrap()).unwrap();
        pairs.extend(line_pairs(&d, pitch));
    }
    let hits: Vec<_> = pairs.iter().filter(|p| p.collides).collect();
    assert!(hits.is_empty(), "lines collide at pitch {pitch}: {hits:?}");
    let tightest = pairs.iter().map(|p| p.tightest_pitch).max().unwrap();
    eprintln!(
        "collisions: {} line pairs clear at pitch {pitch}; tightest safe pitch {tightest}",
        pairs.len()
    );
    r.profile.geometry.line_pitch = 1850;
    let d = r.draw(&r.fit(&resolved, &mapping, 410).unwrap()).unwrap();
    let known = line_pairs(&d, 1850);
    assert!(
        known
            .iter()
            .any(|p| p.upper == 14 && p.lower == 15 && p.collides),
        "the detector misses a known collision"
    );
}

/// Fonts of the riwāyāt that draw KFGQPC's encoding as stored (D2): the KFGQPC Warsh and Qālūn
/// fonts, official v2.1. Their render profiles (T5.2) name the same files.
const KFGQPC_FONTS: [(&str, &str); 2] = [
    ("warsh", "fonts/kfgqpc/uthmanic_warsh_v21.ttf"),
    ("qalun", "fonts/kfgqpc/uthmanic_qaloun_v21.ttf"),
];
const KFGQPC_PENDING: &str = "tests/golden/shape/kfgqpc-rule6-pending.json";

/// T5.3, rule 6 for Warsh and Qālūn: every resolved item of the riwāyah, shaped alone in its
/// KFGQPC font (as stored, identity encoding), has no `.notdef`, no inserted dotted circle, a
/// valid cluster map and every mark attached. The font leaves a few marks unanchored; they are
/// pinned in `KFGQPC_PENDING` until the owner decides on them (G-101), and any other
/// violation fails.
#[test]
#[ignore = "run by `just golden` (release)"]
fn kfgqpc_riwayat_words_pass_rule_6() {
    let ds = mushaf_db::load_dir(&repo().join("data")).unwrap();
    let mut found = Vec::new();
    for (riwayah, font_path) in KFGQPC_FONTS {
        let font = mushaf_shape::FontData::new(fs::read(repo().join(font_path)).unwrap()).unwrap();
        let shaper = font.shaper();
        let sel = mushaf_resolve::Selection {
            riwayah: riwayah.into(),
            ..Default::default()
        };
        let resolved =
            mushaf_resolve::resolve_profile(&ds, &sel, None, mushaf_resolve::Mode::Data).unwrap();
        let table = mushaf_shape::EncodingTable::identity(&ds.profiles[riwayah].spine);
        for it in &resolved.items {
            let cl: Vec<&[char]> = it.clusters.iter().map(|c| c.cps.0.as_slice()).collect();
            let chars = table.convert(&cl).unwrap().chars;
            let glyphs = shaper.shape(&chars, &[]).unwrap();
            for v in check_word(&font, &shaper, &chars, &[], &glyphs).unwrap() {
                found.push(format!("{riwayah} {}: {v:?}", it.id));
            }
        }
    }
    let path = repo().join(KFGQPC_PENDING);
    if std::env::var_os("MUSHAF_UPDATE_GOLDEN").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, serde_json::to_string_pretty(&found).unwrap() + "\n").unwrap();
        return;
    }
    let pending: Vec<String> = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        found, pending,
        "rule-6 findings for the KFGQPC fonts differ from the pinned list awaiting the owner"
    );
    assert!(
        found.iter().all(|f| f.contains("UnattachedMark")),
        "only unattached marks may be pending"
    );
}
