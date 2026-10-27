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
    let exceptions = mushaf_shape::Exceptions::from_json(
        &fs::read_to_string(repo().join("data/render/kfgqpc-rule6-exceptions.json")).unwrap(),
    )
    .unwrap();
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
                if !exceptions.allows(font_path, &v) {
                    found.push(format!("{riwayah} {}: {v:?}", it.id));
                }
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

/// Riwāyāt drawn with DigitalKhatt Madina besides Ḥafṣ (D2).
const DK_RIWAYAT: [&str; 5] = ["shuba", "bazzi", "qunbul", "duri_abuamr", "susi"];
const DK_PENDING: &str = "tests/golden/shape/dk-riwayat-rule6-pending.json";

/// T5.3, rule 6 for Shuʿbah, Bazzi, Qunbul, al-Dūrī and al-Sūsī: every resolved item converts
/// with the Ḥafṣ table plus the owner-approved rows (`hafs-v1.riwayat-rows.json`, D28) and,
/// shaped in DigitalKhatt Madina, passes every check. Two kinds of item are pinned in
/// `DK_PENDING` and listed as blocked: a cluster form whose group the owner has not decided yet
/// cannot convert (G-102), and a few words leave a mark unattached in a context Ḥafṣ never has
/// (G-103). Any other failure fails.
#[test]
#[ignore = "run by `just golden` (release)"]
fn dk_riwayat_words_pass_rule_6() {
    let ds = mushaf_db::load_dir(&repo().join("data")).unwrap();
    let r = Renderer::load(&repo(), "hafs").unwrap();
    let mut table = r.table.clone();
    table
        .extend_from_json(
            &fs::read_to_string(repo().join("data/encodings/hafs-v1.riwayat-rows.json")).unwrap(),
        )
        .unwrap();
    let shaper = r.font.shaper();
    let mut found = Vec::new();
    for riwayah in DK_RIWAYAT {
        let sel = mushaf_resolve::Selection {
            riwayah: riwayah.into(),
            ..Default::default()
        };
        let resolved =
            mushaf_resolve::resolve_profile(&ds, &sel, None, mushaf_resolve::Mode::Data).unwrap();
        for it in &resolved.items {
            let cl: Vec<&[char]> = it.clusters.iter().map(|c| c.cps.0.as_slice()).collect();
            let chars = match table.convert_item(&it.id.to_string(), &cl) {
                Ok(c) => c.chars,
                Err(u) => {
                    found.push(format!("{riwayah} {}: no row for {}", it.id, u.cps));
                    continue;
                }
            };
            let glyphs = shaper.shape(&chars, &[]).unwrap();
            for v in check_word(&r.font, &shaper, &chars, &[], &glyphs).unwrap() {
                found.push(format!("{riwayah} {}: {v:?}", it.id));
            }
        }
    }
    let path = repo().join(DK_PENDING);
    if std::env::var_os("MUSHAF_UPDATE_GOLDEN").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, serde_json::to_string_pretty(&found).unwrap() + "\n").unwrap();
        return;
    }
    let pending: Vec<String> = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        found, pending,
        "DigitalKhatt riwayat: rule-6 findings differ from the pinned list awaiting the owner"
    );
    assert!(
        found
            .iter()
            .all(|f| f.contains("no row for") || f.contains("UnattachedMark")),
        "only unconvertible forms and unattached marks may be pending"
    );
}

/// The seven riwāyāt on their native layouts (T5.2).
const NATIVE: [&str; 7] = [
    "shuba",
    "warsh",
    "qalun",
    "bazzi",
    "qunbul",
    "duri_abuamr",
    "susi",
];
/// DigitalKhatt Madina's spacing upright alef that `rclt` puts after a non-joining letter
/// (G-045): `glyph00103`.
const G045_GLYPH: u32 = 103;

/// Pinned rule-6 findings for one riwāyah: (item, is "no row") from both pending lists.
fn pinned(riwayah: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for f in [KFGQPC_PENDING, DK_PENDING] {
        let list: Vec<String> =
            serde_json::from_str(&fs::read_to_string(repo().join(f)).unwrap()).unwrap();
        for e in list {
            let (r, rest) = e.split_once(' ').unwrap();
            if r == riwayah {
                let (item, what) = rest.split_once(": ").unwrap();
                out.push((item.to_string(), what.starts_with("no row for")));
            }
        }
    }
    out
}

