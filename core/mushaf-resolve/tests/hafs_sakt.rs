//! T6.2: the Ḥafṣ option `hafs.maliyah` on the real data. Expected values cite KFGQPC Ḥafṣ
//! muṣḥaf, explanation of signs, p. ح: for 69:28 مَالِيَهْ, iẓhār with sakt (what the muṣḥaf
//! prints, so the default leaves the spine unchanged), or idghām into the hāʾ of هَلَكَ (69:29):
//! no sukūn on the first hāʾ, shadda on the second. Dropping U+06DC with the sakt is our
//! reading (G-143). Spine codepoints: `data/spine/hafs/v1` 69:28:4:5 and 69:29:1:1.

mod common;

use std::collections::BTreeMap;

use common::repo;
use mushaf_resolve::{Mode, Resolved, Selection, resolve_profile};

fn run(choice: Option<&str>) -> Resolved {
    let ds = mushaf_db::load_dir(&repo().join("data")).unwrap();
    let choices: BTreeMap<String, String> = choice
        .map(|c| [("hafs.maliyah".to_string(), c.to_string())].into())
        .unwrap_or_default();
    let sel = Selection {
        riwayah: "hafs".into(),
        tariq: None,
        choices,
    };
    resolve_profile(&ds, &sel, None, Mode::Data).unwrap()
}

fn cluster(r: &Resolved, item: &str, n: usize) -> String {
    let it = r.items.iter().find(|i| i.id.to_string() == item).unwrap();
    it.clusters[n]
        .cps
        .0
        .iter()
        .map(|c| format!("U+{:04X}", u32::from(*c)))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn default_is_the_printed_izhar_with_sakt() {
    let r = run(None);
    assert_eq!(r.choices["hafs.maliyah"], "izhar_sakt");
    assert_eq!(cluster(&r, "69:28:4", 4), "U+0647 U+06E1 U+06DC");
    assert_eq!(cluster(&r, "69:29:1", 0), "U+0647 U+064E");
    let explicit = run(Some("izhar_sakt"));
    assert_eq!(explicit.items, r.items);
}

#[test]
fn idgham_changes_exactly_the_two_ha() {
    let base = run(None);
    let r = run(Some("idgham"));
    assert_eq!(cluster(&r, "69:28:4", 4), "U+0647");
    assert_eq!(cluster(&r, "69:29:1", 0), "U+0647 U+0651 U+064E");
    let changed: Vec<String> = base
        .items
        .iter()
        .zip(&r.items)
        .filter(|(a, b)| a.clusters != b.clusters || a.text() != b.text())
        .map(|(a, _)| a.id.to_string())
        .collect();
    assert_eq!(changed, ["69:28:4", "69:29:1"]);
}
