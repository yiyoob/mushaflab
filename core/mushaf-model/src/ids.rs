//! Spine and patch identifiers (PROJECT_PLAN §3.1, §3.2; ADR D9, D12).
//!
//! Grammar (canonical, so `parse(display(x)) == x` and `display(parse(s)) == s`):
//!
//! ```text
//! item    = surah ":" ayah ":" pos ins*          e.g. 2:26:0, 9:100:12+ins.1
//! cluster = item ":" n ins*                      e.g. 3:133:1:2, 3:133:1:2+ins.1, 9:100:12+ins.1:2
//! mark    = cluster "#m" n                       e.g. 3:133:1:2#m1
//! ins     = "+ins." n                            n >= 1, the explicit integer `seq`
//! ```
//!
//! Numbers are decimal without leading zeros. `pos` 0 is the hizb sign (D11). Order between
//! items is never derived from these strings; it comes from spine position and `seq`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Error;

/// A spine item (`s:a:n`) or an item inserted after/before one (`s:a:n+ins.k`, nestable).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemRef {
    pub surah: u32,
    pub ayah: u32,
    pub pos: u32,
    /// Insert chain: empty for a spine item.
    pub ins: Vec<u32>,
}

/// A cluster of an item (`<item>:n`), or a cluster inserted next to one (`<item>:n+ins.k`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClusterRef {
    pub item: ItemRef,
    /// 1-based cluster index within the item's own (spine or payload) clusters.
    pub n: u32,
    pub ins: Vec<u32>,
}

/// A mark inside a cluster (`<cluster>#m<n>`); marks are the codepoints after the first.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MarkRef {
    pub cluster: ClusterRef,
    pub n: u32,
}

/// A patch target: an item or a cluster.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Ref {
    Item(ItemRef),
    Cluster(ClusterRef),
}

impl ItemRef {
    pub fn spine(surah: u32, ayah: u32, pos: u32) -> Self {
        Self {
            surah,
            ayah,
            pos,
            ins: Vec::new(),
        }
    }

    pub fn is_spine(&self) -> bool {
        self.ins.is_empty()
    }

    /// The id given to an item inserted next to this one with `seq`.
    pub fn inserted(&self, seq: u32) -> Self {
        let mut r = self.clone();
        r.ins.push(seq);
        r
    }

    pub fn cluster(&self, n: u32) -> ClusterRef {
        ClusterRef {
            item: self.clone(),
            n,
            ins: Vec::new(),
        }
    }
}

impl ClusterRef {
    pub fn inserted(&self, seq: u32) -> Self {
        let mut r = self.clone();
        r.ins.push(seq);
        r
    }
}

impl Ref {
    /// The item this ref is or lies in.
    pub fn item(&self) -> &ItemRef {
        match self {
            Ref::Item(i) => i,
            Ref::Cluster(c) => &c.item,
        }
    }

    /// The id of a node inserted next to this one with `seq`.
    pub fn inserted(&self, seq: u32) -> Ref {
        match self {
            Ref::Item(i) => Ref::Item(i.inserted(seq)),
            Ref::Cluster(c) => Ref::Cluster(c.inserted(seq)),
        }
    }

    /// True if `self` lies strictly inside `other` (a cluster of that item).
    pub fn is_inside(&self, other: &Ref) -> bool {
        matches!((self, other), (Ref::Cluster(c), Ref::Item(i)) if c.item == *i)
    }
}

fn number(s: &str, what: &str, min: u32) -> Result<u32, Error> {
    let canonical =
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'));
    let n: u32 = if canonical { s.parse().ok() } else { None }
        .ok_or_else(|| Error::new(format!("{what}: {s:?} is not a canonical decimal number")))?;
    if n < min {
        return Err(Error::new(format!("{what}: {n} is below {min}")));
    }
    Ok(n)
}

/// `"12+ins.1+ins.2"` -> (12, [1, 2]).
fn with_ins(s: &str, what: &str, min: u32) -> Result<(u32, Vec<u32>), Error> {
    let mut parts = s.split("+ins.");
    let head = number(parts.next().unwrap_or_default(), what, min)?;
    let ins = parts
        .map(|p| number(p, "insert seq", 1))
        .collect::<Result<_, _>>()?;
    Ok((head, ins))
}

fn parse_ref(s: &str) -> Result<Ref, Error> {
    let err = |e: Error| Error::new(format!("id {s:?}: {e}"));
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 && parts.len() != 4 {
        return Err(Error::new(format!(
            "id {s:?}: expected 3 or 4 ':'-separated parts"
        )));
    }
    let surah = number(parts[0], "surah", 1).map_err(err)?;
    let ayah = number(parts[1], "ayah", 1).map_err(err)?;
    let (pos, ins) = with_ins(parts[2], "position", 0).map_err(err)?;
    let item = ItemRef {
        surah,
        ayah,
        pos,
        ins,
    };
    if parts.len() == 3 {
        return Ok(Ref::Item(item));
    }
    let (n, ins) = with_ins(parts[3], "cluster", 1).map_err(err)?;
    Ok(Ref::Cluster(ClusterRef { item, n, ins }))
}

fn write_ins(f: &mut fmt::Formatter<'_>, ins: &[u32]) -> fmt::Result {
    ins.iter().try_for_each(|k| write!(f, "+ins.{k}"))
}

impl fmt::Display for ItemRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.surah, self.ayah, self.pos)?;
        write_ins(f, &self.ins)
    }
}

