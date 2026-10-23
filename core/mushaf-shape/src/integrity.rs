//! Shaping integrity (rule 6, T3.5a): no silent rendering failure.
//!
//! For one shaped word: no `.notdef` (glyph 0); no dotted circle (U+25CC) the input did not
//! contain; every GDEF mark glyph placed by an anchor (its offsets change when the `mark` and
//! `mkmk` features are switched off: the test spike S2 used); and glyph clusters that are
//! valid, monotone character indices starting at 0, so every input character belongs to
//! exactly one glyph cluster.

use std::collections::{BTreeMap, BTreeSet};

use mushaf_model::Error;
use serde::{Deserialize, Serialize};

use crate::{FeatureRange, FontData, ShapedGlyph, Shaper};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Violation {
    Notdef {
        glyph_index: usize,
        cluster: u32,
    },
    DottedCircle {
        glyph_index: usize,
        cluster: u32,
    },
    UnattachedMark {
        glyph: u32,
        cluster: u32,
    },
    /// Clusters out of range, not monotone, or not starting at the first character.
    ClusterMap {
        detail: String,
    },
}

const DOTTED_CIRCLE: char = '\u{25CC}';
const MARK_CLASS: u16 = 3;

fn off(tag: &[u8; 4]) -> FeatureRange {
    FeatureRange {
        tag: *tag,
        value: 0,
        start: 0,
        end: u32::MAX,
    }
}

/// Check a word already shaped with `features` (`glyphs` is that output).
pub fn check_word(
    font: &FontData,
    shaper: &Shaper,
    text: &[char],
    features: &[FeatureRange],
    glyphs: &[ShapedGlyph],
) -> Result<Vec<Violation>, Error> {
    let mut out = Vec::new();
    let dotted = font.nominal_glyph(DOTTED_CIRCLE);
    let input_has_dotted = text.contains(&DOTTED_CIRCLE);
    for (i, g) in glyphs.iter().enumerate() {
        if g.glyph == 0 {
            out.push(Violation::Notdef {
                glyph_index: i,
                cluster: g.cluster,
            });
        }
        if !input_has_dotted && dotted == Some(g.glyph) {
            out.push(Violation::DottedCircle {
                glyph_index: i,
                cluster: g.cluster,
            });
        }
    }
    // RTL output is in visual order: clusters never increase, and the last glyph is cluster 0.
    let n = u32::try_from(text.len()).map_err(|_| Error::new("word too long"))?;
    let monotone = glyphs.windows(2).all(|w| w[0].cluster >= w[1].cluster);
    let in_range = glyphs.iter().all(|g| g.cluster < n);
    let starts = glyphs.last().is_some_and(|g| g.cluster == 0) || text.is_empty();
    if !(monotone && in_range && starts) {
        out.push(Violation::ClusterMap {
            detail: format!("monotone {monotone}, in range {in_range}, starts at 0 {starts}"),
        });
    }
    if glyphs
        .iter()
        .any(|g| font.glyph_class(g.glyph) == MARK_CLASS)
    {
        let mut no_marks = features.to_vec();
        no_marks.push(off(b"mark"));
        no_marks.push(off(b"mkmk"));
        let bare = shaper.shape(text, &no_marks)?;
        if bare.len() != glyphs.len() || bare.iter().zip(glyphs).any(|(a, b)| a.glyph != b.glyph) {
            out.push(Violation::ClusterMap {
                detail: "switching off mark/mkmk changed the glyphs".into(),
            });
        } else {
            for (a, b) in glyphs.iter().zip(&bare) {
                if font.glyph_class(a.glyph) == MARK_CLASS
                    && (a.x_offset, a.y_offset) == (b.x_offset, b.y_offset)
                {
                    out.push(Violation::UnattachedMark {
                        glyph: a.glyph,
                        cluster: a.cluster,
                    });
                }
            }
        }
    }
    Ok(out)
}

/// Owner-approved per-glyph rule-6 exceptions (D27, G-100):
/// `data/render/kfgqpc-rule6-exceptions.json`. A listed glyph of a listed font may be a mark
/// placed without an anchor attachment; every other check still applies to it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Exceptions {
    by_font: BTreeMap<String, BTreeSet<u32>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExceptionsWire {
    #[allow(dead_code)]
    note: String,
    fonts: BTreeMap<String, Vec<ExceptionWire>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExceptionWire {
    glyph: u32,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    date: String,
    #[allow(dead_code)]
    decision: String,
}

impl Exceptions {
    pub fn from_json(text: &str) -> Result<Self, Error> {
        let w: ExceptionsWire = mushaf_model::from_json(text)?;
        Ok(Exceptions {
            by_font: w
                .fonts
                .into_iter()
                .map(|(f, gs)| (f, gs.into_iter().map(|g| g.glyph).collect()))
                .collect(),
        })
    }

    /// Whether `v`, found in `font` (its repository path), is an approved exception.
    pub fn allows(&self, font: &str, v: &Violation) -> bool {
        matches!(v, Violation::UnattachedMark { glyph, .. }
            if self.by_font.get(font).is_some_and(|gs| gs.contains(glyph)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exceptions_cover_only_unattached_listed_glyphs_of_their_font() {
        let e = Exceptions::from_json(
            r#"{"note": "", "fonts": {"f.ttf": [{"glyph": 7, "name": "x", "date": "d", "decision": "y"}]}}"#,
        )
        .unwrap();
        let mark = |glyph| Violation::UnattachedMark { glyph, cluster: 0 };
        assert!(e.allows("f.ttf", &mark(7)));
        assert!(!e.allows("f.ttf", &mark(8)));
        assert!(!e.allows("g.ttf", &mark(7)));
        assert!(!e.allows(
            "f.ttf",
            &Violation::Notdef {
                glyph_index: 0,
                cluster: 0
            }
        ));
    }
}
