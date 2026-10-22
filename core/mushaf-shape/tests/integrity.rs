//! T3.5a: the integrity check catches each kind of silent failure (rule 6). Inputs are
//! synthetic codepoint pairs (test fixtures, not Quranic text) found by trying every base and
//! mark the DigitalKhatt Madina font maps.

mod common;

use common::dk_font;
use mushaf_shape::{Violation, check_word};

fn check(text: &str) -> Vec<Violation> {
    let font = dk_font();
    let sh = font.shaper();
    let chars: Vec<char> = text.chars().collect();
    let glyphs = sh.shape(&chars, &[]).unwrap();
    check_word(&font, &sh, &chars, &[], &glyphs).unwrap()
}

#[test]
fn a_well_formed_cluster_passes() {
    assert_eq!(check("\u{0628}\u{064E}"), vec![]);
}

#[test]
fn a_codepoint_the_font_lacks_is_notdef() {
    // U+0657 is KFGQPC's open fathatan, which DigitalKhatt writes as U+08F0 (G-018).
    assert!(matches!(
        check("\u{0628}\u{0657}")[..],
        [Violation::Notdef { .. }, ..]
    ));
}

#[test]
fn a_mark_without_an_anchor_on_its_base_is_unattached() {
    assert!(
        check("\u{0621}\u{06DC}")
            .iter()
            .any(|v| matches!(v, Violation::UnattachedMark { .. }))
    );
}

#[test]
fn a_lone_mark_is_unattached_because_the_font_has_no_dotted_circle() {
    let font = dk_font();
    assert_eq!(
        font.nominal_glyph('\u{25CC}'),
        None,
        "the font would draw inserted dotted circles"
    );
    assert!(
        check("\u{064E}")
            .iter()
            .any(|v| matches!(v, Violation::UnattachedMark { .. }))
    );
}
