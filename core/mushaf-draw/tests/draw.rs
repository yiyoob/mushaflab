//! T3.3 acceptance (D4, rule 11): exact outlines, deterministic draw commands and SVG, and the
//! edited marker on exactly the non-canonical pages.

use std::fs;
use std::path::{Path, PathBuf};

use mushaf_draw::{DrawItem, DrawPage, Outlines, PageMeta, PathOp, Role, draw_page, to_svg};
use mushaf_justify::Regexps;
use mushaf_layout::fit::{FitData, LineWidths, Profile, SurahNames, basmalah_words, fit_page};
use mushaf_layout::map_lines;
use mushaf_model::{ClusterRef, LineKind, Patch};
use mushaf_resolve::{Mode, Resolved, resolve_patches};
use mushaf_shape::{EncodingTable, FontData};
use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn read(p: &str) -> String {
    fs::read_to_string(repo().join(p)).unwrap()
}

fn font_bytes() -> Vec<u8> {
    fs::read(repo().join("fonts/digitalkhatt/hb-madina.otf")).unwrap()
}

#[test]
fn outlines_match_an_independent_reader() {
    let golden: Value =
        serde_json::from_str(&read("tests/golden/draw/outline-sample.json")).unwrap();
    let outlines = Outlines::new(&font_bytes()).unwrap();
    assert_eq!(outlines.len() as u64, golden["glyphs"].as_u64().unwrap());
    let mut worst = 0i64;
    for case in golden["cases"].as_array().unwrap() {
        let gid = case["glyph"].as_u64().unwrap() as u32;
        let ours: Vec<(&str, Vec<(i64, i64)>)> = outlines
            .get(gid)
            .unwrap()
            .iter()
            .map(|op| match *op {
                PathOp::Move { x, y } => ("moveTo", vec![(x.into(), y.into())]),
                PathOp::Line { x, y } => ("lineTo", vec![(x.into(), y.into())]),
                PathOp::Cubic {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => (
                    "curveTo",
                    vec![
                        (x1.into(), y1.into()),
                        (x2.into(), y2.into()),
                        (x.into(), y.into()),
                    ],
                ),
                PathOp::Close => ("closePath", vec![]),
            })
            .collect();
        let theirs: Vec<(&str, Vec<(i64, i64)>)> = case["ops"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| {
                let pts = o[1]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| (p[0].as_i64().unwrap(), p[1].as_i64().unwrap()))
                    .collect();
                (o[0].as_str().unwrap(), pts)
            })
            .collect();
        assert_eq!(ours.len(), theirs.len(), "glyph {gid}: op count");
        for (a, b) in ours.iter().zip(&theirs) {
            assert_eq!(a.0, b.0, "glyph {gid}: op kind");
            for (p, q) in a.1.iter().zip(&b.1) {
                worst = worst.max((p.0 - q.0).abs()).max((p.1 - q.1).abs());
            }
        }
    }
    assert!(worst <= 1, "outline points differ by {worst}/64 unit");
}

struct Env {
    profile: Profile,
    ds: mushaf_model::DataSet,
    table: EncodingTable,
    widths: LineWidths,
    names: SurahNames,
    font: FontData,
    outlines: Outlines,
}

fn env() -> Env {
    let profile = Profile::from_json(&read("data/render/hafs-uthmani-1441h.json")).unwrap();
    Env {
        ds: mushaf_db::load_dir(&repo().join("data")).unwrap(),
        table: EncodingTable::from_json(&read(&profile.encoding)).unwrap(),
        widths: LineWidths::from_json(&read(&profile.line_widths)).unwrap(),
        names: SurahNames::from_json(&read(&profile.surah_names)).unwrap(),
        font: FontData::new(font_bytes()).unwrap(),
        outlines: Outlines::new(&font_bytes()).unwrap(),
        profile,
    }
}

fn draw(e: &Env, resolved: &Resolved, page: u32) -> DrawPage {
    let spine = &e.ds.spines[&e.profile.spine];
    let layout = &e.ds.layouts[&e.profile.layout_id];
    let basmalah = basmalah_words(spine, &e.table).unwrap();
    let data = FitData {
        profile: &e.profile,
        table: &e.table,
        widths: &e.widths,
        names: &e.names,
        basmalah: &basmalah,
        upem: i64::from(e.font.units_per_em()),
    };
    let mapping = map_lines(layout, spine, resolved).unwrap();
    let fit = fit_page(
        &e.font.shaper(),
        &Regexps::new(),
        &data,
        resolved,
        &mapping,
        page,
    )
    .unwrap();
    let meta = PageMeta {
        layout_id: &e.profile.layout_id,
        script_id: &e.profile.script_id,
        riwayah: &e.profile.riwayah,
        font: &e.profile.font,
    };
    draw_page(&fit, &e.profile.geometry, &e.outlines, &meta).unwrap()
}

fn markers(p: &DrawPage) -> usize {
    p.items
        .iter()
        .filter(|i| {
            matches!(
                i,
                DrawItem::Stroke {
                    role: Role::EditedMarker,
                    ..
                }
            )
        })
        .count()
}

#[test]
fn draw_commands_and_svg_are_byte_identical_and_canonical_pages_have_no_marker() {
    let e = env();
    let spine = &e.ds.spines[&e.profile.spine];
    let resolved = resolve_patches(spine, &[], Mode::Data).unwrap();
    for page in [1, 50] {
        let (a, b) = (draw(&e, &resolved, page), draw(&e, &resolved, page));
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        assert_eq!(to_svg(&a), to_svg(&b));
        assert!(a.canonical);
        assert_eq!(markers(&a), 0);
        // every text glyph knows its cluster; header glyphs do not
        assert!(a.items.iter().any(|i| matches!(
            i,
            DrawItem::Glyph {
                cluster: Some(_),
                ..
            }
        )));
        assert!(
            a.items
                .iter()
                .all(|i| !matches!(i, DrawItem::Glyph { glyph: 0, .. })),
            ".notdef drawn"
        );
    }
}

#[test]
fn an_edited_page_always_carries_the_marker_last() {
    let e = env();
    let spine = &e.ds.spines[&e.profile.spine];
    let layout = &e.ds.layouts[&e.profile.layout_id];
    let first = layout
        .lines
        .iter()
        .find_map(|l| match (&l.kind, l.page, l.line) {
            (LineKind::Text { first, .. }, 3, 2) => Some(first.clone()),
            _ => None,
        })
        .unwrap();
    // Synthetic free-mode edit (test fixture, not Quranic data).
    let target = ClusterRef {
        item: first,
        n: 1,
        ins: Vec::new(),
    };
    let patch: Patch = mushaf_model::from_yaml(&format!(
        "id: 'user.demo'\nowner: {{level: 'user', id: 'demo'}}\nop: 'set_attr'\ntarget: '{target}'\npayload: {{width_em_delta: 300}}\nsource: {{kind: 'user', ref: 'test'}}\nstatus: 'user-edit'\n"
    )).unwrap();
    let edited = resolve_patches(spine, &[&patch], Mode::Free).unwrap();
    let p = draw(&e, &edited, 3);
    assert!(!p.canonical);
    assert_eq!(markers(&p), 2);
    let n = p.items.len();
    assert!(p.items[n - 2..].iter().all(|i| matches!(
        i,
        DrawItem::Stroke {
            role: Role::EditedMarker,
            ..
        }
    )));
    assert!(to_svg(&p).contains("#cc0000") && to_svg(&p).contains(" EDITED</desc>"));
}
