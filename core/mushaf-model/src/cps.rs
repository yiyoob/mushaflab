//! Codepoint sequences, written in tracked files as ASCII `"U+XXXX"` strings (D11).
//!
//! Only the canonical form is accepted (`U+` + uppercase hex, at least 4 digits, no extra
//! leading zeros, single spaces between codepoints), so a sequence has exactly one spelling
//! and no invisible or look-alike character can enter tracked data. No normalization.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Error;

/// One codepoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cp(pub char);

/// A sequence of codepoints, stored exactly as in the source.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cps(pub Vec<char>);

impl Cps {
    pub fn from_text(text: &str) -> Self {
        Cps(text.chars().collect())
    }

    /// The real characters (the form stored in `mushaf.db`).
    pub fn text(&self) -> String {
        self.0.iter().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for Cp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "U+{:04X}", u32::from(self.0))
    }
}

impl FromStr for Cp {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        let bad = || Error::new(format!("{s:?} is not a canonical U+XXXX codepoint"));
        let hex = s.strip_prefix("U+").ok_or_else(bad)?;
        let digits_ok = (4..=6).contains(&hex.len())
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
            && (hex.len() == 4 || !hex.starts_with('0'));
        if !digits_ok {
            return Err(bad());
        }
        let value = u32::from_str_radix(hex, 16).map_err(|_| bad())?;
        char::from_u32(value).map(Cp).ok_or_else(bad)
    }
}

impl fmt::Display for Cps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, c) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            Cp(*c).fmt(f)?;
        }
        Ok(())
    }
}

impl FromStr for Cps {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        if s.is_empty() {
            return Ok(Cps::default());
        }
        s.split(' ')
            .map(|p| p.parse::<Cp>().map(|c| c.0))
            .collect::<Result<_, _>>()
            .map(Cps)
    }
}

macro_rules! serde_via_string {
    ($($t:ty),*) => {$(
        impl Serialize for $t {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_str(self)
            }
        }
        impl<'de> Deserialize<'de> for $t {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }
    )*};
}

serde_via_string!(Cp, Cps);

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn canonical_round_trip() {
        for s in [
            "",
            "U+0020",
            "U+0628 U+0650",
            "U+00A0",
            "U+FC00",
            "U+1F600",
            "U+10FFFF",
        ] {
            assert_eq!(s.parse::<Cps>().unwrap().to_string(), s);
        }
        assert_eq!("U+0061 U+0062".parse::<Cps>().unwrap().text(), "ab");
    }

    #[test]
    fn rejects_non_canonical() {
        for s in [
            "U+20",
            "u+0020",
            "U+0020 ",
            " U+0020",
            "U+0020  U+0021",
            "U+00a0",
            "U+00020",
            "U+D800",
            "U+110000",
            "0020",
            "U+",
            "U+0020,U+0021",
            "U+0000020",
        ] {
            assert!(s.parse::<Cps>().is_err(), "accepted {s:?}");
        }
    }

    proptest! {
        #![proptest_config(crate::test_config())]
        #[test]
        fn text_and_string_forms_round_trip(text in "\\PC{0,8}") {
            let cps = Cps::from_text(&text);
            prop_assert_eq!(cps.to_string().parse::<Cps>().unwrap(), cps.clone());
            prop_assert_eq!(cps.text(), text);
        }
    }
}
