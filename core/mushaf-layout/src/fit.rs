//! Page fitting (T3.2; D15, G-020, G-021): from resolved, line-mapped items to shaped and
//! justified lines, in the font's units at the page's scale.
//!
//! - Every item's clusters are converted to the font's encoding (D14); a cluster with no row
//!   is an error. Words are separated by one space; the justifier tells ayah spaces apart.
//! - A full text line targets the profile's `line_width`; a short line (the 1441H print's own
//!   width, `line-widths.json`) targets that share of it and is centred.
//! - The page gets one font scale, `clamp(target / widest line at minimum spacing, floor, 1)`
//!   over its text lines, and every line is justified to `target / scale` in font units. A line
//!   still too wide is squeezed horizontally, and reported.
//! - Manual overrides (`width_em_delta`, `stretch`) are applied before justification and their
//!   letters are locked; an unreachable stretch is reported, never dropped.
//! - Surah-name and basmalah lines keep their natural width, centred.

use std::collections::BTreeMap;
use std::ops::Range;

use mushaf_justify::{
    Features, LineInput, ONE, Params, Regexps, SCALE_ONE, justify_line, manual_stretch,
    natural_width, space_width, tight_width, word_ranges,
};
use mushaf_model::{ClusterRef, Cps, Error, ItemRef, LineKind};
use mushaf_resolve::Resolved;
use mushaf_shape::{EncodingTable, FeatureRange, ShapedGlyph, Shaper};
use serde::{Deserialize, Serialize};

use crate::Mapping;

/// `data/render/<profile>.json`: which data and font a riwayah renders with, and the design
/// constants of its pages (D15).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub profile: String,
    pub riwayah: String,
    /// Display style (D36): `hijazi` or `maghribi`.
    pub style: String,
    pub script_id: String,
    pub layout_id: String,
    pub spine: String,
    pub font: String,
    /// The KFGQPC -> font encoding table; absent for a font that reads KFGQPC's encoding as
    /// stored (the identity table, T5.3).
    #[serde(default)]
    pub encoding: Option<String>,
    /// Owner-approved rows for cluster forms outside Ḥafṣ (D28), loaded next to `encoding`.
    #[serde(default)]
    pub encoding_extra: Option<String>,
    /// Style conversion rows (T6.7, D44) and the style of the riwāyah's text: the rows from
    /// `text_style` to this profile's `style` are applied before the encoding table. A
    /// Maghribi text in a Hijazi profile is also rewritten with the owner-approved
    /// conventions (D46, `mushaf_resolve::maghribi_to_hijazi`) when resolved.
    #[serde(default)]
    pub style_conversion: Option<String>,
    #[serde(default)]
    pub text_style: Option<String>,
    pub line_widths: String,
    pub surah_names: String,
    pub note: String,
    /// Source codepoints that mark a word for the sajdah overline (D18), drawn by the core
    /// because the font has no such mark.
    pub overline_marks: Vec<Cps>,
    pub geometry: Geometry,
    pub justify: JustifySettings,
}

/// Font units of the font.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Geometry {
    pub line_width: i64,
    pub line_pitch: i64,
    pub margin_x: i64,
    pub margin_top: i64,
    pub margin_bottom: i64,
    /// Smallest page scale, per mille.
    pub page_scale_floor: i64,
    /// The sajdah overline (D18): its thickness, and its gap above the line's tallest
    /// overlined word.
    pub overline_thickness: i64,
    pub overline_clearance: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JustifySettings {
    pub space_stretch: i64,
    pub min_space: i64,
    /// Letter stretching (DigitalKhatt fonts); `false` justifies by spacing only (D2, T5.3).
    #[serde(default = "letters_default")]
    pub letters: bool,
}

fn letters_default() -> bool {
    true
}

impl Profile {
    pub fn from_json(text: &str) -> Result<Self, Error> {
        mushaf_model::from_json(text)
    }

    pub fn params(&self) -> Params {
        Params {
            space_stretch: self.justify.space_stretch,
            min_space: self.justify.min_space,
            letters: self.justify.letters,
        }
    }
}

/// `data/render/<layout>.line-widths.json`: the print's width of each short line (ppm).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineWidths {
    pub short: BTreeMap<(u32, u32), i64>,
}

