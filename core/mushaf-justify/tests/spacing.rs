//! T5.3: spacing-only justification for fonts without stretch features (the KFGQPC Warsh and
//! Qālūn fonts, D2): letters never stretch, spaces take the whole difference, and a space next
//! to a KFGQPC ayah sign (U+FC00 onwards) counts as an ayah space.
//!
//! Lines are synthetic letter sequences (test fixtures, not Quranic text).

use std::fs;
use std::path::{Path, PathBuf};

use mushaf_justify::{LineInput, ONE, Params, Regexps, SpaceKind, justify_line, natural_width};
use mushaf_shape::FontData;

fn warsh_font() -> FontData {
    let repo: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    FontData::new(fs::read(repo.join("fonts/kfgqpc/uthmanic_warsh_v21.ttf")).unwrap()).unwrap()
}

#[test]
fn spaces_alone_close_the_line_and_letters_never_stretch() {
    let font = warsh_font();
    let sh = font.shaper();
    let text: Vec<char> =
        "\u{0628}\u{0628}\u{0628} \u{0628}\u{0628} \u{0628}\u{0628}\u{0628}\u{0628} \u{FC00}"
            .chars()
            .collect();
    let input = LineInput {
        text,
        ..LineInput::default()
    };
    let natural = natural_width(&sh, &input).unwrap();
    let params = Params {
        letters: false,
        ..Params::default()
    };
    let target = natural * 2;
    let j = justify_line(&sh, &Regexps::new(), &input, target, params).unwrap();
    assert_eq!(j.width, target);
    assert_eq!(j.letter_gain, 0);
    assert!(j.features.is_empty());
    assert_eq!(
        j.words.iter().sum::<i64>() + j.spaces.iter().sum::<i64>(),
        target
    );
    assert_eq!(
        j.space_kinds,
        [SpaceKind::Simple, SpaceKind::Simple, SpaceKind::Aya]
    );
    // overfull: tighten, then squeeze, as for every font (D15)
    let tight = justify_line(&sh, &Regexps::new(), &input, natural - 200 * ONE, params).unwrap();
    assert!(tight.width <= natural);
}
