//! Test helpers: the repository, the tracked font, and pinned sources checked by sha256.
//!
//! Pinned sources are not tracked (rule 10). Locally a test that needs one skips with a message
//! when it is missing; CI sets MUSHAF_REQUIRE_SOURCES=1, which makes a missing source a failure.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use mushaf_shape::{EncodingTable, FontData};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn manifest_source(id: &str) -> Value {
    let text = fs::read_to_string(repo().join("data/sources/MANIFEST.json")).unwrap();
    let manifest: Value = serde_json::from_str(&text).unwrap();
    manifest["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == id)
        .unwrap_or_else(|| panic!("{id} not in MANIFEST.json"))
        .clone()
}

/// The bytes of a pinned source after checking its sha256, or None (skip) if absent locally.
pub fn pinned(id: &str) -> Option<Vec<u8>> {
    let src = manifest_source(id);
    let path = repo()
        .join("data/sources/raw")
        .join(src["file"].as_str().unwrap());
    match fs::read(&path) {
        Ok(bytes) => {
            assert_eq!(
                sha256(&bytes),
                src["sha256"].as_str().unwrap(),
                "{id}: sha256 differs from MANIFEST.json"
            );
            Some(bytes)
        }
        Err(_) => {
            assert!(
                std::env::var_os("MUSHAF_REQUIRE_SOURCES").is_none(),
                "{id} missing at {path:?} and MUSHAF_REQUIRE_SOURCES is set"
            );
            eprintln!("skipping: pinned source {id} not fetched (run `just data-fetch`)");
            None
        }
    }
}

/// The tracked DigitalKhatt Madina font (license-checked copy of its pinned source).
pub fn dk_font() -> FontData {
    let bytes = fs::read(repo().join("fonts/digitalkhatt/hb-madina.otf")).unwrap();
    assert_eq!(
        sha256(&bytes),
        manifest_source("digitalkhatt-hb-madina-font")["sha256"]
            .as_str()
            .unwrap()
    );
    FontData::new(bytes).unwrap()
}

pub fn hafs_table() -> EncodingTable {
    let path = repo().join("data/encodings/hafs-v1.kfgqpc-to-dk-madina.json");
    EncodingTable::from_json(&fs::read_to_string(path).unwrap()).unwrap()
}

/// DigitalKhatt's `quranText`: pages of quoted lines (the same parse as the pipeline).
pub fn dk_pages(text: &str) -> Vec<Vec<String>> {
    let body = &text[text.find('[').unwrap()..=text.rfind(']').unwrap()];
    let mut pages = Vec::new();
    let mut depth = 0;
    let mut page: Vec<String> = Vec::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 1 {
                    pages.push(std::mem::take(&mut page));
                }
            }
            '\'' => page.push(chars.by_ref().take_while(|&q| q != '\'').collect()),
            _ => {}
        }
    }
    assert_eq!(pages.len(), 604);
    pages
}