#[derive(Deserialize)]
struct LineWidthsWire {
    short_lines: Vec<ShortLine>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShortLine {
    page: u32,
    line: u32,
    width_ppm: i64,
}

impl LineWidths {
    pub fn from_json(text: &str) -> Result<Self, Error> {
        let wire: LineWidthsWire = mushaf_model::from_json(text)?;
        Ok(LineWidths {
            short: wire
                .short_lines
                .into_iter()
                .map(|s| ((s.page, s.line), s.width_ppm))
                .collect(),
        })
    }
}

/// `data/render/surah-names.json` (KFGQPC `sura_name_ar`, D16).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SurahNames {
    pub names: BTreeMap<u32, Vec<char>>,
}

#[derive(Deserialize)]
struct NamesWire {
    names: Vec<NameWire>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NameWire {
    surah: u32,
    name: Cps,
}

impl SurahNames {
    pub fn from_json(text: &str) -> Result<Self, Error> {
        let wire: NamesWire = mushaf_model::from_json(text)?;
        Ok(SurahNames {
            names: wire
                .names
                .into_iter()
                .map(|n| (n.surah, n.name.0))
                .collect(),
        })
    }
}

impl Profile {
    /// The profile's encoding table: the identity table without `encoding`, else the table
    /// plus any `encoding_extra` rows. `read` returns a repository file's text.
    fn with_style(
        &self,
        table: EncodingTable,
        read: &impl Fn(&str) -> Result<String, Error>,
    ) -> Result<EncodingTable, Error> {
        match (&self.style_conversion, &self.text_style) {
            (None, _) => Ok(table),
            (Some(path), Some(from)) => {
                table.with_style_conversion(&read(path)?, from, &self.style)
            }
            (Some(_), None) => Err(Error::new("style_conversion needs text_style")),
        }
    }

