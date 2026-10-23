//! T3.2: manual overrides take precedence and are never silently dropped (D15).
//!
//! Lines here are synthetic letter sequences (test fixtures, not Quranic text), shaped with the
//! tracked DigitalKhatt Madina font.

mod common;

use common::dk_font;
use mushaf_justify::{
    LineInput, ONE, Params, Regexps, justify_line, manual_stretch, natural_width,
};

const BEH: char = '\u{0628}';

/// Three words of four joined behs each.
fn line() -> Vec<char> {
    let word: String = std::iter::repeat_n(BEH, 4).collect();
    format!("{word} {word} {word}").chars().collect()
}

#[test]
fn extra_advance_counts_in_the_natural_width_and_the_line_still_closes() {
    let font = dk_font();
    let sh = font.shaper();
    let text = line();
    let plain = LineInput {
        text: text.clone(),
        ..LineInput::default()
    };
    let wide = LineInput {
        text: text.clone(),
        extra: vec![0, 300 * ONE, 0],
        ..LineInput::default()
    };
    let n0 = natural_width(&sh, &plain).unwrap();
    assert_eq!(natural_width(&sh, &wide).unwrap(), n0 + 300 * ONE);
    let target = n0 + 2000 * ONE;
    let j = justify_line(&sh, &Regexps::new(), &wide, target, Params::default()).unwrap();
    assert_eq!(j.width, target);
    assert_eq!(
        j.words.iter().sum::<i64>() + j.spaces.iter().sum::<i64>(),
        target
    );
}

#[test]
fn locked_letters_are_never_stretched_and_manual_features_are_kept() {
    let font = dk_font();
    let sh = font.shaper();
    let text = line();
    let target = natural_width(
        &sh,
        &LineInput {
            text: text.clone(),
            ..LineInput::default()
        },
    )
    .unwrap()
        + 3000 * ONE;
    let free = justify_line(
        &sh,
        &Regexps::new(),
        &LineInput {
            text: text.clone(),
            ..LineInput::default()
        },
        target,
        Params::default(),
    )
    .unwrap();
    assert!(
        free.features.keys().any(|i| (5..9).contains(i)),
        "the middle word is stretched when free"
    );
    let mut locked = vec![false; text.len()];
    for flag in &mut locked[5..9] {
        *flag = true;
    }
    let mut input = LineInput {
        text: text.clone(),
        locked,
        ..LineInput::default()
    };
    input.manual.entry(6).or_default().insert(*b"cv01", 1);
    let j = justify_line(&sh, &Regexps::new(), &input, target, Params::default()).unwrap();
    let middle: Vec<_> = j.features.range(5..9).collect();
    assert_eq!(middle.len(), 1);
    assert_eq!(j.features[&6].get(b"cv01"), Some(&1));
    assert_eq!(j.features[&6].len(), 1);
    assert_eq!(j.width, target);
}

#[test]
fn an_overfull_line_tightens_spaces_then_squeezes() {
    let font = dk_font();
    let sh = font.shaper();
    let input = LineInput {
        text: line(),
        ..LineInput::default()
    };
    let n = natural_width(&sh, &input).unwrap();
    let rx = Regexps::new();
    // a little too wide: spaces absorb it, no squeeze
    let j = justify_line(&sh, &rx, &input, n - 20 * ONE, Params::default()).unwrap();
    assert_eq!(
        (j.width, j.x_scale),
        (n - 20 * ONE, mushaf_justify::SCALE_ONE)
    );
    // far too wide: spaces at 60%, then x_scale brings the line to its target
    let target = n * 8 / 10;
    let j = justify_line(&sh, &rx, &input, target, Params::default()).unwrap();
    let space = mushaf_justify::space_width(&sh).unwrap();
    assert!(j.spaces.iter().all(|s| *s == space * 6 / 10));
    assert!(j.x_scale < mushaf_justify::SCALE_ONE);
    // x_scale (16.16) is for reports; positions are scaled by the exact ratio target / width
    let drawn = j.width * j.x_scale / mushaf_justify::SCALE_ONE;
    assert!(
        (drawn - target).abs() <= j.width / mushaf_justify::SCALE_ONE + 1,
        "{drawn} vs {target}"
    );
}

#[test]
fn manual_stretch_takes_the_smallest_level_or_reports_it_cannot() {
    let font = dk_font();
    let sh = font.shaper();
    let text = line();
    let small = manual_stretch(&sh, &text, 1, ONE).unwrap();
    assert!(small.reached && small.achieved >= ONE);
    assert_eq!(small.features[&1].get(b"cv01"), Some(&1));
    let huge = manual_stretch(&sh, &text, 1, 100_000 * ONE).unwrap();
    assert!(!huge.reached);
    assert!(huge.achieved >= small.achieved);
    assert!(
        manual_stretch(&sh, &text, 4, ONE).is_err(),
        "a space is not a letter"
    );
}
