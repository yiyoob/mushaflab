//! T3.1 regression (D3): harfrust reproduces HarfBuzz 14.6.0 on the 1,559 words of spike S3,
//! with and without stretch features. Expectations: tests/golden/shape/s3-parity.json, written
//! by data/spikes/s3_parity_fixture.py from HarfBuzz's output; the words themselves are read
//! from the pinned DigitalKhatt text (rule 1: no text copied into tests).

mod common;

use std::fs;

use common::{dk_font, dk_pages, pinned, repo};
use mushaf_shape::FeatureRange;
use serde_json::Value;

#[test]
fn harfrust_matches_harfbuzz_on_the_s3_word_set() {
    let Some(text) = pinned("digitalkhatt-text-madina") else {
        return;
    };
    let pages = dk_pages(std::str::from_utf8(&text).unwrap());
    let golden: Value = serde_json::from_str(
        &fs::read_to_string(repo().join("tests/golden/shape/s3-parity.json")).unwrap(),
    )
    .unwrap();
    let font = dk_font();
    let shaper = font.shaper();
    let cases = golden["cases"].as_array().unwrap();
    let (mut with_features, mut diffs) = (0, Vec::new());
    for case in cases {
        let line = &pages[case["page"].as_u64().unwrap() as usize - 1]
            [case["line"].as_u64().unwrap() as usize - 1];
        let word: Vec<char> = line
            .split(' ')
            .nth(case["word"].as_u64().unwrap() as usize)
            .unwrap()
            .chars()
            .collect();
        let features: Vec<FeatureRange> = case["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| FeatureRange {
                tag: f[0].as_str().unwrap().as_bytes().try_into().unwrap(),
                start: f[1].as_u64().unwrap() as u32,
                end: f[2].as_u64().unwrap() as u32,
                value: f[3].as_u64().unwrap() as u32,
            })
            .collect();
        with_features += usize::from(!features.is_empty());
        let ours: Vec<[i64; 5]> = shaper
            .shape(&word, &features)
            .unwrap()
            .iter()
            .map(|g| {
                [
                    i64::from(g.glyph),
                    i64::from(g.cluster),
                    i64::from(g.x_advance),
                    i64::from(g.x_offset),
                    i64::from(g.y_offset),
                ]
            })
            .collect();
        let theirs: Vec<[i64; 5]> = case["glyphs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| [0, 1, 2, 3, 4].map(|k| g[k].as_i64().unwrap()))
            .collect();
        if ours != theirs {
            diffs.push(format!(
                "{} {} {}",
                case["page"], case["line"], case["word"]
            ));
        }
    }
    assert_eq!(cases.len(), 1559);
    assert_eq!(with_features, 124);
    assert!(
        diffs.is_empty(),
        "{} words differ: {:?}",
        diffs.len(),
        &diffs[..diffs.len().min(10)]
    );
}
