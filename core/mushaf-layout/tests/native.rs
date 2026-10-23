//! T5.2: native layouts (D29) on the real data. The fitting reproduces the 1441H Ḥafṣ layout
//! from KFGQPC's Ḥafṣ ayah lines, and every riwāyah's layout keeps each ayah on the lines its
//! KFGQPC file gives. Widths do not matter here (every Ḥafṣ break is forced or transferred).

use std::fs;
use std::path::{Path, PathBuf};

use mushaf_layout::native::{AyahLinesFile, NativeInput, ayah_ranges, fit_native, lines_per_page};
use mushaf_model::{FitTag, LineKind, from_yaml};
use mushaf_resolve::{Mode, Selection, resolve_profile};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn hafs_ayah_lines_reproduce_the_1441h_layout() {
    let ds = mushaf_db::load_dir(&repo().join("data")).unwrap();
    let hafs = &ds.layouts["uthmani-1441h"];
    let sel = Selection {
        riwayah: "hafs".into(),
        ..Selection::default()
    };
    let base = resolve_profile(&ds, &sel, None, Mode::Data).unwrap();
    let file: AyahLinesFile = from_yaml(
        &fs::read_to_string(repo().join("data/layout_inputs/hafs.ayah-lines.yaml")).unwrap(),
    )
    .unwrap();
    let input = NativeInput {
        riwayah: "hafs",
        layout_id: "uthmani-hafs-kfgqpc",
        text_source: &file.source,
        base: &base,
        ayahs: ayah_ranges(&base, None, &file).unwrap(),
        hafs,
        spine: &ds.spines["hafs/v1"],
        lines_per_page: lines_per_page(hafs),
        headers: &ds.profiles["hafs"].surah_headers,
        widths: vec![Some(64); base.items.len()],
        space: 64,
    };
    let fitted = fit_native(&input).unwrap();
    assert!(fitted.report.iter().all(|l| l.fit != FitTag::Fitted));
    let kinds = |ls: &[mushaf_model::Line]| -> Vec<(u32, u32, LineKind)> {
        ls.iter()
            .map(|l| (l.page, l.line, l.kind.clone()))
            .collect()
    };
    assert_eq!(kinds(&fitted.layout.lines), kinds(&hafs.lines));
}

/// Every tracked native layout puts each ayah's first item on the line where its KFGQPC file
/// starts it and its last item on the line where it ends it, and maps the riwāyah's text.
#[test]
fn native_layouts_keep_every_ayah_on_its_kfgqpc_lines() {
    use mushaf_layout::map_lines_on;
    let ds = mushaf_db::load_dir(&repo().join("data")).unwrap();
    for riwayah in [
        "shuba",
        "warsh",
        "qalun",
        "bazzi",
        "qunbul",
        "duri_abuamr",
        "susi",
    ] {
        let layout = &ds.layouts[&format!("uthmani-{riwayah}-kfgqpc")];
        assert_eq!(layout.riwayah.as_deref(), Some(riwayah));
        let sel = Selection {
            riwayah: riwayah.into(),
            ..Selection::default()
        };
        let base = resolve_profile(&ds, &sel, None, Mode::Data).unwrap();
        let mapping = map_lines_on(layout, &base, &base).unwrap();
        let mut line_of = vec![(0, 0); base.items.len()];
        for l in &mapping.lines {
            for i in l.items.clone() {
                line_of[i] = (l.page, l.line);
            }
        }
        let file: AyahLinesFile = from_yaml(
            &fs::read_to_string(
                repo().join(format!("data/layout_inputs/{riwayah}.ayah-lines.yaml")),
            )
            .unwrap(),
        )
        .unwrap();
        let map = ds.profiles[riwayah]
            .ayah_map
            .as_ref()
            .map(|m| &ds.ayah_maps[m]);
        for (a, b, l) in ayah_ranges(&base, map, &file).unwrap() {
            assert_eq!(
                line_of[a],
                (l.page, l.line_start),
                "{riwayah} {}:{} start",
                l.surah,
                l.ayah
            );
            assert_eq!(
                line_of[b],
                (l.last_page, l.line_end),
                "{riwayah} {}:{} end",
                l.surah,
                l.ayah
            );
        }
    }
}
