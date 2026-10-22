//! Spine, clusters, IDs and patch types (PROJECT_PLAN §3; ADR D9, D11, D12).
//!
//! Pure data and parsing: no file system, no clock, no network. Builds for every core target.

use std::fmt;

use serde::Serialize;
use serde::de::DeserializeOwned;

mod ayah_map;
mod cps;
mod ids;
mod layout;
mod options;
mod overlay;
mod patch;
mod spine;

pub use ayah_map::{AyahMap, AyahMapEntry};
pub use cps::{Cp, Cps};
pub use ids::{ClusterRef, ItemRef, MarkRef, Ref};
pub use layout::{Layout, Line, LineKind};
pub use options::{OptionDefault, OptionGroup};
pub use overlay::EditOverlay;
pub use patch::{
    AttrChange, AttrName, Layer, MarkAdd, MarkAt, NewCluster, NewItem, NewNode, Op, Owner,
    OwnerLevel, Patch, PatchFile, Side, Source, Status, When,
};
pub use spine::{Attrs, Cluster, ClusterKind, Item, ItemKind, Spine};

/// A model error: malformed data or a broken invariant, with a readable message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(String);

impl Error {
    pub fn new(msg: impl Into<String>) -> Self {
        Error(msg.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// Strict YAML: duplicate keys, merge keys, unknown tags, YAML 1.1 booleans (`yes`/`on`) and
/// unquoted numbers or booleans in string fields are all errors.
pub fn from_yaml<T: DeserializeOwned>(text: &str) -> Result<T, Error> {
    let options = serde_saphyr::options! {
        strict_booleans: true,
        no_schema: true,
        reject_unsupported_tags: true,
        merge_keys: serde_saphyr::MergeKeyPolicy::Error,
    };
    serde_saphyr::from_str_with_options(text, options).map_err(|e| Error::new(e.to_string()))
}

/// YAML with every string quoted, so YAML 1.1 readers (PyYAML) cannot retype ids such as
/// `1:1:1` (a base-60 integer in YAML 1.1).
pub fn to_yaml<T: Serialize>(value: &T) -> Result<String, Error> {
    let options = serde_saphyr::ser_options! { quote_all: true };
    serde_saphyr::to_string_with_options(value, options).map_err(|e| Error::new(e.to_string()))
}

pub fn from_json<T: DeserializeOwned>(text: &str) -> Result<T, Error> {
    serde_json::from_str(text).map_err(|e| Error::new(e.to_string()))
}

/// Compact JSON with fields in declaration order: the same value always gives the same bytes.
pub fn to_json<T: Serialize>(value: &T) -> Result<String, Error> {
    serde_json::to_string(value).map_err(|e| Error::new(e.to_string()))
}

/// Property tests use a fixed seed: CI runs are reproducible.
#[cfg(test)]
pub(crate) fn test_config() -> proptest::test_runner::Config {
    proptest::test_runner::Config {
        cases: 256,
        failure_persistence: None,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x6d75_7368_6166),
        ..proptest::test_runner::Config::default()
    }
}
