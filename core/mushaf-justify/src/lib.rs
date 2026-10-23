//! Calligraphic justification (T3.2; D3, D10, D15).
//!
//! A port of DigitalKhatt's "experimental" justifier for the Madina font (`just.service.ts`,
//! MIT), following its Python port `src/db/v2/justify.py` in quran-madina-html @ d520c5c,
//! which serves as the executable oracle (`tests/golden/justify/oracle-1421h.json`). On a line
//! narrower than its target, spaces grow first (up to `space_stretch` times their normal width),
//! then letters stretch through the font's `cv01` to `cv18` features in DigitalKhatt's priority
//! order, and the spaces take what is left. Function and step names follow the original.
//!
//! The overflow path (a line wider than its target) is ours (D15), not the oracle's: spaces
//! tighten to `min_space` of normal, then the line is squeezed horizontally (`x_scale`).
//!
//! All arithmetic is integer: widths in 26.6 (1/64 font unit), scales in 16.16 (D10). Every
//! space gets an exact width, so a justified line closes to its target exactly.
//!
//! Letter sets are written as `\u{...}` escapes, copied mechanically from the oracle source.

mod tables;

use std::collections::BTreeMap;

use mushaf_model::Error;
use mushaf_shape::{FeatureRange, Shaper};
use regex::Regex;
use serde::Serialize;

use tables::*;

/// One unit in 26.6.
pub const ONE: i64 = 64;
/// 1.0 in 16.16.
pub const SCALE_ONE: i64 = 1 << 16;

pub type Tag = [u8; 4];
/// Feature values per character index of the line text.
pub type Features = BTreeMap<usize, BTreeMap<Tag, u32>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    /// How far spaces grow, in 1/1000 of their normal width, before letters stretch
    /// (DigitalKhatt's reference: 1000).
    pub space_stretch: i64,
    /// Narrowest simple space on an overfull line, in 1/1000 of normal.
    pub min_space: i64,
    /// Whether letters stretch (fonts with DigitalKhatt's `cvNN` features). When false, the
    /// spaces alone take the difference (spacing-only justification, D2, T5.3).
    pub letters: bool,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            space_stretch: 1000,
            min_space: 600,
            letters: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceKind {
    /// Next to an ayah number.
    Aya,
    Simple,
}

