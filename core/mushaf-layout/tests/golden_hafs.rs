//! T2.7 golden: Hafs resolved with an empty stack and mapped to the 1441H layout equals its
//! source, page by page. Every expectation comes from a cited source, not from this code:
//!   * KFGQPC official v2.0 Hafs (`kfgqpc-official-hafs`, member `hafsData_v2-0.json`, pinned
//!     by sha256 in data/sources/MANIFEST.json and verified here before reading, rule 2):
//!     per ayah `aya_text` (byte for byte), `page`, `line_start`, `line_end`;
//!   * tests/golden/page_boundaries.json (T1.5): per page the line count and first/last item.
//!
//! The pinned zip is not tracked (rule 10). Locally the test skips with a message when it is
//! missing; CI sets MUSHAF_REQUIRE_SOURCES=1, which makes a missing source a failure.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use mushaf_layout::map_lines;
use mushaf_model::LineKind;
use mushaf_resolve::{Mode, resolve_patches};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// (page, line)
type At = (u32, u32);

const SOURCE_ID: &str = "kfgqpc-official-hafs";
const MEMBER: &str = "hafsData_v2-0.json";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// The pinned source rows, after checking the zip's sha256 against MANIFEST.json and the
/// spine manifest (the spine was built from exactly this file). None if absent locally.
fn kfgqpc_rows() -> Option<Vec<Value>> {
    let manifest = json(&repo().join("data/sources/MANIFEST.json"));
    let entry = manifest["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == SOURCE_ID)
        .unwrap();
    let path = repo()
        .join("data/sources/raw")
        .join(entry["file"].as_str().unwrap());
    if !path.exists() {
        assert!(
            std::env::var_os("MUSHAF_REQUIRE_SOURCES").is_none(),
            "{} missing: run `just data-fetch`",
            path.display()
        );
        eprintln!(
            "SKIPPED: {} missing (run `just data-fetch`)",
            path.display()
        );
        return None;
    }
    let bytes = fs::read(&path).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        sha,
        entry["sha256"].as_str().unwrap(),
        "pinned source checksum mismatch (rule 2)"
    );
    let spine_manifest = json(&repo().join("data/spine/hafs/v1/manifest.json"));
    assert_eq!(
        spine_manifest["source"]["sha256"].as_str().unwrap(),
        sha,
        "spine built from another file"
    );
    assert_eq!(spine_manifest["source"]["member"].as_str().unwrap(), MEMBER);
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let names: Vec<String> = zip
        .file_names()
        .map(|n| n.unwrap().into_owned())
        .filter(|n| n.ends_with(MEMBER))
        .collect();
    assert_eq!(
        names.len(),
        1,
        "expected one member ending {MEMBER}: {names:?}"
    );
    let mut text = String::new();
    zip.by_name(&names[0])
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    Some(
        serde_json::from_str::<Value>(text)
            .unwrap()
            .as_array()
            .unwrap()
            .clone(),
    )
}

fn int(v: &Value) -> u32 {
    v.as_u64()
        .map(|n| n as u32)
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap()
}

#[test]
fn resolved_hafs_pages_equal_the_kfgqpc_source() {
    let Some(rows) = kfgqpc_rows() else { return };
    let ds = mushaf_db::load_dir(&repo().join("data")).unwrap();
    let (spine, layout) = (&ds.spines["hafs/v1"], &ds.layouts["uthmani-1441h"]);
    let resolved = resolve_patches(spine, &[], Mode::Data).unwrap();
    let mapping = map_lines(layout, spine, &resolved).unwrap();

    // Walk the mapped lines: per ayah its text and (page, line) span; per page its boundaries.
    let mut ayah_text: BTreeMap<(u32, u32), String> = BTreeMap::new();
    let mut ayah_span: BTreeMap<(u32, u32), (At, At)> = BTreeMap::new();
    let mut pages: BTreeMap<u32, (u32, Option<String>, Option<String>)> = BTreeMap::new();
    let mut page_text: BTreeMap<u32, String> = BTreeMap::new();
    for ml in &mapping.lines {
        let page = pages.entry(ml.page).or_default();
        page.0 += 1;
        for it in &resolved.items[ml.items.clone()] {
            let key = (it.id.surah, it.id.ayah);
            ayah_text.entry(key).or_default().push_str(&it.text());
            page_text.entry(ml.page).or_default().push_str(&it.text());
            let at = (ml.page, ml.line);
            ayah_span
                .entry(key)
                .and_modify(|s| s.1 = at)
                .or_insert((at, at));
            page.1.get_or_insert_with(|| it.id.to_string());
            page.2 = Some(it.id.to_string());
        }
        if let LineKind::Text { .. } = ml.kind {
            assert!(
                !ml.items.is_empty(),
                "text line {}:{} is empty",
                ml.page,
                ml.line
            );
        }
    }

    // (1) and (2): per ayah, against KFGQPC.
    assert_eq!(rows.len(), ayah_text.len(), "ayah count");
    let mut source_text = String::new();
    for row in &rows {
        let key = (int(&row["sura_no"]), int(&row["aya_no"]));
        let source = row["aya_text"].as_str().unwrap();
        source_text.push_str(source);
        assert!(
            ayah_text[&key] == source,
            "ayah {key:?}: resolved text differs from aya_text"
        );
        let ((p0, l0), (p1, l1)) = ayah_span[&key];
        let mine = (
            if p0 == p1 {
                p0.to_string()
            } else {
                format!("{p0}-{p1}")
            },
            l0,
            l1,
        );
        let page = match &row["page"] {
            Value::String(s) => s.clone(),
            v => v.to_string(),
        };
        let theirs = (page, int(&row["line_start"]), int(&row["line_end"]));
        assert_eq!(mine, theirs, "ayah {key:?}: (page, line_start, line_end)");
    }
    // The pages, concatenated in order, are the whole source text.
    let all_pages: String = page_text.values().map(String::as_str).collect();
    assert!(
        all_pages == source_text,
        "concatenated pages differ from the source"
    );

    // (3): per page, against tests/golden/page_boundaries.json.
    let golden = json(&repo().join("tests/golden/page_boundaries.json"));
    let golden = golden.as_object().unwrap();
    assert_eq!(golden.len(), pages.len(), "page count");
    for (page, (lines, first, last)) in &pages {
        let g = &golden[&page.to_string()];
        assert_eq!(*lines, int(&g["lines"]), "page {page}: line count");
        assert_eq!(
            first.as_deref(),
            g["first"].as_str(),
            "page {page}: first item"
        );
        assert_eq!(
            last.as_deref(),
            g["last"].as_str(),
            "page {page}: last item"
        );
    }
}
