//! T3.2: page fitting with the tracked Hafs render profile (D15). Expectations come from the
//! tracked data: the layout's line kinds, the print's short-line widths (line-widths.json),
//! and the profile's geometry; nothing about the text is asserted here.

use std::fs;
use std::path::{Path, PathBuf};

use mushaf_justify::{ONE, Regexps, SCALE_ONE};
use mushaf_layout::fit::{
    FitData, FitKind, FitPage, LineWidths, Profile, SurahNames, basmalah_words, fit_page,
};
use mushaf_layout::map_lines;
use mushaf_model::{Attrs, ClusterRef, Patch};
use mushaf_resolve::{Mode, Resolved, resolve_patches};
use mushaf_shape::{EncodingTable, FontData};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

struct Env {
    profile: Profile,
    ds: mushaf_model::DataSet,
    table: EncodingTable,
    widths: LineWidths,
    names: SurahNames,
    font: FontData,
}

fn env() -> Env {
    let read = |p: &str| fs::read_to_string(repo().join(p)).unwrap();
    let profile = Profile::from_json(&read("data/render/hafs-uthmani-1441h.json")).unwrap();
    Env {
        ds: mushaf_db::load_dir(&repo().join("data")).unwrap(),
        table: EncodingTable::from_json(&read(&profile.encoding)).unwrap(),
        widths: LineWidths::from_json(&read(&profile.line_widths)).unwrap(),
        names: SurahNames::from_json(&read(&profile.surah_names)).unwrap(),
        font: FontData::new(fs::read(repo().join(&profile.font)).unwrap()).unwrap(),
        profile,
    }
}

fn fit(e: &Env, resolved: &Resolved, page: u32) -> FitPage {
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
    fit_page(
        &e.font.shaper(),
        &Regexps::new(),
        &data,
        resolved,
        &mapping,
        page,
    )
    .unwrap()
}

#[test]
fn every_text_line_closes_to_its_target_and_short_lines_are_centred() {
    let e = env();
    let spine = &e.ds.spines[&e.profile.spine];
    let resolved = resolve_patches(spine, &[], Mode::Data).unwrap();
    let floor = e.profile.geometry.page_scale_floor * SCALE_ONE / 1000;
    for page in [1, 2, 3, 50, 300, 604] {
        let fp = fit(&e, &resolved, page);
        assert!(fp.canonical);
        assert!(
            (floor..=SCALE_ONE).contains(&fp.scale),
            "page {page} scale {}",
            fp.scale
        );
        assert_eq!(fp.lines.len(), if page <= 2 { 8 } else { 15 });
        for l in &fp.lines {
            let full = e.profile.geometry.line_width * ONE * SCALE_ONE / fp.scale;
            assert_eq!(l.box_width, full);
            match l.kind {
                FitKind::Text => {
                    let ppm = e.widths.short.get(&(page, l.line)).copied();
                    assert_eq!(l.short_ppm, ppm);
                    assert_eq!(l.centered, ppm.is_some());
                    let want = ppm.map_or(full, |p| {
                        e.profile.geometry.line_width * ONE * p / 1_000_000 * SCALE_ONE / fp.scale
                    });
                    assert!(
                        (l.target - want).abs() <= ONE,
                        "page {page} line {}",
                        l.line
                    );
                    match l.squeeze {
                        None => assert_eq!(l.width, l.target),
                        Some((t, w)) => assert!(t == l.target && w > t),
                    }
                }
                _ => assert!(l.centered && l.width == l.natural),
            }
        }
    }
}

#[test]
fn an_edited_page_is_not_canonical_and_overrides_widen_the_word() {
    let e = env();
    let spine = &e.ds.spines[&e.profile.spine];
    // Synthetic free-mode edit (a test fixture, not Quranic data): widen the first cluster of
    // the page-3 line-1 word by 0.5 em.
    let layout = &e.ds.layouts[&e.profile.layout_id];
    let first = layout
        .lines
        .iter()
        .find_map(|l| match (&l.kind, l.page, l.line) {
            (mushaf_model::LineKind::Text { first, .. }, 3, 1) => Some(first.clone()),
            _ => None,
        })
        .unwrap();
    let target = ClusterRef {
        item: first.clone(),
        n: 1,
        ins: Vec::new(),
    };
    let patch: Patch = mushaf_model::from_yaml(&format!(
        "id: 'user.demo.width'\nowner: {{level: 'user', id: 'demo'}}\nop: 'set_attr'\ntarget: '{target}'\npayload: {{width_em_delta: 500}}\nsource: {{kind: 'user', ref: 'test'}}\nstatus: 'user-edit'\n"
    ))
    .unwrap();
    let edited = resolve_patches(spine, &[&patch], Mode::Free).unwrap();
    let plain = resolve_patches(spine, &[], Mode::Data).unwrap();
    assert_eq!(edited.user_patches, vec!["user.demo.width".to_string()]);
    let item = edited.items.iter().find(|i| i.id == first).unwrap();
    assert_eq!(
        item.clusters[0].attrs,
        Attrs {
            width_em_delta: Some(500),
            ..Attrs::default()
        }
    );
    let (a, b) = (fit(&e, &plain, 3), fit(&e, &edited, 3));
    assert!(a.canonical && !b.canonical);
    let word = &b.lines[0].words[0];
    assert_eq!(edited.items[word.item.unwrap()].id, first);
    let upem = i64::from(e.font.units_per_em());
    assert_eq!(
        word.gaps,
        vec![(word.spans[0].chars.end - 1, 500 * upem * ONE / 1000)]
    );
    assert_eq!(a.lines[0].words[0].gaps, vec![]);
    assert!(
        b.lines[0].squeeze.is_none() && b.lines[0].width == b.lines[0].target,
        "the line still closes"
    );
}