fn pct(v: &[i64], p: usize) -> i64 {
    v[(v.len() - 1) * p / 100]
}

/// T5.2 (D29, D31), every page of every riwāyah on its native layout: a page whose words cannot
/// all be converted yet is blocked, exactly the pages holding a G-102 form; every other page
/// passes rule 6 except the pinned words (G-101, G-103), its text lines close within 0.1%, and
/// no ink of adjacent lines touches at the profile's pitch. The overflow report per riwāyah
/// (`docs/review/phase5/overflow-<r>.md`) must equal what the pages give.
#[test]
#[ignore = "run by `just golden` (release)"]
fn native_layouts_pass_rule_6_close_and_never_collide() {
    use mushaf_render::collide::line_pairs;
    use mushaf_render::fit_checked;
    for riwayah in NATIVE {
        let r = Renderer::load(&repo(), riwayah).unwrap();
        let resolved = r.resolve(None).unwrap();
        let mapping = r.map(&resolved).unwrap();
        let shaper = r.font.shaper();
        let space = mushaf_justify::space_width(&shaper).unwrap();
        let mut page_of = std::collections::BTreeMap::new();
        for l in &mapping.lines {
            for it in &resolved.items[l.items.clone()] {
                page_of.insert(it.id.to_string(), l.page);
            }
        }
        let pins = pinned(riwayah);
        let want_blocked: BTreeSet<u32> = pins
            .iter()
            .filter(|(_, no_row)| *no_row)
            .map(|(i, _)| page_of[i])
            .collect();
        let want_flagged: BTreeSet<String> = pins
            .iter()
            .filter(|(i, no_row)| !no_row && !want_blocked.contains(&page_of[i]))
            .map(|(i, _)| i.clone())
            .collect();
        let (mut blocked, mut flagged) = (BTreeSet::new(), BTreeSet::new());
        let (mut misfit, mut hits) = (Vec::new(), Vec::new());
        let (mut scaled, mut floor, mut squeezed) = (Vec::new(), Vec::new(), Vec::new());
        let (mut widest, mut g045, mut lines) = (Vec::new(), 0usize, 0usize);
        let mut flag_pages = std::collections::BTreeMap::<u32, Vec<String>>::new();
        for page in 1..=604 {
            let cp = fit_checked(&r, &shaper, &resolved, &mapping, page).unwrap();
            let Some(fit) = cp.fit else {
                blocked.insert(page);
                continue;
            };
            for (item, v) in &cp.violations {
                assert!(
                    matches!(v, mushaf_shape::Violation::UnattachedMark { .. }),
                    "{riwayah} {item}: {v:?}"
                );
                flagged.insert(item.clone());
                flag_pages.entry(page).or_default().push(item.clone());
            }
            for n in &fit.notes {
                match n {
                    FitNote::PageScaled { scale } => scaled.push((page, *scale)),
                    FitNote::PageScaleAtFloor { .. } => floor.push(page),
                    FitNote::Squeezed { line, x_scale } => squeezed.push((page, *line, *x_scale)),
                    _ => {}
                }
            }
            for l in &fit.lines {
                if l.kind == FitKind::Text && l.width > 0 {
                    lines += 1;
                    if let Some(w) = l.spaces.iter().max() {
                        widest.push(w * 1000 / space);
                    }
                }
                for w in &l.words {
                    g045 += w.glyphs.iter().filter(|g| g.glyph == G045_GLYPH).count();
                }
            }
            let d = r.draw(&fit).unwrap();
            for (line, drawn, want) in line_fit(&d) {
                if (drawn - want).abs() * 1000 > want {
                    misfit.push(format!("{page}:{line} drawn {drawn} vs {want}"));
                }
            }
            hits.extend(
                line_pairs(&d, r.profile.geometry.line_pitch)
                    .into_iter()
                    .filter(|p| p.collides)
                    .map(|p| (page, p.upper, p.lower)),
            );
        }
        assert_eq!(blocked, want_blocked, "{riwayah}: blocked pages (G-102)");
        assert_eq!(
            flagged, want_flagged,
            "{riwayah}: rule-6 findings (G-101, G-103)"
        );
        assert!(
            misfit.is_empty(),
            "{riwayah}: lines outside 0.1%: {misfit:?}"
        );
        assert!(hits.is_empty(), "{riwayah}: colliding lines: {hits:?}");
        widest.sort_unstable();
        let mut md = format!(
            "# Overflow report: {riwayah} (T5.2, D31)\n\nGenerated by `just golden` \
             (`MUSHAF_UPDATE_GOLDEN=1` rewrites it) from every page of `{}` drawn with `{}`. \
             Overflow policy (D15, extended per riwayah): one font scale per page (floor {}), \
             then simple spaces tighten to {}%, then a horizontal squeeze; each step is \
             listed. Justification: {}.\n\n",
            r.profile.layout_id,
            r.profile.font,
            fmt_milli(r.profile.geometry.page_scale_floor),
            r.profile.justify.min_space / 10,
            if r.profile.justify.letters {
                "letter stretching (DigitalKhatt features), then spaces"
            } else {
                "spaces only (D27)"
            }
        );
        md += &format!(
            "| | |\n|---|---|\n| Text lines drawn | {lines} |\n| Pages blocked (G-102) | {} |\n\
             | Pages scaled below 1 | {} |\n| Pages at the scale floor | {} |\n| Squeezed lines | {} |\n\
             | Widest space per line, median / p95 / max (x normal) | {} / {} / {} |\n\
             | Spacing upright alef after a non-joining letter (G-045) | {} |\n\n",
            blocked.len(),
            scaled.len(),
            floor.len(),
            squeezed.len(),
            fmt_milli(pct(&widest, 50)),
            fmt_milli(pct(&widest, 95)),
            fmt_milli(pct(&widest, 100)),
            if r.profile.justify.letters {
                g045.to_string()
            } else {
                "n/a (KFGQPC font)".into()
            }
        );
        md += "## Blocked pages (G-102: a word has no DigitalKhatt row yet)\n\n";
        md += &list_or_none(blocked.iter().map(u32::to_string));
        md += "\n## Pages with pinned rule-6 findings (G-101, G-103)\n\n";
        md += &list_or_none(
            flag_pages
                .iter()
                .map(|(p, items)| format!("{p} ({})", items.join(", "))),
        );
        md += "\n## Pages at the scale floor\n\n";
        md += &list_or_none(floor.iter().map(u32::to_string));
        md += "\n## Squeezed lines\n\n";
        md += &list_or_none(
            squeezed
                .iter()
                .map(|(p, l, x)| format!("{p}:{l} ({})", fmt_ratio(*x))),
        );
        let path = repo().join(format!("docs/review/phase5/overflow-{riwayah}.md"));
        if std::env::var_os("MUSHAF_UPDATE_GOLDEN").is_some() {
            fs::write(&path, &md).unwrap();
        } else {
            let want = fs::read_to_string(&path).unwrap_or_default();
            assert_eq!(md, want, "{riwayah}: overflow report differs from {path:?}");
        }
        eprintln!(
            "{riwayah}: {lines} lines, {} blocked pages, {} squeezed, G-045 {g045}",
            blocked.len(),
            squeezed.len()
        );
    }
}

