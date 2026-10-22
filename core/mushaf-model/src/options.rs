//! Option groups (wujūh), §3.3. Defaults are data from cited sources, never guessed: a group
//! with no default for the active riwāyah/ṭarīq and no user choice is an error (D12).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionDefault {
    pub riwayah: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tariq: Option<String>,
    pub choice: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "OptionGroupWire", into = "OptionGroupWire")]
pub struct OptionGroup {
    pub id: String,
    pub title_ar: String,
    pub choices: Vec<String>,
    pub defaults: Vec<OptionDefault>,
    /// Patch or rule ids this group switches; every patch with `when.option == id` must be listed.
    pub affects: Vec<String>,
    /// Advisory only (lint), never enforced (rule 11).
    pub allowed_for: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OptionGroupWire {
    id: String,
    title_ar: String,
    choices: Vec<String>,
    #[serde(default)]
    defaults: Vec<OptionDefault>,
    #[serde(default)]
    affects: Vec<String>,
    #[serde(default)]
    allowed_for: Vec<String>,
}

impl TryFrom<OptionGroupWire> for OptionGroup {
    type Error = Error;
    fn try_from(w: OptionGroupWire) -> Result<Self, Error> {
        let err = |m: String| Error::new(format!("option group {:?}: {m}", w.id));
        let choices: BTreeSet<&str> = w.choices.iter().map(String::as_str).collect();
        if w.choices.is_empty() || choices.len() != w.choices.len() {
            return Err(err("choices must be non-empty and unique".into()));
        }
        let mut keys = BTreeSet::new();
        for d in &w.defaults {
            if !choices.contains(d.choice.as_str()) {
                return Err(err(format!("default {:?} is not a choice", d.choice)));
            }
            if !keys.insert((d.riwayah.as_str(), d.tariq.as_deref())) {
                return Err(err(format!("two defaults for {} {:?}", d.riwayah, d.tariq)));
            }
        }
        Ok(OptionGroup {
            id: w.id,
            title_ar: w.title_ar,
            choices: w.choices,
            defaults: w.defaults,
            affects: w.affects,
            allowed_for: w.allowed_for,
        })
    }
}

impl From<OptionGroup> for OptionGroupWire {
    fn from(g: OptionGroup) -> Self {
        OptionGroupWire {
            id: g.id,
            title_ar: g.title_ar,
            choices: g.choices,
            defaults: g.defaults,
            affects: g.affects,
            allowed_for: g.allowed_for,
        }
    }
}

impl OptionGroup {
    /// The default for a riwāyah and optional ṭarīq: the ṭarīq's own default wins over the
    /// riwāyah's.
    pub fn default_for(&self, riwayah: &str, tariq: Option<&str>) -> Option<&str> {
        let find = |t: Option<&str>| {
            self.defaults
                .iter()
                .find(|d| d.riwayah == riwayah && d.tariq.as_deref() == t)
        };
        tariq
            .and_then(|t| find(Some(t)))
            .or_else(|| find(None))
            .map(|d| d.choice.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::from_yaml;

    const G: &str = "- id: \"t.opt\"\n  title_ar: \"test\"\n  choices: [\"off\", \"on\"]\n  defaults:\n    - {riwayah: \"r\", choice: \"off\"}\n    - {riwayah: \"r\", tariq: \"t\", choice: \"on\"}\n  affects: [\"p1\"]\n  allowed_for: [\"r\"]\n";

    #[test]
    fn parse_and_defaults() {
        let gs: Vec<OptionGroup> = from_yaml(G).unwrap();
        let g = &gs[0];
        assert_eq!(g.default_for("r", None), Some("off"));
        assert_eq!(g.default_for("r", Some("t")), Some("on"));
        assert_eq!(g.default_for("r", Some("other")), Some("off"));
        assert_eq!(g.default_for("x", None), None);
    }

    #[test]
    fn rejects_bad_groups() {
        for bad in [
            G.replace("[\"off\", \"on\"]", "[]"),
            G.replace("[\"off\", \"on\"]", "[\"on\", \"on\"]"),
            G.replace("tariq: \"t\", choice: \"on\"", "choice: \"on\""),
            G.replace("choice: \"off\"", "choice: \"maybe\""),
        ] {
            assert!(
                from_yaml::<Vec<OptionGroup>>(&bad).is_err(),
                "accepted {bad}"
            );
        }
    }
}
