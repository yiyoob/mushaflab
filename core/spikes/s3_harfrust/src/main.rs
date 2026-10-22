//! Shape every word in build/s3/cases.json with harfrust and compare each glyph's
//! id, cluster, advance and offsets with HarfBuzz's output recorded by
//! data/spikes/s3_justify.py. Exits non-zero on any difference.

use harfrust::{Buffer, ClusterLevel, Feature, Font, ShapeOptions, ShaperFont, Tag};
use serde_json::Value;

fn main() {
    let path = std::env::args().nth(1).expect("usage: s3-harfrust <cases.json>");
    let cases: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let font_bytes = std::fs::read(cases["font"].as_str().unwrap()).unwrap();
    let font = Font::new(font_bytes, 0).unwrap();
    let shaper = ShaperFont::new(&font);

    let (mut words, mut same, mut glyphs) = (0usize, 0usize, 0usize);
    let (mut with_features, mut same_with_features) = (0usize, 0usize);
    let mut first_diffs = Vec::new();
    for case in cases["cases"].as_array().unwrap() {
        let text = case["text"].as_str().unwrap();
        let features: Vec<Feature> = case["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                let tag = f[0].as_str().unwrap().as_bytes();
                let (s, e, v) = (f[1].as_u64().unwrap(), f[2].as_u64().unwrap(), f[3].as_u64().unwrap());
                // Built directly with HarfBuzz semantics (end exclusive): harfrust 0.14.0's
                // Feature::new turns `s..e` into an inclusive end that its matcher then
                // treats as exclusive, so single-cluster ranges never apply.
                Feature { tag: Tag::new(&[tag[0], tag[1], tag[2], tag[3]]), value: v as u32, start: s as u32, end: e as u32 }
            })
            .collect();
        let mut buffer = Buffer::new();
        // Clusters are character indices, as in the Python (uharfbuzz) run.
        for (i, ch) in text.chars().enumerate() {
            buffer.push(ch as u32, i as u32);
        }
        buffer.guess_segment_properties();
        buffer.set_cluster_level(ClusterLevel::MonotoneCharacters);
        harfrust::shape(&shaper, &mut buffer, ShapeOptions::new().features(&features)).unwrap();
        let ours: Vec<[i64; 5]> = buffer
            .glyph_infos()
            .iter()
            .zip(buffer.glyph_positions())
            .map(|(i, p)| [i.glyph_id as i64, i.cluster as i64, p.x_advance as i64, p.x_offset as i64, p.y_offset as i64])
            .collect();
        let theirs: Vec<[i64; 5]> = case["glyphs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| {
                let a = g.as_array().unwrap();
                [0, 1, 2, 3, 4].map(|k| a[k].as_i64().unwrap())
            })
            .collect();
        words += 1;
        glyphs += theirs.len();
        if !features.is_empty() {
            with_features += 1;
            same_with_features += usize::from(ours == theirs);
        }
        if ours == theirs {
            same += 1;
        } else if first_diffs.len() < 5 {
            first_diffs.push(format!(
                "page {} line {}: harfrust {:?} vs harfbuzz {:?}",
                case["page"], case["line"], &ours[..ours.len().min(3)], &theirs[..theirs.len().min(3)]
            ));
        }
    }
    println!("harfbuzz reference {}", cases["harfbuzz"]);
    println!("words {words}, glyphs {glyphs}, identical words {same}");
    println!(
        "  without stretch features: {}/{}; with stretch features: {same_with_features}/{with_features}",
        same - same_with_features,
        words - with_features
    );
    for d in &first_diffs {
        println!("  {d}");
    }
    if same != words {
        std::process::exit(1);
    }
}