/// 16.16 as a decimal with three places.
fn fmt_ratio(v: i64) -> String {
    let m = v * 1000 / mushaf_justify::SCALE_ONE;
    format!("{}.{:03}", m / 1000, m % 1000)
}

fn fmt_milli(v: i64) -> String {
    format!("{}.{:02}", v / 1000, (v % 1000) / 10)
}

fn list_or_none(items: impl Iterator<Item = String>) -> String {
    let v: Vec<String> = items.collect();
    if v.is_empty() {
        "None.\n".into()
    } else {
        v.join(", ") + "\n"
    }
}

/// T5.4: where each riwāyah's golden pages come from. Cases named by the plan (3:133, 2:10,
/// 2:184, the numbering of al-Fātiḥah) are located through the alignment with the Ḥafṣ spine: the
/// page holding the first item of the riwāyah's text at or after that Ḥafṣ ayah's first word.
/// What these pages show about the readings is a **hypothesis** (G-074): the goldens pin what the
/// KFGQPC data render, never a claim about the qirāʾah.
const NATIVE_CASES: [(&str, (u32, u32)); 4] = [
    ("fatiha", (1, 1)),
    ("2:10", (2, 10)),
    ("2:184", (2, 184)),
    ("3:133", (3, 133)),
];
/// A page-crossing ayah that every non-Ḥafṣ riwāyah has (4:44, ingest report T4.1): the page it
/// starts on.
const CROSSING: (u32, u32) = (4, 44);
const NATIVE_PAGES: &str = "tests/golden/pages/native-pages.json";

