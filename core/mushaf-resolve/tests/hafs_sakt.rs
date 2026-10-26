//! T6.2/T6.3: the Ḥafṣ option `hafs.maliyah` on the real data. Source: KFGQPC Ḥafṣ muṣḥaf,
//! explanation of signs, p. ح: for 69:28 مَالِيَهْ, iẓhār with sakt (as printed, the default) or
//! idghām into the hāʾ of هَلَكَ (69:29). Owner decision (D41): the idghām wajh changes no
//! codepoint; it is written to the annotation channel (D40). Spine codepoints:
//! `data/spine/hafs/v1` 69:28:4:5 and 69:29:1:1.

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
    assert_eq!(r.choices["hafs.tariq"], "shatibiyyah"); // T6.6, p. ح
    assert_eq!(cluster(&r, "69:28:4", 4), "U+0647 U+06E1 U+06DC");
    assert_eq!(cluster(&r, "69:29:1", 0), "U+0647 U+064E");
    let explicit = run(Some("izhar_sakt"));
    assert_eq!(explicit.items, r.items);
}

#[test]
fn idgham_changes_no_codepoint_only_annotations() {
    let base = run(None);
    let r = run(Some("idgham"));
    let text = |r: &Resolved| r.items.iter().map(|i| i.text()).collect::<String>();
    assert_eq!(text(&r), text(&base));
    assert_eq!(cluster(&r, "69:28:4", 4), "U+0647 U+06E1 U+06DC");
    assert_eq!(cluster(&r, "69:29:1", 0), "U+0647 U+064E");
    let ann = |item: &str, n: usize| {
        let it = r.items.iter().find(|i| i.id.to_string() == item).unwrap();
        it.clusters[n].attrs.annotations.clone()
    };
    assert_eq!(ann("69:28:4", 4)["pron.idgham"], "into_next");
    assert_eq!(ann("69:29:1", 0)["pron.idgham"], "from_previous");
    let annotated: usize = r
        .items
        .iter()
        .flat_map(|i| &i.clusters)
        .filter(|c| !c.attrs.annotations.is_empty())
        .count();
    assert_eq!(annotated, 2);
    assert!(
        base.items
            .iter()
            .flat_map(|i| &i.clusters)
            .all(|c| c.attrs.annotations.is_empty())
    );
}
