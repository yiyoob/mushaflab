//! Fit all 604 Hafs pages with the tracked render profile and print the numbers behind D15.
//!
//!     cargo run --release --manifest-path core/Cargo.toml -p mushaf-layout --example fit_report

use std::fs;
use std::path::Path;

use mushaf_justify::{ONE, Regexps, SCALE_ONE};
use mushaf_layout::fit::{
    FitData, FitKind, LineWidths, Profile, SurahNames, basmalah_words, fit_page,
};
use mushaf_layout::map_lines;
use mushaf_resolve::{Mode, resolve_patches};
use mushaf_shape::{EncodingTable, FontData};

fn pct(v: &mut [i64], q: usize) -> i64 {
    v.sort();
    v[(v.len() - 1) * q / 100]
}

fn main() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let read = |p: &str| fs::read_to_string(repo.join(p)).unwrap();
    let mut profile = Profile::from_json(&read("data/render/hafs-uthmani-1441h.json")).unwrap();
    // exploration only: MUSHAF_LINE_WIDTH / MUSHAF_SCALE_FLOOR override the tracked profile
    if let Ok(w) = std::env::var("MUSHAF_LINE_WIDTH") {
        profile.geometry.line_width = w.parse().unwrap();
    }
    if let Ok(f) = std::env::var("MUSHAF_SCALE_FLOOR") {
        profile.geometry.page_scale_floor = f.parse().unwrap();
    }
    println!(
        "line width {}, page scale floor {}",
        profile.geometry.line_width, profile.geometry.page_scale_floor
    );
    let ds = mushaf_db::load_dir(&repo.join("data")).unwrap();
    let spine = &ds.spines[&profile.spine];
    let layout = &ds.layouts[&profile.layout_id];
    let table = EncodingTable::from_json(&read(&profile.encoding)).unwrap();
    let widths = LineWidths::from_json(&read(&profile.line_widths)).unwrap();
    let names = SurahNames::from_json(&read(&profile.surah_names)).unwrap();
    let font = FontData::new(fs::read(repo.join(&profile.font)).unwrap()).unwrap();
    let shaper = font.shaper();
    let basmalah = basmalah_words(spine, &table).unwrap();
    let data = FitData {
        profile: &profile,
        table: &table,
        widths: &widths,
        names: &names,
        basmalah: &basmalah,
        upem: i64::from(font.units_per_em()),
    };
    let resolved = resolve_patches(spine, &[], Mode::Data).unwrap();
    let mapping = map_lines(layout, spine, &resolved).unwrap();
    let rx = Regexps::new();
    let space = mushaf_justify::space_width(&shaper).unwrap();
    let (mut scales, mut squeezes, mut gains, mut space_ratio) = (vec![], vec![], vec![], vec![]);
    let (mut lines, mut stretched, mut at_floor) = (0, 0, 0);
    for page in 1..=604 {
        let fp = fit_page(&shaper, &rx, &data, &resolved, &mapping, page).unwrap();
        scales.push(fp.scale * 1000 / SCALE_ONE);
        at_floor += usize::from(
            fp.notes
                .iter()
                .any(|n| matches!(n, mushaf_layout::fit::FitNote::PageScaleAtFloor { .. })),
        );
        for l in fp
            .lines
            .iter()
            .filter(|l| l.kind == FitKind::Text && !l.words.is_empty())
        {
            lines += 1;
            if let Some((t, w)) = l.squeeze {
                squeezes.push(t * 1000 / w);
            }
            if l.letter_gain > 0 {
                stretched += 1;
                gains.push(l.letter_gain / ONE);
            }
            if let Some(max) = l.spaces.iter().max() {
                space_ratio.push(max * 100 / space);
            }
        }
    }
    let scaled = scales.iter().filter(|s| **s < 1000).count();
    println!("text lines {lines}; pages scaled {scaled}; pages at floor {at_floor}");
    println!(
        "page scale per mille: min {} p5 {} median {}",
        pct(&mut scales, 0),
        pct(&mut scales, 5),
        pct(&mut scales, 50)
    );
    let hist = |lo: i64, hi: i64| squeezes.iter().filter(|x| (lo..hi).contains(*x)).count();
    println!(
        "squeeze per mille histogram: [990,1000) {} [950,990) {} [900,950) {} <900 {}",
        hist(990, 1000),
        hist(950, 990),
        hist(900, 950),
        hist(0, 900)
    );
    println!(
        "squeezed lines {}; x-scale per mille min {}",
        squeezes.len(),
        squeezes.iter().min().copied().unwrap_or(1000)
    );
    println!(
        "lines with letter stretching {stretched}; gain font units median {} p95 {} max {}",
        pct(&mut gains, 50),
        pct(&mut gains, 95),
        pct(&mut gains, 100)
    );
    println!(
        "widest space per line, % of normal: median {} p95 {} max {}",
        pct(&mut space_ratio, 50),
        pct(&mut space_ratio, 95),
        pct(&mut space_ratio, 100)
    );
}