fn golden_page_choice(
    r: &Renderer,
    resolved: &mushaf_resolve::Resolved,
    mapping: &mushaf_layout::Mapping,
) -> Vec<(String, u32)> {
    let spine = &r.data.spines[&r.profile.spine];
    let page_of_spine_ayah = |(s, a): (u32, u32)| -> u32 {
        let first = mushaf_model::ItemRef::spine(s, a, 1);
        let pos = spine.position(&first).unwrap();
        let k = resolved
            .items
            .iter()
            .position(|it| {
                let (mushaf_resolve::Origin::Spine { pos: p }
                | mushaf_resolve::Origin::Inserted { anchor: p }) = it.origin;
                p >= pos
            })
            .unwrap();
        mapping
            .lines
            .iter()
            .find(|l| l.items.contains(&k))
            .unwrap()
            .page
    };
    let mut out: Vec<(String, u32)> = NATIVE_CASES
        .iter()
        .map(|(name, key)| (name.to_string(), page_of_spine_ayah(*key)))
        .collect();
    out.push(("crossing 4:44".into(), page_of_spine_ayah(CROSSING)));
    out
}

/// T5.4: golden pages per riwāyah. The page list itself is pinned in `NATIVE_PAGES` (so a change
/// in layout or alignment shows), and every page in it is pinned like Ḥafṣ's goldens: the draw
/// list's hash and its line fit, plus the PNG within a pixel tolerance. A page that cannot be
/// drawn yet (G-102) is pinned as blocked.
#[test]
#[ignore = "run by `just golden` (release)"]
fn native_golden_pages() {
    use mushaf_render::fit_checked;
    let update = std::env::var_os("MUSHAF_UPDATE_GOLDEN").is_some();
    let mut chosen = serde_json::Map::new();
    for riwayah in NATIVE {
        let r = Renderer::load(&repo(), riwayah).unwrap();
        let resolved = r.resolve(None).unwrap();
        let mapping = r.map(&resolved).unwrap();
        let shaper = r.font.shaper();
        let pages = golden_page_choice(&r, &resolved, &mapping);
        chosen.insert(riwayah.into(), serde_json::json!(pages));
        let mut done = BTreeSet::new();
        for (_, page) in pages {
            if !done.insert(page) {
                continue;
            }
            let name = format!("{riwayah}-p{page:03}");
            let cp = fit_checked(&r, &shaper, &resolved, &mapping, page).unwrap();
            match cp.fit {
                Some(fit) => check_golden(&name, &r.draw(&fit).unwrap()),
                None => {
                    let json = serde_json::to_string_pretty(&serde_json::json!({
                        "page": page,
                        "riwayah": riwayah,
                        "blocked": cp.blocked,
                    }))
                    .unwrap()
                        + "\n";
                    let path = repo().join(format!("tests/golden/pages/{name}.json"));
                    if update {
                        fs::write(&path, json).unwrap();
                    } else {
                        assert_eq!(json, fs::read_to_string(&path).unwrap(), "{name}");
                    }
                }
            }
        }
    }
    let text = serde_json::to_string_pretty(&chosen).unwrap() + "\n";
    let path = repo().join(NATIVE_PAGES);
    if update {
        fs::write(&path, text).unwrap();
    } else {
        assert_eq!(
            text,
            fs::read_to_string(&path).unwrap(),
            "golden page list changed"
        );
    }
}