    pub fn encoding_table(
        &self,
        read: impl Fn(&str) -> Result<String, Error>,
    ) -> Result<EncodingTable, Error> {
        let Some(path) = &self.encoding else {
            if self.encoding_extra.is_some() {
                return Err(Error::new("encoding_extra without an encoding table"));
            }
            return self.with_style(EncodingTable::identity(&self.spine), &read);
        };
        let mut table = EncodingTable::from_json(&read(path)?)?;
        if let Some(extra) = &self.encoding_extra {
            table.extend_from_json(&read(extra)?)?;
        }
        self.with_style(table, &read)
    }
}

/// Where the basmalah of header lines is taken from: the words of the basmalah inside 27:30
/// (owner decision at the T5.3 checkpoint, D30). In Ḥafṣ they equal 1:1:1 to 1:1:4 codepoint for
/// codepoint; riwāyāt that do not number the basmalah of al-Fātiḥah keep 27:30 in their own
/// spelling.
pub const BASMALAH_ITEMS: [(u32, u32, u32); 4] =
    [(27, 30, 5), (27, 30, 6), (27, 30, 7), (27, 30, 8)];

/// The basmalah line's words: the riwāyah's own `BASMALAH_ITEMS` in its resolved text (data
/// mode, without edits), converted (D16, D30).
pub fn basmalah_words(resolved: &Resolved, table: &EncodingTable) -> Result<Vec<Vec<char>>, Error> {
    BASMALAH_ITEMS
        .iter()
        .map(|&(s, a, n)| {
            let id = ItemRef::spine(s, a, n);
            let item = resolved
                .items
                .iter()
                .find(|i| i.id == id)
                .ok_or_else(|| Error::new(format!("resolved text has no {id}")))?;
            let cl: Vec<&[char]> = item.clusters.iter().map(|c| c.cps.0.as_slice()).collect();
            table
                .convert(&cl)
                .map(|c| c.chars)
                .map_err(|e| Error::new(format!("{id}: no row for {}", e.cps)))
        })
        .collect()
}

/// Everything a page needs besides the resolved text.
pub struct FitData<'a> {
    pub profile: &'a Profile,
    pub table: &'a EncodingTable,
    pub widths: &'a LineWidths,
    pub names: &'a SurahNames,
    pub basmalah: &'a [Vec<char>],
    pub upem: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WordSpan {
    /// Source clusters of these characters (one, or several for a sequence row).
    pub clusters: Vec<ClusterRef>,
    pub chars: Range<usize>,
    pub lossy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FitWord {
    /// Index into `Resolved::items` (text lines only).
    pub item: Option<usize>,
    pub chars: Vec<char>,
    pub spans: Vec<WordSpan>,
    pub features: Vec<FeatureRange>,
    pub glyphs: Vec<ShapedGlyph>,
    /// Extra advance after a character (manual `width_em_delta`): (char index, 26.6).
    pub gaps: Vec<(usize, i64)>,
    /// Advance of the word with its features and gaps (26.6).
    pub width: i64,
    /// Draw the sajdah overline over this word (D18).
    pub overline: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum FitKind {
    Text,
    SurahName { surah: u32 },
    Basmalah,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FitLine {
    pub page: u32,
    pub line: u32,
    pub kind: FitKind,
    /// The print's width of a short line, ppm of a full line.
    pub short_ppm: Option<i64>,
    pub centered: bool,
    /// A full line at this page's scale (26.6 font units).
    pub box_width: i64,
    /// What the line was justified to (26.6); its natural width for header lines.
    pub target: i64,
    pub natural: i64,
    /// Words plus spaces before any squeeze (26.6).
    pub width: i64,
    pub spaces: Vec<i64>,
    /// Horizontal squeeze `target / width` of an overfull line.
    pub squeeze: Option<(i64, i64)>,
    pub letter_gain: i64,
    pub words: Vec<FitWord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum FitNote {
    /// The page's font scale is below 1 (16.16).
    PageScaled {
        scale: i64,
    },
    /// The page needed more than the floor allows; some lines are squeezed.
    PageScaleAtFloor {
        needed: i64,
    },
    /// An overfull line drawn with a horizontal squeeze (16.16).
    Squeezed {
        line: u32,
        x_scale: i64,
    },
    /// A word uses a lossy encoding row (G-040).
    Lossy {
        item: String,
    },
    StretchUnreachable {
        cluster: String,
        requested: i64,
        achieved: i64,
    },
    /// `stretch` on something that is not a base letter (or on a whole item).
    StretchNotOnLetter {
        target: String,
    },
    EmptyLine {
        line: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FitPage {
    pub page: u32,
    /// The page's font scale (16.16).
    pub scale: i64,
    /// False if the text was edited or resolution reported anything (rule 11).
    pub canonical: bool,
    pub lines: Vec<FitLine>,
    pub notes: Vec<FitNote>,
}

struct Plan {
    page: u32,
    line: u32,
    kind: FitKind,
    short_ppm: Option<i64>,
    centered: bool,
    words: Vec<FitWord>,
    input: LineInput,
    target: i64, // unscaled, 26.6
}

fn em(value: i32, upem: i64) -> i64 {
    i64::from(value) * upem * ONE / 1000
}

fn plain_word(chars: Vec<char>) -> FitWord {
    FitWord {
        item: None,
        spans: Vec::new(),
        chars,
        features: Vec::new(),
        glyphs: Vec::new(),
        gaps: Vec::new(),
        width: 0,
        overline: false,
    }
}

/// The words of a text line, its input for the justifier, and the override notes.
fn text_plan(
    shaper: &Shaper,
    data: &FitData,
    resolved: &Resolved,
    items: Range<usize>,
    notes: &mut Vec<FitNote>,
) -> Result<(Vec<FitWord>, LineInput), Error> {
    let mut words = Vec::new();
    let mut extra = Vec::new();
    let mut lock_ranges: Vec<Range<usize>> = Vec::new(); // word-relative, per word index
    let mut lock_words = Vec::new();
    let mut stretches = Vec::new(); // (word index, char in word, requested, cluster)
    for idx in items {
        let it = &resolved.items[idx];
        if it.clusters.is_empty() {
            continue;
        }
        let cl: Vec<&[char]> = it.clusters.iter().map(|c| c.cps.0.as_slice()).collect();
        let conv = data
            .table
            .convert_item(&it.id.to_string(), &cl)
            .map_err(|e| Error::new(format!("{}: no encoding row for cluster {}", it.id, e.cps)))?;
        let wi = words.len();
        let mut word = plain_word(conv.chars);
        word.item = Some(idx);
        word.overline = cl.iter().any(|c| {
            data.profile
                .overline_marks
                .iter()
                .any(|m| c.windows(m.0.len()).any(|w| w == m.0.as_slice()))
        });
        let mut word_extra = 0;
        for s in &conv.spans {
            word.spans.push(WordSpan {
                clusters: it.clusters[s.clusters.clone()]
                    .iter()
                    .map(|c| c.id.clone())
                    .collect(),
                chars: s.chars.clone(),
                lossy: s.lossy,
            });
            for c in &it.clusters[s.clusters.clone()] {
                if let Some(d) = c.attrs.width_em_delta {
                    let w = em(d, data.upem);
                    word.gaps.push((s.chars.end - 1, w));
                    word_extra += w;
                }
                if let Some(st) = c.attrs.stretch {
                    stretches.push((wi, s.chars.start, em(st, data.upem), c.id.to_string()));
                }
                if c.attrs.width_em_delta.is_some() || c.attrs.stretch.is_some() {
                    lock_words.push(wi);
                    lock_ranges.push(s.chars.clone());
                }
            }
        }
        if conv.spans.iter().any(|s| s.lossy) {
            notes.push(FitNote::Lossy {
                item: it.id.to_string(),
            });
        }
        if let Some(d) = it.attrs.width_em_delta {
            let w = em(d, data.upem);
            word.gaps.push((word.chars.len() - 1, w));
            word_extra += w;
        }
        if it.attrs.stretch.is_some() {
            notes.push(FitNote::StretchNotOnLetter {
                target: it.id.to_string(),
            });
        }
        extra.push(word_extra);
        words.push(word);
    }
    let mut text = Vec::new();
    let mut starts = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            text.push(' ');
        }
        starts.push(text.len());
        text.extend_from_slice(&w.chars);
    }
    let mut input = LineInput {
        locked: vec![false; text.len()],
        text,
        manual: Features::new(),
        extra,
    };
    for (wi, r) in lock_words.iter().zip(&lock_ranges) {
        for flag in &mut input.locked[starts[*wi] + r.start..starts[*wi] + r.end] {
            *flag = true;
        }
    }
    for (wi, ch, requested, cluster) in stretches {
        match manual_stretch(shaper, &input.text, starts[wi] + ch, requested) {
            Ok(ms) => {
                for (i, fs) in ms.features {
                    input.locked[i] = true;
                    input.manual.entry(i).or_default().extend(fs);
                }
                if !ms.reached {
                    notes.push(FitNote::StretchUnreachable {
                        cluster,
                        requested,
                        achieved: ms.achieved,
                    });
                }
            }
            Err(_) => notes.push(FitNote::StretchNotOnLetter { target: cluster }),
        }
    }
    Ok((words, input))
}

fn header_input(words: &[FitWord]) -> LineInput {
    let mut text = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            text.push(' ');
        }
        text.extend_from_slice(&w.chars);
    }
    LineInput {
        locked: vec![false; text.len()],
        text,
        ..LineInput::default()
    }
}

/// Fit one page. Deterministic and pure.
pub fn fit_page(
    shaper: &Shaper,
    regexps: &Regexps,
    data: &FitData,
    resolved: &Resolved,
    mapping: &Mapping,
    page: u32,
) -> Result<FitPage, Error> {
    let g = data.profile.geometry;
    let params = data.profile.params();
    let full = g.line_width * ONE;
    let mut notes = Vec::new();
    let mut plans = Vec::new();
    for ml in mapping.lines.iter().filter(|l| l.page == page) {
        let (kind, words, input) = match &ml.kind {
            LineKind::Text { .. } => {
                let (words, input) =
                    text_plan(shaper, data, resolved, ml.items.clone(), &mut notes)?;
                if words.is_empty() {
                    notes.push(FitNote::EmptyLine { line: ml.line });
                }
                (FitKind::Text, words, input)
            }
            LineKind::SurahName { surah } => {
                let name = data
                    .names
                    .names
                    .get(surah)
                    .ok_or_else(|| Error::new(format!("no name for surah {surah}")))?;
                let words: Vec<FitWord> = name
                    .split(|c| *c == ' ')
                    .map(|w| plain_word(w.to_vec()))
                    .collect();
                let input = header_input(&words);
                (FitKind::SurahName { surah: *surah }, words, input)
            }
            LineKind::Basmalah => {
                let words: Vec<FitWord> = data.basmalah.iter().cloned().map(plain_word).collect();
                let input = header_input(&words);
                (FitKind::Basmalah, words, input)
            }
        };
        let short_ppm = match kind {
            FitKind::Text => data.widths.short.get(&(ml.page, ml.line)).copied(),
            _ => None,
        };
        let target = short_ppm.map_or(full, |ppm| full * ppm / 1_000_000);
        plans.push(Plan {
            page: ml.page,
            line: ml.line,
            centered: short_ppm.is_some() || kind != FitKind::Text,
            kind,
            short_ppm,
            words,
            input,
            target,
        });
    }
    // One font scale for the page (G-021).
    let floor = g.page_scale_floor * SCALE_ONE / 1000;
    let mut needed = SCALE_ONE;
    for p in plans
        .iter()
        .filter(|p| p.kind == FitKind::Text && !p.words.is_empty())
    {
        let tight = tight_width(shaper, &p.input, params)?;
        if tight > 0 {
            needed = needed.min(p.target * SCALE_ONE / tight);
        }
    }
    let scale = needed.max(floor);
    if scale < SCALE_ONE {
        notes.push(FitNote::PageScaled { scale });
    }
    if needed < floor {
        notes.push(FitNote::PageScaleAtFloor { needed });
    }
    let box_width = full * SCALE_ONE / scale;
    let space_w = space_width(shaper)?;
    let mut lines = Vec::with_capacity(plans.len());
    for mut p in plans {
        let natural = natural_width(shaper, &p.input)?;
        let ranges = word_ranges(&p.input.text);
        let (target, width, spaces, squeeze, letter_gain, features) =
            if p.kind == FitKind::Text && !p.words.is_empty() {
                let target = p.target * SCALE_ONE / scale;
                let j = justify_line(shaper, regexps, &p.input, target, params)?;
                let squeeze = (j.x_scale < SCALE_ONE).then_some((target, j.width));
                if let Some(_sq) = squeeze {
                    notes.push(FitNote::Squeezed {
                        line: p.line,
                        x_scale: j.x_scale,
                    });
                }
                (
                    target,
                    j.width,
                    j.spaces,
                    squeeze,
                    j.letter_gain,
                    j.features,
                )
            } else {
                let spaces = vec![space_w; p.words.len().saturating_sub(1)];
                (natural, natural, spaces, None, 0, Features::new())
            };
        for (wi, w) in p.words.iter_mut().enumerate() {
            let r = ranges[wi].clone();
            w.features = features
                .range(r.clone())
                .flat_map(|(&i, fs)| {
                    let rel = u32::try_from(i - r.start).expect("index fits");
                    fs.iter().map(move |(&tag, &value)| FeatureRange {
                        tag,
                        value,
                        start: rel,
                        end: rel + 1,
                    })
                })
                .collect();
            w.glyphs = shaper.shape(&w.chars, &w.features)?;
            w.width = w.glyphs.iter().map(|g| i64::from(g.x_advance)).sum::<i64>() * ONE
                + w.gaps.iter().map(|g| g.1).sum::<i64>();
        }
        let words_sum: i64 = p.words.iter().map(|w| w.width).sum();
        if words_sum + spaces.iter().sum::<i64>() != width {
            return Err(Error::new(format!(
                "page {} line {}: words and spaces do not add up",
                p.page, p.line
            )));
        }
        lines.push(FitLine {
            page: p.page,
            line: p.line,
            kind: p.kind,
            short_ppm: p.short_ppm,
            centered: p.centered,
            box_width,
            target,
            natural,
            width,
            spaces,
            squeeze,
            letter_gain,
            words: p.words,
        });
    }
    notes.sort();
    notes.dedup();
    Ok(FitPage {
        page,
        scale,
        canonical: resolved.canonical(),
        lines,
        notes,
    })
}
