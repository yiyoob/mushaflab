//! T3.2 acceptance: the Rust port reproduces the oracle (quran-madina-html `justify.py` @
//! d520c5c, a Python port of DigitalKhatt's MIT justifier) on every loose `ayah` line of
//! DigitalKhatt's own Madina text at its own line widths. Expectations:
//! tests/golden/justify/oracle-1421h.json, written by data/spikes/s3_oracle.py; the lines are
//! read from the pinned source (no text copied into tests).
//!
//! A third run uses the D17 render font (the pinned font with one rclt substitution changed).
//!
//! Exact: natural width and every stretch feature (character, tag, value). Spaces: within one
//! font unit of the oracle's float widths (ours are exact 1/64 units that close the line).

mod common;

use std::fs;

use common::{derived_font, dk_font, dk_pages, pinned, repo};
use mushaf_justify::{LineInput, ONE, Params, Regexps, SpaceKind, justify_line};
use serde_json::Value;

/// Loose lines of the third run (D17 render font), as the oracle counted them.
const DERIVED_RUN_LINES: usize = 4121;

#[test]
fn port_matches_the_oracle_on_every_loose_line() {
    let Some(text) = pinned("digitalkhatt-text-madina") else {
        return;
    };
    let pages = dk_pages(std::str::from_utf8(&text).unwrap());
    let golden: Value = serde_json::from_str(
        &fs::read_to_string(repo().join("tests/golden/justify/oracle-1421h.json")).unwrap(),
    )
    .unwrap();
    let pinned = dk_font();
    let derived = derived_font();
    let (pinned_shaper, derived_shaper) = (pinned.shaper(), derived.shaper());
    let rx = Regexps::new();
    let (mut lines, mut stretched, mut worst) = (0, 0, 0i64);
    let mut bad = Vec::new();
    for run in golden["runs"].as_array().unwrap() {
        // the third run uses the D17 render font
        let shaper = match run["font"].as_str().unwrap() {
            "pinned" => &pinned_shaper,
            "derived" => &derived_shaper,
            other => panic!("unknown font {other}"),
        };
        let params = Params {
            space_stretch: run["space_stretch_per_mille"].as_i64().unwrap(),
            ..Params::default()
        };
        for l in run["lines"].as_array().unwrap() {
            let (p, n) = (l["page"].as_u64().unwrap(), l["line"].as_u64().unwrap());
            let input = LineInput {
                text: pages[p as usize - 1][n as usize - 1].chars().collect(),
                ..LineInput::default()
            };
            let target = l["target"].as_i64().unwrap();
            let j = justify_line(shaper, &rx, &input, target, params).unwrap();
            lines += 1;
            let ours: Vec<(u64, String, u64)> = j
                .features
                .iter()
                .flat_map(|(i, fs)| {
                    fs.iter().map(move |(t, v)| {
                        (
                            *i as u64,
                            String::from_utf8(t.to_vec()).unwrap(),
                            u64::from(*v),
                        )
                    })
                })
                .collect();
            let theirs: Vec<(u64, String, u64)> = l["features"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| {
                    (
                        f[0].as_u64().unwrap(),
                        f[1].as_str().unwrap().to_string(),
                        f[2].as_u64().unwrap(),
                    )
                })
                .collect();
            stretched += usize::from(!theirs.is_empty());
            if j.natural != l["natural"].as_i64().unwrap() * ONE || ours != theirs {
                bad.push(format!("{p}:{n} features/natural"));
                continue;
            }
            for (w, k) in j.spaces.iter().zip(&j.space_kinds) {
                let want = l[if *k == SpaceKind::Aya {
                    "aya"
                } else {
                    "simple"
                }]
                .as_i64()
                .unwrap();
                worst = worst.max((w - want).abs());
                if (w - want).abs() > ONE {
                    bad.push(format!("{p}:{n} space {w} vs {want}"));
                }
            }
            assert_eq!(j.width, target, "{p}:{n} does not close");
        }
    }
    eprintln!(
        "oracle parity: {lines} lines, {stretched} with letter stretching, worst space diff {worst}/64 unit"
    );
    assert_eq!(lines, 4062 + 14 + DERIVED_RUN_LINES);
    assert!(
        bad.is_empty(),
        "{} lines differ: {:?}",
        bad.len(),
        &bad[..bad.len().min(10)]
    );
}