/// T6.2/T6.3: the Ḥafṣ option `hafs.maliyah` (KFGQPC Ḥafṣ muṣḥaf, explanation of signs, p. ح).
/// By the owner's decision (D41) the idghām wajh changes no glyph: it only writes the
/// annotation channel (D40). The page holding 69:28 is pinned under the default (iẓhār with
/// sakt), passes rule 6 and line fit, and the idghām draw list must be byte-identical to it.
/// The page is found from the layout.
#[test]
#[ignore = "run by `just golden` (release)"]
fn hafs_maliyah_option_pair() {
    use mushaf_model::LineKind;
    use mushaf_render::fit_checked;
    use std::collections::BTreeMap;
    let r = Renderer::load(&repo(), "hafs").unwrap();
    let spine = &r.data.spines[&r.profile.spine];
    let layout = &r.data.layouts[&r.profile.layout_id];
    let p = spine.position(&"69:28:4".parse().unwrap()).unwrap();
    let page = layout
        .lines
        .iter()
        .find(|l| match &l.kind {
            LineKind::Text { first, last } => {
                spine.position(first).unwrap() <= p && p <= spine.position(last).unwrap()
            }
            _ => false,
        })
        .unwrap()
        .page;
    let shaper = r.font.shaper();
    let mut drawn = Vec::new();
    for choice in ["izhar_sakt", "idgham"] {
        let choices = BTreeMap::from([("hafs.maliyah".to_string(), choice.to_string())]);
        let resolved = r.resolve_with(&choices, None).unwrap();
        assert_eq!(resolved.choices["hafs.maliyah"], choice);
        let mapping = r.map(&resolved).unwrap();
        let cp = fit_checked(&r, &shaper, &resolved, &mapping, page).unwrap();
        assert!(cp.blocked.is_none(), "{choice}: {:?}", cp.blocked);
        assert!(
            cp.violations.is_empty(),
            "{choice}: rule 6: {:?}",
            cp.violations
        );
        let d = r.draw(cp.fit.as_ref().unwrap()).unwrap();
        assert!(d.canonical, "{choice}: an option choice is not a free edit");
        drawn.push(serde_json::to_string(&d).unwrap());
        if choice == "izhar_sakt" {
            check_golden(&format!("hafs-maliyah-p{page:03}"), &d);
        }
    }
    assert_eq!(
        drawn[0], drawn[1],
        "the idgham wajh must draw exactly as the printed text"
    );
}

/// T6.9 (D44): coverage of every riwāyah in the style it does not use by default. Maghribi:
/// the KFGQPC Warsh font, KFGQPC codepoints after the owner-confirmed style rows. Ḥijāzī:
/// DigitalKhatt Madina through the Ḥafṣ table and the approved riwāyah rows (D28), after the
/// style rows (there are none from Maghribi). Every resolved word is converted, shaped and
/// checked against rule 6. A combination is available only if no word fails. The result is
/// pinned in `STYLE_AVAILABILITY` (rewritten with MUSHAF_UPDATE_GOLDEN=1).
const STYLE_AVAILABILITY: &str = "data/render/style-availability.json";
const STYLE_FONTS: [(&str, &str); 2] = [
    ("maghribi", "fonts/kfgqpc/uthmanic_warsh_v21.ttf"),
    ("hijazi", "fonts/digitalkhatt/hb-madina.otf"),
];
const ALL_RIWAYAT: [&str; 8] = [
    "hafs",
    "shuba",
    "warsh",
    "qalun",
    "bazzi",
    "qunbul",
    "duri_abuamr",
    "susi",
];
const SHOWN_FORMS: usize = 10;