/// A line to justify: the text in the font's encoding, words separated by U+0020.
#[derive(Clone, Debug, Default)]
pub struct LineInput {
    pub text: Vec<char>,
    /// Characters the automatic justifier must not touch (manual overrides, D15).
    pub locked: Vec<bool>,
    /// Features set by hand; kept as they are.
    pub manual: Features,
    /// Extra advance per word (26.6), e.g. from `width_em_delta`.
    pub extra: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Justified {
    /// Stretch features per character of the line (manual ones included).
    pub features: Features,
    /// Width of each space, in reading order (26.6).
    pub spaces: Vec<i64>,
    pub space_kinds: Vec<SpaceKind>,
    /// Width of each word with its features and extra advance (26.6).
    pub words: Vec<i64>,
    /// Natural width: words as they are, spaces at normal width (26.6).
    pub natural: i64,
    /// Sum of words and spaces before `x_scale` (26.6).
    pub width: i64,
    /// Horizontal squeeze of an overfull line (16.16; `SCALE_ONE` = none).
    pub x_scale: i64,
    /// Width gained by stretching letters (26.6).
    pub letter_gain: i64,
}

#[derive(Clone, Debug, Default)]
struct SubWord {
    base_text: String,
    base_indexes: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Word {
    start: usize,
    end: usize, // inclusive, as in the original
    text: Vec<char>,
    subwords: Vec<SubWord>,
}

struct LineInfo {
    words: Vec<Word>,
    spaces: Vec<(usize, SpaceKind)>,
}

/// KFGQPC ayah signs: U+FC00 + (n - 1), drawn by KFGQPC fonts only (G-013).
fn is_kfgqpc_ayah_sign(c: char) -> bool {
    ('\u{FC00}'..='\u{FDFF}').contains(&c)
}

fn is_aya_space(text: &[char], i: usize) -> bool {
    let prev = if i > 0 { Some(text[i - 1]) } else { None };
    let next = text.get(i + 1).copied();
    prev.is_some_and(|c| ('\u{0660}'..='\u{0669}').contains(&c) || is_kfgqpc_ayah_sign(c))
        || next.is_some_and(|c| c == AYAH_END || is_kfgqpc_ayah_sign(c))
}

/// `analyzeLineForJust`: words, sub-words of joined base letters, and space kinds.
fn analyze_line(text: &[char]) -> LineInfo {
    let new_word = |start: usize| Word {
        start,
        end: start.wrapping_sub(1),
        text: Vec::new(),
        subwords: vec![SubWord::default()],
    };
    let mut words = vec![new_word(0)];
    let mut spaces = Vec::new();
    for (i, &ch) in text.iter().enumerate() {
        if ch == ' ' {
            let kind = if is_aya_space(text, i) {
                SpaceKind::Aya
            } else {
                SpaceKind::Simple
            };
            spaces.push((i, kind));
            words.push(new_word(i + 1));
            continue;
        }
        let word = words.last_mut().expect("at least one word");
        word.text.push(ch);
        if BASES.contains(ch) {
            let rel = i - word.start;
            let is_hamza = ch == HAMZA;
            if is_hamza {
                word.subwords.push(SubWord::default());
            }
            let sub = word.subwords.last_mut().expect("at least one sub-word");
            sub.base_text.push(ch);
            sub.base_indexes.push(rel);
            if i < text.len() - 1 && RIGHT_NO_JOIN.contains(ch) && !is_hamza {
                word.subwords.push(SubWord::default());
            }
        }
        word.end = word.end.wrapping_add(1);
    }
    LineInfo { words, spaces }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Applied {
    NoChange,
    Positive,
    Overflow,
    Forbidden,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
enum Stretch {
    Beh,
    FinaAscendant,
    Other,
    Kaf,
    SecondNotSame,
    SecondSame,
}

fn word_features(feats: &Features, w: &Word) -> Vec<FeatureRange> {
    let mut out = Vec::new();
    for (&idx, fs) in feats.range(w.start..=w.end.max(w.start)) {
        if idx > w.end {
            continue;
        }
        for (&tag, &value) in fs {
            let rel = u32::try_from(idx - w.start).expect("index fits");
            out.push(FeatureRange {
                tag,
                value,
                start: rel,
                end: rel + 1,
            });
        }
    }
    out
}

struct Justifier<'a, 's> {
    shaper: &'a Shaper<'s>,
    text: &'a [char],
    locked: &'a [bool],
    line: &'a LineInfo,
    features: Features,
    text_width: i64,
    desired: i64,
    word_widths: Vec<i64>,
    extra: &'a [i64],
    applied: Vec<BTreeMap<Stretch, (usize, usize)>>,
    regexps: &'a Regexps,
}

impl Justifier<'_, '_> {
    fn word_width(&self, wi: usize, feats: &Features) -> Result<i64, Error> {
        let w = &self.line.words[wi];
        let fr = word_features(feats, w);
        Ok(i64::from(self.shaper.width(&w.text, &fr)?) * ONE + self.extra[wi])
    }

    fn try_apply(&mut self, wi: usize, new_feats: Features) -> Result<Applied, Error> {
        let new_w = self.word_width(wi, &new_feats)?;
        let diff = new_w - self.word_widths[wi];
        if new_w != self.word_widths[wi] && self.text_width + diff < self.desired {
            self.text_width += diff;
            self.word_widths[wi] = new_w;
            self.features = new_feats;
            return Ok(Applied::Positive);
        }
        Ok(if diff == 0 {
            Applied::NoChange
        } else {
            Applied::Overflow
        })
    }

    fn fatha_index(&self, i: usize) -> Option<usize> {
        let t = self.text;
        if t.get(i + 1) == Some(&FATHA) {
            return Some(i + 1);
        }
        if t.get(i + 1) == Some(&SHADDA) && t.get(i + 2) == Some(&FATHA) {
            return Some(i + 2);
        }
        None
    }

    fn is_locked(&self, i: usize) -> bool {
        self.locked.get(i).copied().unwrap_or(false)
    }

    /// `applyKashida`: stretch the connection between two letters of a sub-word.
    fn apply_kashida(
        &mut self,
        wi: usize,
        si: usize,
        first: usize,
        second: usize,
    ) -> Result<Applied, Error> {
        let word = &self.line.words[wi];
        let sub = &word.subwords[si];
        let (first_m, second_m) = (sub.base_indexes[first], sub.base_indexes[second]);
        let (i1, i2) = (word.start + first_m, word.start + second_m);
        if self.is_locked(i1) || self.is_locked(i2) {
            return Ok(Applied::Forbidden);
        }
        let mut temp = self.features.clone();
        if temp.get(&i2).is_some_and(|f| f.contains_key(b"cv01")) {
            return Ok(Applied::Forbidden);
        }
        let (c3, c4) = (self.text[i1], self.text[i2]);
        let last_base = sub.base_indexes.last() == Some(&second_m);
        let first_base = sub.base_indexes.first() == Some(&first_m);
        if QAF.contains(c4) && last_base {
            return Ok(Applied::Forbidden);
        }
        if LAM.contains(c3)
            && (KAF_DAL_THAL_TEH_MARBUTA.contains(c4) || (HEH.contains(c4) && last_base))
        {
            return Ok(Applied::Forbidden);
        }
        if YEH_BEH_TEH_THEH_NOON_YEH_MAKSURA.contains(c3) && !first_base && REH_ZAIN.contains(c4) {
            return Ok(Applied::Forbidden);
        }
        let mut first_feats = temp.get(&i1).cloned().unwrap_or_default();
        let cv01 = (first_feats.get(b"cv01").copied().unwrap_or(0) + 1).min(6);
        first_feats.insert(*b"cv01", cv01);
        let mut second_feats: BTreeMap<Tag, u32> = BTreeMap::new();
        if BEH_LIKE.contains(c3) {
            first_feats.insert(*b"cv10", 1);
        }
        let mut both = |tag: &Tag| {
            first_feats.insert(*tag, 1);
            second_feats.insert(*tag, 1);
        };
        let bt: Vec<char> = sub.base_text.chars().collect();
        if HEH.contains(c3) && MEEM.contains(c4) && last_base {
            both(b"cv11");
        } else if BEH_LIKE.contains(c3) && first_base && HAH_LIKE.contains(c4) {
            both(b"cv12");
        } else if MEEM.contains(c3) && first_base && HAH_LIKE.contains(c4) {
            both(b"cv13");
        } else if FEH_QAF.contains(c3) && first_base && HAH_LIKE.contains(c4) {
            both(b"cv14");
        } else if LAM.contains(c3) && first_base && HAH_LIKE.contains(c4) {
            both(b"cv15");
        } else if AIN_GHAIN.contains(c3)
            && first_base
            && (ALEF_DAL_LAM.contains(c4)
                || (BEH_LIKE.contains(c4) && bt.len() > 2 && SEEN_SHEEN.contains(bt[2])))
        {
            both(b"cv16");
        } else if HAH_LIKE.contains(c3) {
            let n = sub.base_indexes.len();
            if ALEF_DAL_LAM.contains(c4)
                || (HEH_TEH_MARBUTA.contains(c4) && last_base)
                || (BEH_LIKE.contains(c4)
                    && n > 1
                    && sub.base_indexes[n - 2] == second_m
                    && bt.last().is_some_and(|&c| REH_ZAIN_NOON.contains(c)))
            {
                both(b"cv16");
            } else if first_base && MEEM.contains(c4) {
                both(b"cv18");
            }
        } else if SEEN_SAD.contains(c3) && REH_ZAIN.contains(c4) {
            both(b"cv17");
        }
        let cv02 = if FINAL_ASCENDANT.contains(c4) && last_base {
            cv01
        } else {
            2 * cv01
        };
        second_feats.insert(*b"cv02", cv02);
        temp.insert(i1, first_feats);
        temp.insert(i2, second_feats);
        self.try_apply(wi, temp)
    }

    /// `applyKaf`: the wide kaf form with its following letter.
    fn apply_kaf(
        &mut self,
        wi: usize,
        si: usize,
        first: usize,
        second: usize,
    ) -> Result<Applied, Error> {
        let word = &self.line.words[wi];
        let sub = &word.subwords[si];
        let i1 = word.start + sub.base_indexes[first];
        let i2 = word.start + sub.base_indexes[second];
        if self.is_locked(i1) || self.is_locked(i2) {
            return Ok(Applied::Forbidden);
        }
        let mut temp = self.features.clone();
        temp.entry(i1).or_default().insert(*b"cv03", 1);
        temp.entry(i2).or_default().insert(*b"cv03", 1);
        if let Some(fatha) = self.fatha_index(i1) {
            let v = 1 + temp[&i2].get(b"cv01").copied().unwrap_or(0) / 3;
            temp.insert(fatha, BTreeMap::from([(*b"cv01", v)]));
        }
        self.try_apply(wi, temp)
    }

    /// `applyAlternate`: widen the final or isolated letter at `idx` one level.
    fn apply_alternate(&mut self, wi: usize, idx: usize) -> Result<Applied, Error> {
        if self.is_locked(idx) {
            return Ok(Applied::Forbidden);
        }
        let mut temp = self.features.clone();
        let prev = temp.get(&idx).cloned().unwrap_or_default();
        if prev.get(b"cv02").copied().unwrap_or(0) > 0 {
            return Ok(Applied::Forbidden);
        }
        let mut new = prev;
        let cv01 = (new.get(b"cv01").copied().unwrap_or(0) + 1).min(12);
        new.insert(*b"cv01", cv01);
        temp.insert(idx, new);
        if let Some(fatha) = self.fatha_index(idx) {
            temp.insert(fatha, BTreeMap::from([(*b"cv01", 1 + cv01 / 3)]));
        }
        self.try_apply(wi, temp)
    }

    /// Per word: the sub-words with matches, and each sub-word's matches (start of group 1,
    /// as a character index in the sub-word's base text), in regex order then match order.
    fn match_subwords(word: &Word, regexps: &[Regex]) -> (Vec<usize>, Vec<Vec<usize>>) {
        let mut sub_idx = Vec::new();
        let mut matches = Vec::new();
        for (si, sub) in word.subwords.iter().enumerate() {
            let mut found = Vec::new();
            for rx in regexps {
                for caps in rx.captures_iter(&sub.base_text) {
                    let g = caps.get(1).expect("group 1 always takes part");
                    found.push(sub.base_text[..g.start()].chars().count());
                }
            }
            if !found.is_empty() {
                sub_idx.push(si);
            }
            matches.push(found);
        }
        (sub_idx, matches)
    }

    /// Kashidas of one stretch type, level by level. True on overflow.
    fn kashidas(&mut self, stype: Stretch, levels: usize) -> Result<bool, Error> {
        let rx = self.regexps.kashida(stype);
        let found: Vec<_> = self
            .line
            .words
            .iter()
            .map(|w| Self::match_subwords(w, rx))
            .collect();
        for _ in 0..levels {
            for (wi, (sub_idx, matches)) in found.iter().enumerate() {
                let applied = &self.applied[wi];
                let (t1, t2, t3) = (
                    applied.get(&Stretch::Beh).copied(),
                    applied.get(&Stretch::FinaAscendant).copied(),
                    applied.get(&Stretch::Other).copied(),
                );
                let t5 = applied.get(&Stretch::SecondNotSame).copied();
                let skip = match stype {
                    Stretch::Beh => t2.is_some() || t3.is_some(),
                    Stretch::FinaAscendant => t1.is_some() || t3.is_some(),
                    Stretch::Other => t1.is_some() || t2.is_some(),
                    _ => false,
                };
                if skip {
                    continue;
                }
                let t123 = t1.or(t2).or(t3);
                let mut done = false;
                for &si in sub_idx.iter().rev() {
                    if done {
                        break;
                    }
                    for &first in &matches[si] {
                        if stype == Stretch::SecondNotSame && t123.is_some_and(|t| t.0 == si) {
                            continue;
                        }
                        if stype == Stretch::SecondSame
                            && (t123 == Some((si, first)) || t5 == Some((si, first)))
                        {
                            continue;
                        }
                        let result = if stype == Stretch::Kaf {
                            self.apply_kaf(wi, si, first, first + 1)?
                        } else {
                            self.apply_kashida(wi, si, first, first + 1)?
                        };
                        match result {
                            Applied::Positive => {
                                self.applied[wi].insert(stype, (si, first));
                            }
                            Applied::Overflow => return Ok(true),
                            Applied::Forbidden => continue,
                            Applied::NoChange => {}
                        }
                        done = true;
                        break;
                    }
                }
            }
        }
        Ok(false)
    }

    /// Widen final letters of `chars`, level by level. True on overflow.
    fn alternates(&mut self, rx: &Regex, levels: usize) -> Result<bool, Error> {
        let found: Vec<_> = self
            .line
            .words
            .iter()
            .map(|w| Self::match_subwords(w, std::slice::from_ref(rx)))
            .collect();
        for _ in 0..levels {
            for (wi, (sub_idx, matches)) in found.iter().enumerate() {
                let word = &self.line.words[wi];
                for &si in sub_idx.iter().rev() {
                    let m = matches[si][0];
                    let idx = word.start + word.subwords[si].base_indexes[m];
                    match self.apply_alternate(wi, idx)? {
                        Applied::Overflow => return Ok(true),
                        Applied::Forbidden => continue,
                        Applied::Positive | Applied::NoChange => break,
                    }
                }
            }
        }
        Ok(false)
    }

    /// The priority order of `applyExperimentalJust` (Madina layouts).
    fn run(&mut self) -> Result<(), Error> {
        let r = self.regexps;
        let steps: [(Option<Stretch>, Option<&Regex>, usize); 19] = [
            (Some(Stretch::Beh), None, 2),
            (None, Some(&r.alt_beh_kaf_noon), 2),
            (Some(Stretch::FinaAscendant), None, 3),
            (Some(Stretch::Other), None, 2),
            (None, Some(&r.alt_yeh_sad_seen_feh), 2),
            (Some(Stretch::Kaf), None, 1),
            (Some(Stretch::Beh), None, 1),
            (None, Some(&r.alt_beh_kaf_noon), 1),
            (Some(Stretch::FinaAscendant), None, 1),
            (Some(Stretch::Other), None, 1),
            (None, Some(&r.alt_yeh_sad_seen_feh), 1),
            (None, Some(&r.alt_beh_kaf_noon), 2),
            (None, Some(&r.alt_all), 2),
            (Some(Stretch::Beh), None, 1),
            (Some(Stretch::FinaAscendant), None, 1),
            (Some(Stretch::Other), None, 1),
            (None, Some(&r.alt_all), 2),
            (Some(Stretch::SecondNotSame), None, 2),
            (Some(Stretch::SecondSame), None, 2),
        ];
        for (stype, alt, levels) in steps {
            let overflow = match (stype, alt) {
                (Some(s), _) => self.kashidas(s, levels)?,
                (None, Some(rx)) => self.alternates(rx, levels)?,
                (None, None) => unreachable!(),
            };
            if overflow {
                break;
            }
        }
        Ok(())
    }
}

/// The compiled patterns (built once per process).
pub struct Regexps {
    beh: Vec<Regex>,
    fina: Vec<Regex>,
    other: Vec<Regex>,
    kaf: Vec<Regex>,
    second_not_same: Vec<Regex>,
    second_same: Vec<Regex>,
    alt_beh_kaf_noon: Regex,
    alt_yeh_sad_seen_feh: Regex,
    alt_all: Regex,
}

impl Regexps {
    pub fn new() -> Self {
        let rx = |p: &str| Regex::new(p).expect("valid pattern");
        let alt = |chars: &str| rx(&format!("^.*([{chars}])$"));
        Regexps {
            beh: vec![rx(RX_BEH)],
            fina: vec![rx(&rx_fina())],
            other: vec![rx(&rx_reh()), rx(&rx_other())],
            kaf: vec![rx(RX_KAF)],
            second_not_same: vec![rx(RX_BEH), rx(&rx_fina()), rx(&rx_reh()), rx(&rx_other())],
            second_same: vec![
                rx(RX_BEH),
                rx(&rx_fina_unanchored()),
                rx(&rx_reh_bare()),
                rx(&rx_other_bare()),
            ],
            alt_beh_kaf_noon: alt(ALT_BEH_KAF_NOON.0),
            alt_yeh_sad_seen_feh: alt(ALT_YEH_SAD_SEEN_FEH.0),
            alt_all: alt(&format!("{}{}", ALT_YEH_SAD_SEEN_FEH.0, ALT_BEH_KAF_NOON.0)),
        }
    }

    fn kashida(&self, s: Stretch) -> &[Regex] {
        match s {
            Stretch::Beh => &self.beh,
            Stretch::FinaAscendant => &self.fina,
            Stretch::Other => &self.other,
            Stretch::Kaf => &self.kaf,
            Stretch::SecondNotSame => &self.second_not_same,
            Stretch::SecondSame => &self.second_same,
        }
    }
}

impl Default for Regexps {
    fn default() -> Self {
        Self::new()
    }
}

/// Width of one space in 26.6.
pub fn space_width(shaper: &Shaper) -> Result<i64, Error> {
    Ok(i64::from(shaper.width(&[' '], &[])?) * ONE)
}

/// Spread `total` over the spaces: each gets its base width, and the rounding remainder goes
/// one 1/64 unit at a time to the first spaces in reading order.
fn close(base: &[i64], total: i64) -> Vec<i64> {
    let mut out = base.to_vec();
    let n = i64::try_from(out.len()).expect("few spaces");
    if n == 0 {
        return out;
    }
    let rem = total - out.iter().sum::<i64>();
    let (each, mut left) = (rem.div_euclid(n), rem.rem_euclid(n));
    for s in &mut out {
        *s += each;
        if left > 0 {
            *s += 1;
            left -= 1;
        }
    }
    out
}

/// Justify one line to `target` (26.6). Deterministic and pure.
pub fn justify_line(
    shaper: &Shaper,
    regexps: &Regexps,
    input: &LineInput,
    target: i64,
    params: Params,
) -> Result<Justified, Error> {
    let line = analyze_line(&input.text);
    let n_words = line.words.len();
    let extra: Vec<i64> = (0..n_words)
        .map(|i| input.extra.get(i).copied().unwrap_or(0))
        .collect();
    let space_w = space_width(shaper)?;
    let kinds: Vec<SpaceKind> = line.spaces.iter().map(|s| s.1).collect();
    let n_aya = i64::try_from(kinds.iter().filter(|k| **k == SpaceKind::Aya).count()).expect("few");
    let n_simple = i64::try_from(kinds.len()).expect("few") - n_aya;
    let aya_w = space_w * 3 / 2; // AYA_SPACE = 1.5 normal spaces
    let mut words = Vec::with_capacity(n_words);
    for (wi, w) in line.words.iter().enumerate() {
        let fr = word_features(&input.manual, w);
        words.push(i64::from(shaper.width(&w.text, &fr)?) * ONE + extra[wi]);
    }
    let natural = words.iter().sum::<i64>() + space_w * n_simple + aya_w * n_aya;
    let base_spaces = |simple: i64, aya: i64| -> Vec<i64> {
        kinds
            .iter()
            .map(|k| if *k == SpaceKind::Aya { aya } else { simple })
            .collect()
    };
    if target > natural {
        let max_simple = (100 * ONE).min(space_w) * params.space_stretch / 1000;
        let max_aya = (200 * ONE).min(2 * space_w) * params.space_stretch / 1000;
        let max_stretch = max_simple * n_simple + max_aya * n_aya;
        let stretch = (target - natural).min(max_stretch);
        let (mut simple, mut aya) = (space_w, aya_w);
        if max_stretch > 0 {
            simple += stretch * max_simple / max_stretch;
            aya += stretch * max_aya / max_stretch;
        }
        let mut current = natural + stretch;
        let mut features = input.manual.clone();
        let mut letter_gain = 0;
        if target > current && params.letters {
            let mut j = Justifier {
                shaper,
                text: &input.text,
                locked: &input.locked,
                line: &line,
                features: input.manual.clone(),
                text_width: current,
                desired: target,
                word_widths: words.clone(),
                extra: &extra,
                applied: vec![BTreeMap::new(); n_words],
                regexps,
            };
            j.run()?;
            letter_gain = j.text_width - current;
            current = j.text_width;
            features = j.features;
            words = j.word_widths;
        }
        let word_sum: i64 = words.iter().sum();
        let spaces = if kinds.is_empty() {
            Vec::new()
        } else {
            let n = i64::try_from(kinds.len()).expect("few");
            let fill = (target - current).max(0) / n;
            close(&base_spaces(simple + fill, aya + fill), target - word_sum)
        };
        let width = word_sum + spaces.iter().sum::<i64>();
        return Ok(Justified {
            features,
            spaces,
            space_kinds: kinds,
            words,
            natural,
            width,
            x_scale: SCALE_ONE,
            letter_gain,
        });
    }
    // Overfull (D15): simple spaces tighten, aya spaces keep their width, then squeeze.
    let mut simple = space_w;
    let mut current = natural;
    if n_simple > 0 {
        // ceiling: the spaces absorb the whole excess when they can (close() gives back the rest)
        let per_space = (natural - target + n_simple - 1) / n_simple;
        simple = (space_w * params.min_space / 1000).max(space_w - per_space);
        current -= (space_w - simple) * n_simple;
    }
    let word_sum: i64 = words.iter().sum();
    let spaces = if current <= target && !kinds.is_empty() {
        // tightening overshot (floors): give the rest back so the line closes exactly
        close(&base_spaces(simple, aya_w), target - word_sum)
    } else {
        base_spaces(simple, aya_w)
    };
    let width = word_sum + spaces.iter().sum::<i64>();
    let x_scale = if width > target {
        target * SCALE_ONE / width
    } else {
        SCALE_ONE
    };
    Ok(Justified {
        features: input.manual.clone(),
        spaces,
        space_kinds: kinds,
        words,
        natural,
        width,
        x_scale,
        letter_gain: 0,
    })
}

/// A manual stretch request (attribute `stretch`, D15) realised with the font's own features.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ManualStretch {
    /// Features to add to `LineInput::manual`; their characters must be locked.
    pub features: Features,
    /// Width gained (26.6).
    pub achieved: i64,
    /// False when the font's largest stretch on that letter stays below the request.
    pub reached: bool,
}

/// Stretch the letter at line character `idx` by at least `requested` (26.6) using the
/// smallest feature level that reaches it: a kashida towards the next joined letter of its
/// sub-word (`cv01` on the letter, `cv02` on the next, as `applyKashida`), or, for the last
/// letter of a sub-word, its wider final form (`cv01`, as `applyAlternate`). Levels go up to the
/// justifier's own limits (6 and 12); if none reaches the request the largest is returned with
/// `reached: false`, never dropped.
pub fn manual_stretch(
    shaper: &Shaper,
    text: &[char],
    idx: usize,
    requested: i64,
) -> Result<ManualStretch, Error> {
    let line = analyze_line(text);
    let not_a_letter = || Error::new(format!("stretch: character {idx} is not a base letter"));
    let (word, sub) = line
        .words
        .iter()
        .find_map(|w| {
            let rel = idx.checked_sub(w.start)?;
            w.subwords
                .iter()
                .find(|s| s.base_indexes.contains(&rel))
                .map(|s| (w, s))
        })
        .ok_or_else(not_a_letter)?;
    let rel = idx - word.start;
    let pos = sub
        .base_indexes
        .iter()
        .position(|&b| b == rel)
        .ok_or_else(not_a_letter)?;
    let next = sub.base_indexes.get(pos + 1).map(|&b| word.start + b);
    let base = i64::from(shaper.width(&word.text, &[])?) * ONE;
    let max_level = if next.is_some() { 6 } else { 12 };
    let mut best = ManualStretch {
        features: Features::new(),
        achieved: 0,
        reached: false,
    };
    for level in 1..=max_level {
        let mut feats = Features::new();
        feats.entry(idx).or_default().insert(*b"cv01", level);
        if let Some(n) = next {
            let last = sub.base_indexes.last() == Some(&(n - word.start));
            let v = if FINAL_ASCENDANT.contains(text[n]) && last {
                level
            } else {
                2 * level
            };
            feats.entry(n).or_default().insert(*b"cv02", v);
        }
        let w = i64::from(shaper.width(&word.text, &word_features(&feats, word))?) * ONE;
        if w - base > best.achieved {
            best = ManualStretch {
                features: feats,
                achieved: w - base,
                reached: false,
            };
        }
        if w - base >= requested {
            best.reached = true;
            break;
        }
    }
    Ok(best)
}

/// Natural width of a line (26.6), as `justify_line` measures it.
pub fn natural_width(shaper: &Shaper, input: &LineInput) -> Result<i64, Error> {
    let line = analyze_line(&input.text);
    let space_w = space_width(shaper)?;
    let mut total = 0;
    for (wi, w) in line.words.iter().enumerate() {
        let fr = word_features(&input.manual, w);
        total += i64::from(shaper.width(&w.text, &fr)?) * ONE
            + input.extra.get(wi).copied().unwrap_or(0);
    }
    for (_, k) in &line.spaces {
        total += if *k == SpaceKind::Aya {
            space_w * 3 / 2
        } else {
            space_w
        };
    }
    Ok(total)
}

/// Width of a line with simple spaces at their narrowest (`min_space`), aya spaces normal:
/// the widest a line can be without a horizontal squeeze (26.6).
pub fn tight_width(shaper: &Shaper, input: &LineInput, params: Params) -> Result<i64, Error> {
    let space_w = space_width(shaper)?;
    let simple = i64::try_from(
        analyze_line(&input.text)
            .spaces
            .iter()
            .filter(|s| s.1 == SpaceKind::Simple)
            .count(),
    )
    .expect("few spaces");
    let shrink = space_w - space_w * params.min_space / 1000;
    Ok(natural_width(shaper, input)? - shrink * simple)
}

/// Character ranges of the words of a line text (start..end, exclusive).
pub fn word_ranges(text: &[char]) -> Vec<std::ops::Range<usize>> {
    analyze_line(text)
        .words
        .iter()
        .map(|w| w.start..w.start + w.text.len())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_spreads_the_remainder_deterministically() {
        assert_eq!(close(&[10, 10, 10], 32), vec![11, 11, 10]);
        assert_eq!(close(&[10, 10], 18), vec![9, 9]);
        assert_eq!(close(&[], 5), Vec::<i64>::new());
    }

    #[test]
    fn analyze_marks_aya_spaces_and_words() {
        let text: Vec<char> = "ab \u{06DD}\u{0661} c".chars().collect();
        let line = analyze_line(&text);
        assert_eq!(line.words.len(), 3);
        assert_eq!(line.spaces, vec![(2, SpaceKind::Aya), (5, SpaceKind::Aya)]);
        assert_eq!((line.words[1].start, line.words[1].end), (3, 4));
    }
}
