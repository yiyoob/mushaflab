//! T3.1: the core converts every Hafs spine item exactly as the pipeline does (D14).
//! Expected values come from the tracked table itself: `converted_spine` (sha256 over
//! `id TAB converted cps LF`, computed by data/pipeline/dk_encoding.py) and `lossy_items`.

mod common;

use common::{hafs_table, repo, sha256};

#[test]
fn every_spine_item_converts_like_the_pipeline() {
    let ds = mushaf_db::load_dir(&repo().join("data")).unwrap();
    let spine = &ds.spines["hafs/v1"];
    let table = hafs_table();
    assert_eq!(table.spine, spine.id);
    let mut digest = String::new();
    let mut lossy = Vec::new();
    for item in &spine.items {
        let clusters: Vec<&[char]> = item.clusters.iter().map(|c| c.cps.0.as_slice()).collect();
        let conv = table
            .convert(&clusters)
            .unwrap_or_else(|e| panic!("{}: no row for cluster {}", item.id, e.cps));
        digest.push_str(&format!(
            "{}\t{}\n",
            item.id,
            mushaf_model::Cps(conv.chars.clone())
        ));
        if conv.spans.iter().any(|s| s.lossy) {
            lossy.push(item.id.to_string());
        }
    }
    assert_eq!(table.converted_spine.items, spine.items.len());
    assert_eq!(sha256(digest.as_bytes()), table.converted_spine.sha256);
    assert_eq!(lossy, table.lossy_items);
    assert_eq!(
        lossy.len(),
        33,
        "G-040 lists 33 lossy words (owner decision, T3.1)"
    );
}