#[test]
#[ignore = "run by `just golden` (release)"]
fn style_availability_audit() {
    use std::collections::BTreeMap;
    let ds = mushaf_db::load_dir(&repo().join("data")).unwrap();
    let conv = fs::read_to_string(repo().join("data/encodings/style-conversion.json")).unwrap();
    let hafs = Renderer::load(&repo(), "hafs").unwrap();
    let mut entries = Vec::new();
    for riwayah in ALL_RIWAYAT {
        let own = mushaf_render::default_style(riwayah).unwrap();
        let (style, font_path) = *STYLE_FONTS.iter().find(|(s, _)| *s != own).unwrap();
        let spine = &ds.profiles[riwayah].spine;
        let base = if style == "maghribi" {
            mushaf_shape::EncodingTable::identity(spine)
        } else {
            let mut t = hafs.table.clone();
            if riwayah != "hafs" {
                t.extend_from_json(
                    &fs::read_to_string(repo().join("data/encodings/hafs-v1.riwayat-rows.json"))
                        .unwrap(),
                )
                .unwrap();
            }
            t
        };
        let table = base.with_style_conversion(&conv, own, style).unwrap();
        let font = mushaf_shape::FontData::new(fs::read(repo().join(font_path)).unwrap()).unwrap();
        let shaper = font.shaper();
        let sel = mushaf_resolve::Selection {
            riwayah: riwayah.into(),
            ..Default::default()
        };
        let mut resolved =
            mushaf_resolve::resolve_profile(&ds, &sel, None, mushaf_resolve::Mode::Data).unwrap();
        if own == "maghribi" {
            // D46: the owner-approved Maghribi -> Hijazi writing conventions.
            resolved = mushaf_resolve::maghribi_to_hijazi(&resolved, &ds.spines[spine]);
        }
        let (mut words, mut unconvertible, mut rule6) = (0usize, 0usize, 0usize);
        let mut forms: BTreeMap<String, usize> = BTreeMap::new();
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        for it in resolved
            .items
            .iter()
            .filter(|i| i.kind == mushaf_model::ItemKind::Word)
        {
            words += 1;
            let cl: Vec<&[char]> = it.clusters.iter().map(|c| c.cps.0.as_slice()).collect();
            let chars = match table.convert(&cl) {
                Ok(c) => c.chars,
                Err(u) => {
                    unconvertible += 1;
                    *forms.entry(u.cps.to_string()).or_default() += 1;
                    continue;
                }
            };
            let glyphs = shaper.shape(&chars, &[]).unwrap();
            let v = check_word(&font, &shaper, &chars, &[], &glyphs).unwrap();
            if !v.is_empty() {
                rule6 += 1;
                for x in v {
                    let k = format!("{x:?}");
                    let k = k
                        .split(['{', '(', ' '])
                        .next()
                        .unwrap_or_default()
                        .to_string();
                    *kinds.entry(k).or_default() += 1;
                }
            }
        }
        let mut top: Vec<(String, usize)> = forms.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        top.truncate(SHOWN_FORMS);
        entries.push(serde_json::json!({
            "riwayah": riwayah,
            "style": style,
            "font": font_path,
            "words": words,
            "unconvertible_words": unconvertible,
            "rule6_failing_words": rule6,
            // D46: offered when a render profile exists; words that cannot convert block their
            // pages, and rule-6 findings are pinned (`maghribi_hijazi_pages`).
            "available": repo().join(format!("data/render/{riwayah}-{style}.json")).exists(),
            "top_unconvertible_forms": top,
            "rule6_kinds": kinds,
        }));
    }
    let doc = serde_json::json!({
        "note": "T6.9 (D44, D46): every riwayah in its non-default style. Generated by the golden test style_availability_audit (MUSHAF_UPDATE_GOLDEN=1). A combination is available when its render profile exists; its words that cannot convert block their pages (pinned by maghribi_hijazi_pages).",
        "entries": entries,
    });
    let text = serde_json::to_string_pretty(&doc).unwrap() + "\n";
    let path = repo().join(STYLE_AVAILABILITY);
    if std::env::var_os("MUSHAF_UPDATE_GOLDEN").is_some() {
        fs::write(&path, &text).unwrap();
        return;
    }
    assert_eq!(
        text,
        fs::read_to_string(&path).unwrap(),
        "style availability changed"
    );
}

