//! Ayah maps (§3.4): a riwāyah's own `(surah, ayah)` numbering mapped to Ḥafṣ spine item
//! ranges. Item ranges (not whole ayahs) cover split and merged ayahs; inserted items (an
//! inserted ayah-end marker) may bound a range. Filled from data in T4.2.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Error, ItemRef};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AyahMapEntry {
    pub surah: u32,
    pub ayah: u32,
    pub first: ItemRef,
    pub last: ItemRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AyahMapWire", into = "AyahMapWire")]
pub struct AyahMap {
    pub riwayah: String,
    pub spine: String,
    pub entries: Vec<AyahMapEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AyahMapWire {
    riwayah: String,
    spine: String,
    entries: Vec<AyahMapEntry>,
}

impl TryFrom<AyahMapWire> for AyahMap {
    type Error = Error;
    fn try_from(w: AyahMapWire) -> Result<Self, Error> {
        let mut seen = BTreeSet::new();
        if let Some(e) = w.entries.iter().find(|e| !seen.insert((e.surah, e.ayah))) {
            return Err(Error::new(format!(
                "ayah map {}: {}:{} occurs twice",
                w.riwayah, e.surah, e.ayah
            )));
        }
        Ok(AyahMap {
            riwayah: w.riwayah,
            spine: w.spine,
            entries: w.entries,
        })
    }
}

impl From<AyahMap> for AyahMapWire {
    fn from(m: AyahMap) -> Self {
        AyahMapWire {
            riwayah: m.riwayah,
            spine: m.spine,
            entries: m.entries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::from_yaml;

    #[test]
    fn parse_and_reject_duplicates() {
        let y = "riwayah: \"r\"\nspine: \"t/v1\"\nentries:\n  - {surah: 1, ayah: 1, first: \"1:1:1\", last: \"1:1:3\"}\n  - {surah: 1, ayah: 2, first: \"1:1:3+ins.1\", last: \"1:2:4\"}\n";
        let m: AyahMap = from_yaml(y).unwrap();
        assert_eq!(m.entries[1].first.to_string(), "1:1:3+ins.1");
        assert!(from_yaml::<AyahMap>(&y.replace("ayah: 2", "ayah: 1")).is_err());
    }
}