impl fmt::Display for ClusterRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.item, self.n)?;
        write_ins(f, &self.ins)
    }
}

impl fmt::Display for MarkRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#m{}", self.cluster, self.n)
    }
}

impl fmt::Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ref::Item(i) => i.fmt(f),
            Ref::Cluster(c) => c.fmt(f),
        }
    }
}

impl FromStr for Ref {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        parse_ref(s)
    }
}

impl FromStr for ItemRef {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        match parse_ref(s)? {
            Ref::Item(i) => Ok(i),
            Ref::Cluster(_) => Err(Error::new(format!("{s:?} is a cluster id, not an item id"))),
        }
    }
}

impl FromStr for ClusterRef {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        match parse_ref(s)? {
            Ref::Cluster(c) => Ok(c),
            Ref::Item(_) => Err(Error::new(format!("{s:?} is an item id, not a cluster id"))),
        }
    }
}

impl FromStr for MarkRef {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        let (cluster, mark) = s
            .split_once("#m")
            .ok_or_else(|| Error::new(format!("{s:?} has no '#m' part")))?;
        let n = number(mark, "mark", 1).map_err(|e| Error::new(format!("id {s:?}: {e}")))?;
        Ok(MarkRef {
            cluster: cluster.parse()?,
            n,
        })
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

serde_via_string!(ItemRef, ClusterRef, MarkRef, Ref);

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn rt<T: FromStr<Err = Error> + fmt::Display>(s: &str) -> String {
        s.parse::<T>()
            .map(|r| r.to_string())
            .unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn canonical_forms_round_trip() {
        for s in ["1:1:1", "2:26:0", "9:100:12+ins.1", "9:100:12+ins.1+ins.3"] {
            assert_eq!(rt::<ItemRef>(s), s);
            assert_eq!(rt::<Ref>(s), s);
        }
        for s in [
            "3:133:1:1",
            "3:133:1:2+ins.1",
            "9:100:12+ins.1:2",
            "9:100:12+ins.1:2+ins.4",
        ] {
            assert_eq!(rt::<ClusterRef>(s), s);
            assert_eq!(rt::<Ref>(s), s);
        }
        assert_eq!(rt::<MarkRef>("3:133:1:2#m1"), "3:133:1:2#m1");
    }

    #[test]
    fn structure() {
        let c: ClusterRef = "9:100:12+ins.1:2+ins.4".parse().unwrap();
        assert_eq!(c.item, ItemRef::spine(9, 100, 12).inserted(1));
        assert_eq!((c.n, c.ins.as_slice()), (2, &[4][..]));
        let item = Ref::Item(c.item.clone());
        assert!(Ref::Cluster(c.clone()).is_inside(&item));
        assert!(!item.is_inside(&Ref::Cluster(c)));
        assert!(!Ref::Item(ItemRef::spine(9, 100, 12)).is_inside(&item));
    }

    #[test]
    fn rejects_non_canonical_or_malformed() {
        for s in [
            "",
            "1",
            "1:1",
            "1:1:1:1:1",
            "01:1:1",
            "1:1:01",
            "1:1:+1",
            "1:1:-1",
            "0:1:1",
            "1:0:1",
            "1:1:1:0",
            "1:1:1+ins.0",
            "1:1:1+ins.",
            "1:1:1+ins.01",
            "1:1:1+ins1",
            "1:1:1 ",
            " 1:1:1",
            "1:1:a",
            "1:1:1:1#m1",
            "4294967296:1:1",
        ] {
            assert!(s.parse::<Ref>().is_err(), "accepted {s:?}");
        }
        for s in ["1:1:1:1#m0", "1:1:1:1#m", "1:1:1#m1", "1:1:1:1#m01"] {
            assert!(s.parse::<MarkRef>().is_err(), "accepted {s:?}");
        }
        assert!("1:1:1:1".parse::<ItemRef>().is_err());
        assert!("1:1:1".parse::<ClusterRef>().is_err());
    }

    #[test]
    fn serde_is_the_string_form() {
        let r: Ref = serde_json::from_str("\"9:100:12+ins.1:2\"").unwrap();
        assert_eq!(serde_json::to_string(&r).unwrap(), "\"9:100:12+ins.1:2\"");
        assert!(serde_json::from_str::<Ref>("9").is_err());
    }

    fn arb_ins() -> impl Strategy<Value = Vec<u32>> {
        proptest::collection::vec(1u32..=u32::MAX, 0..3)
    }

    fn arb_ref() -> impl Strategy<Value = Ref> {
        let item = (1u32..=u32::MAX, 1u32..=u32::MAX, 0u32..=u32::MAX, arb_ins()).prop_map(
            |(surah, ayah, pos, ins)| ItemRef {
                surah,
                ayah,
                pos,
                ins,
            },
        );
        (item, proptest::option::of((1u32..=u32::MAX, arb_ins()))).prop_map(|(item, c)| match c {
            None => Ref::Item(item),
            Some((n, ins)) => Ref::Cluster(ClusterRef { item, n, ins }),
        })
    }

    proptest! {
        #![proptest_config(crate::test_config())]
        #[test]
        fn parse_display_round_trip(r in arb_ref()) {
            let s = r.to_string();
            prop_assert_eq!(s.parse::<Ref>().unwrap(), r.clone());
            prop_assert_eq!(s.parse::<Ref>().unwrap().to_string(), s);
        }
    }
}