/// T6.11–T6.12 (D37, D45): difference highlighting per riwāyah, on the first page from 3 that
/// draws and has a highlight. Highlighting changes nothing drawn: the highlighted page's draw list without its
/// highlights equals the plain page byte for byte. With cited rules only nothing highlights
/// today, as every data-derived rule is a hypothesis; by default hypotheses are shown. With hypotheses
/// shown, every highlight comes from a rule that is neither `encoding` nor `spelling`, and an
/// encoding rule applied on the page never highlights. The highlights are pinned per riwāyah
/// in `tests/golden/pages/<r>-p<NNN>-highlights.json`.
#[test]
#[ignore = "run by `just golden` (release)"]
fn highlight_pairs_per_riwayah() {
    use mushaf_render::HighlightPolicy;
    use std::collections::BTreeMap;
    let update = std::env::var_os("MUSHAF_UPDATE_GOLDEN").is_some();
    let none = BTreeMap::new();
    for riwayah in [
        "shuba",
        "warsh",
        "qalun",
        "bazzi",
        "qunbul",
        "duri_abuamr",
        "susi",
    ] {
        let r = Renderer::load(&repo(), riwayah).unwrap();
        let shown = HighlightPolicy::default(); // hypotheses shown (owner, D45)
        let (page, mut hl) = (3..=604)
            .find_map(|p| {
                r.render_highlighted(p, &none, shown)
                    .ok()
                    .filter(|d| !d.highlights.is_empty())
                    .map(|d| (p, d))
            })
            .unwrap();
        let plain = r.render(page, None).unwrap();
        let cited = r
            .render_highlighted(page, &none, HighlightPolicy::CITED_ONLY)
            .unwrap();
        assert_eq!(
            cited, plain,
            "{riwayah}: no rule is cited yet, so nothing highlights"
        );
        let highlights = std::mem::take(&mut hl.highlights);
        assert_eq!(
            serde_json::to_string(&hl).unwrap(),
            serde_json::to_string(&plain).unwrap(),
            "{riwayah}: highlighting changed the drawing"
        );
        // D45 addendum 2: a highlight is a reading difference from Hafs, whatever made it;
        // labelled with what made it, never cited yet.
        for h in &highlights {
            assert!(!h.label.is_empty(), "{riwayah}: unlabelled highlight");
            assert!(!h.cited, "{riwayah}: {} is shown as cited", h.label);
        }
        let name = format!("{riwayah}-p{page:03}-highlights");
        let path = repo().join(format!("tests/golden/pages/{name}.json"));
        let text = serde_json::to_string_pretty(&highlights).unwrap() + "\n";
        if update {
            fs::write(&path, &text).unwrap();
        } else {
            assert_eq!(text, fs::read_to_string(&path).unwrap(), "{name} changed");
        }
    }
}

/// D46: Warsh and Qālūn in the Ḥijāzī style, every page. A page whose words all convert is
/// drawn and checked against rule 6 and line fit; a page with a word that has no approved
/// DigitalKhatt row is blocked. The blocked pages and every rule-6 finding are pinned in
/// `MAGHRIBI_HIJAZI_PENDING` for the owner (G-148); any change fails. A few drawn pages are
/// pinned as goldens.
const MAGHRIBI_HIJAZI_PENDING: &str = "tests/golden/shape/maghribi-hijazi-pending.json";

#[test]
#[ignore = "run by `just golden` (release)"]
fn maghribi_hijazi_pages() {
    use mushaf_render::fit_checked;
    let update = std::env::var_os("MUSHAF_UPDATE_GOLDEN").is_some();
    let mut doc = serde_json::Map::new();
    for riwayah in ["warsh", "qalun"] {
        let r = Renderer::load_style(&repo(), riwayah, "hijazi").unwrap();
        let resolved = r.resolve(None).unwrap();
        let mapping = r.map(&resolved).unwrap();
        let shaper = r.font.shaper();
        let pages = r.data.layouts[&r.profile.layout_id].pages;
        let (mut blocked, mut violations, mut drawn) = (Vec::new(), Vec::new(), Vec::new());
        for page in 1..=pages {
            let cp = fit_checked(&r, &shaper, &resolved, &mapping, page).unwrap();
            if cp.blocked.is_some() {
                blocked.push(page);
                continue;
            }
            drawn.push(page);
            for (at, v) in &cp.violations {
                violations.push(format!("p{page} {at}: {v:?}"));
            }
            if drawn.len() <= 2 {
                let d = r.draw(cp.fit.as_ref().unwrap()).unwrap();
                check_golden(&format!("{riwayah}-hijazi-p{page:03}"), &d);
            }
        }
        doc.insert(
            riwayah.into(),
            serde_json::json!({"pages": pages, "drawn": drawn.len(), "blocked": blocked, "rule6": violations}),
        );
    }
    let text = serde_json::to_string_pretty(&doc).unwrap() + "\n";
    let path = repo().join(MAGHRIBI_HIJAZI_PENDING);
    if update {
        fs::write(&path, &text).unwrap();
        return;
    }
    assert_eq!(
        text,
        fs::read_to_string(&path).unwrap(),
        "Maghribi -> Hijazi pages changed"
    );
}
