//! Line layouts: per line, the first and last item it holds (T1.3, D6). A Ḥafṣ layout names
//! spine items; a riwāyah's native layout (T5.2, D29) names items of that riwāyah's resolved
//! text, inserted ones included, and says which riwāyah it belongs to.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Error, ItemRef};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum LineKind {
    Text { first: ItemRef, last: ItemRef },
    SurahName { surah: u32 },
    Basmalah,
}

/// How a native layout's line break was found (T5.2, D29).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitTag {
    /// The source's ayah lines fix the break.
    Forced,
    /// The 1441H Ḥafṣ break at the same page and line, consistent with the ayah lines.
    Hafs,
    /// Chosen by width where neither applies.
    Fitted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "LineWire", into = "LineWire")]
pub struct Line {
    pub page: u32,
    pub line: u32,
    pub kind: LineKind,
    pub centered: bool,
    /// Provenance of a manual correction (e.g. an owner print check, G-011).
    pub correction: Option<String>,
    /// How the break before this line was found (native layouts, text lines).
    pub fit: Option<FitTag>,
}

/// The tracked JSON form of a line (`data/layouts/<layout_id>.json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LineWire {
    page: u32,
    line: u32,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    surah: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    first: Option<ItemRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last: Option<ItemRef>,
    centered: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    correction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fit: Option<FitTag>,
}

impl TryFrom<LineWire> for Line {
    type Error = Error;
    fn try_from(w: LineWire) -> Result<Self, Error> {
        let at = format!("line {}:{}", w.page, w.line);
        let kind = match (w.kind.as_str(), w.surah, w.first, w.last) {
            ("text", None, Some(first), Some(last)) => LineKind::Text { first, last },
            ("surah_name", Some(surah), None, None) => LineKind::SurahName { surah },
            ("basmalah", None, None, None) => LineKind::Basmalah,
            (k, ..) => return Err(Error::new(format!("{at}: invalid fields for type {k:?}"))),
        };
        if w.fit.is_some() && !matches!(kind, LineKind::Text { .. }) {
            return Err(Error::new(format!("{at}: only a text line has a fit tag")));
        }
        Ok(Line {
            page: w.page,
            line: w.line,
            kind,
            centered: w.centered,
            correction: w.correction,
            fit: w.fit,
        })
    }
}

impl Line {
    /// A line from the columns of the `lines` table in `mushaf.db`.
    #[allow(clippy::too_many_arguments)]
    pub fn from_columns(
        page: u32,
        line: u32,
        kind: &str,
        surah: Option<u32>,
        first: Option<&str>,
        last: Option<&str>,
        centered: bool,
        correction: Option<String>,
        fit: Option<&str>,
    ) -> Result<Line, Error> {
        let item = |s: Option<&str>| s.map(str::parse::<ItemRef>).transpose();
        let fit = fit
            .map(|f| {
                serde_json::from_value(serde_json::Value::String(f.into()))
                    .map_err(|e| Error::new(format!("fit {f:?}: {e}")))
            })
            .transpose()?;
        Line::try_from(LineWire {
            page,
            line,
            kind: kind.into(),
            surah,
            first: item(first)?,
            last: item(last)?,
            centered,
            correction,
            fit,
        })
    }
}

impl From<Line> for LineWire {
    fn from(l: Line) -> Self {
        let (kind, surah, first, last) = match l.kind {
            LineKind::Text { first, last } => ("text", None, Some(first), Some(last)),
            LineKind::SurahName { surah } => ("surah_name", Some(surah), None, None),
            LineKind::Basmalah => ("basmalah", None, None, None),
        };
        LineWire {
            page: l.page,
            line: l.line,
            kind: kind.into(),
            surah,
            first,
            last,
            centered: l.centered,
            correction: l.correction,
            fit: l.fit,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub layout_id: String,
    pub script_id: String,
    /// The spine id the anchors refer to, e.g. `hafs/v1`.
    pub spine: String,
    /// A native layout's riwāyah: its anchors are items of that riwāyah's resolved text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub riwayah: Option<String>,
    pub pages: u32,
    pub lines_per_page: u32,
    pub sources: BTreeMap<String, String>,
    pub lines: Vec<Line>,
}

impl Layout {
    /// Checks that lines are ordered by (page, line) without duplicates, and that a layout
    /// without a riwāyah names only spine items.
    pub fn validate(&self) -> Result<(), Error> {
        if self.riwayah.is_none() {
            for l in &self.lines {
                if let LineKind::Text { first, last } = &l.kind
                    && !(first.is_spine() && last.is_spine())
                {
                    return Err(Error::new(format!(
                        "layout {}: line {}:{} names an inserted item but the layout has no riwayah",
                        self.layout_id, l.page, l.line
                    )));
                }
            }
        }
        for w in self.lines.windows(2) {
            if (w[0].page, w[0].line) >= (w[1].page, w[1].line) {
                return Err(Error::new(format!(
                    "layout {}: line {}:{} is out of order",
                    self.layout_id, w[1].page, w[1].line
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_forms() {
        for s in [
            r#"{"page":1,"line":1,"type":"surah_name","surah":1,"centered":true}"#,
            r#"{"page":2,"line":2,"type":"basmalah","centered":true}"#,
            r#"{"page":1,"line":2,"type":"text","first":"1:1:1","last":"1:1:5","centered":false,"correction":"x"}"#,
            r#"{"page":1,"line":2,"type":"text","first":"1:1:1+ins.1","last":"1:1:5","centered":false,"fit":"hafs"}"#,
        ] {
            let l: Line = serde_json::from_str(s).unwrap();
            assert_eq!(serde_json::to_string(&l).unwrap(), s);
        }
        for s in [
            r#"{"page":1,"line":1,"type":"text","first":"1:1:1","centered":true}"#,
            r#"{"page":1,"line":1,"type":"basmalah","surah":1,"centered":true}"#,
            r#"{"page":1,"line":1,"type":"other","centered":true}"#,
            r#"{"page":1,"line":1,"type":"basmalah","centered":true,"fit":"forced"}"#,
            r#"{"page":1,"line":1,"type":"text","first":"1:1:1","last":"1:1:2","centered":true,"fit":"guessed"}"#,
        ] {
            assert!(serde_json::from_str::<Line>(s).is_err(), "accepted {s}");
        }
    }

    #[test]
    fn inserted_anchors_need_a_riwayah() {
        let line: Line = serde_json::from_str(
            r#"{"page":1,"line":1,"type":"text","first":"1:1:1+ins.1","last":"1:1:2","centered":true}"#,
        )
        .unwrap();
        let mut l = Layout {
            layout_id: "x".into(),
            script_id: "s".into(),
            spine: "t/v1".into(),
            riwayah: None,
            pages: 1,
            lines_per_page: 1,
            sources: BTreeMap::new(),
            lines: vec![line],
        };
        assert!(l.validate().is_err());
        l.riwayah = Some("r".into());
        assert!(l.validate().is_ok());
    }
}
