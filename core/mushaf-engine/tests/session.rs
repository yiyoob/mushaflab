//! T8.1 (D56, D60): the session the web app runs draws exactly what the CLI draws, and its
//! option groups, margin notes, inspector and edits come from the data. Real data: run by
//! `just golden` (release).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use mushaf_draw::{DrawItem, Role};
use mushaf_engine::session::{EditOp, Session};
use mushaf_engine::{HighlightPolicy, Renderer, Source};
use mushaf_model::DataSet;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn json<T: serde::Serialize>(t: &T) -> String {
    serde_json::to_string(t).unwrap()
}

fn has_marker(d: &mushaf_draw::DrawPage) -> bool {
    d.items.iter().any(|i| {
        matches!(
            i,
            DrawItem::Stroke {
                role: Role::EditedMarker,
                ..
            }
        )
    })
}

/// The page holding `id` in the riwāyah's layout (found from the data, as in golden.rs).
fn page_of(r: &Renderer, id: &str) -> u32 {
    use mushaf_model::LineKind;
    let spine = &r.data.spines[&r.profile.spine];
    let layout = &r.data.layouts[&r.profile.layout_id];
    let p = spine.position(&id.parse().unwrap()).unwrap();
    layout
        .lines
        .iter()
        .find(|l| match &l.kind {
            LineKind::Text { first, last } => {
                spine.position(first).unwrap() <= p && p <= spine.position(last).unwrap()
            }
            _ => false,
        })
        .unwrap()
        .page
}

#[test]
#[ignore = "run by `just golden` (release)"]
fn session_draws_what_the_cli_draws() {
    let repo = repo();
    let data = Arc::new(DataSet::from_files(&repo.data_files().unwrap()).unwrap());
    for riwayah in ["hafs", "warsh"] {
        let style = mushaf_engine::default_style(riwayah).unwrap();
        let cli = Renderer::load_with(&repo, data.clone(), riwayah, style).unwrap();
        let mut s = Session::new(
            &repo,
            Renderer::load_with(&repo, data.clone(), riwayah, style).unwrap(),
            style,
        );
        for page in [1, 3, 106] {
            assert_eq!(
                json(&s.render(page, false).unwrap()),
                json(&cli.render(page, None).unwrap()),
                "{riwayah} p{page} plain"
            );
            assert_eq!(
                json(&s.render(page, true).unwrap()),
                json(
                    &cli.render_highlighted(page, &BTreeMap::new(), HighlightPolicy::default())
                        .unwrap()
                ),
                "{riwayah} p{page} highlighted"
            );
        }
    }
}

/// Option groups and their defaults come from the data (`data/options/*.yaml`), and the
/// margin carries the alternative wajh of `hafs.maliyah` on the page holding 69:28 (T6.2,
/// golden `hafs-maliyah-p567`) and nothing on a page without one.
#[test]
#[ignore = "run by `just golden` (release)"]
fn option_groups_and_margin_notes_come_from_the_data() {
    let repo = repo();
    let r = Renderer::load(&repo, "hafs").unwrap();
    let page = page_of(&r, "69:28:4");
    let groups = r.data.option_groups.clone();
    let mut s = Session::new(&repo, r, "hijazi");
    let views = s.option_groups();
    assert!(!views.is_empty());
    for v in &views {
        let g = &groups[&v.id];
        let data_default = g
            .defaults
            .iter()
            .find(|d| d.riwayah == "hafs")
            .map(|d| d.choice.clone());
        assert_eq!(v.default, data_default, "{}", v.id);
        assert_eq!(v.current, v.default);
    }
    assert!(s.margin_notes(1).unwrap().is_empty());
    let notes = s.margin_notes(page).unwrap();
    assert!(!notes.is_empty());
    for n in &notes {
        assert_eq!(n.group, "hafs.maliyah");
        assert_ne!(n.annotations, n.current_annotations, "{n:?}");
        assert!(n.word.is_some(), "{:?}", n.word_error);
    }
    // choosing the other wajh moves the note the other way; the page is drawn identically (D41)
    let before = json(&s.render(page, false).unwrap());
    let other = notes[0].choice.clone();
    s.set_choices(BTreeMap::from([("hafs.maliyah".into(), other.clone())]))
        .unwrap();
    assert_eq!(json(&s.render(page, false).unwrap()), before);
    let back = s.margin_notes(page).unwrap();
    assert!(!back.is_empty() && back.iter().all(|n| n.choice != other));
    assert!(
        s.set_choices(BTreeMap::from([("hafs.maliyah".into(), "x".into())]))
            .is_err()
    );
}

/// The Phase 7 edit fixture through the session: same draw list as the CLI with the overlay,
/// marked (rule 11), and clearing the edits restores the canonical page exactly.
#[test]
#[ignore = "run by `just golden` (release)"]
fn edits_mark_the_page_and_clear_exactly() {
    let repo = repo();
    let overlay_text =
        std::fs::read_to_string(repo.join("tests/golden/pages/hafs-p003-edit.overlay.yaml"))
            .unwrap();
    let overlay = mushaf_model::from_yaml(&overlay_text).unwrap();
    let cli = Renderer::load(&repo, "hafs").unwrap();
    let want = json(&cli.render(3, Some(&overlay)).unwrap());
    let plain = json(&cli.render(3, None).unwrap());
    let mut s = Session::new(&repo, cli, "hijazi");
    s.import_edits(&overlay_text).unwrap();
    let d = s.render(3, false).unwrap();
    assert!(!d.canonical && has_marker(&d));
    assert_eq!(json(&d), want);
    assert!(!s.lint().unwrap().is_empty());
    let insp = s.inspect(3).unwrap();
    assert!(!insp.user_patches.is_empty());
    assert!(
        insp.clusters
            .iter()
            .flat_map(|c| &c.provenance)
            .any(|p| p.kind == "user")
    );
    s.edit(EditOp::Clear).unwrap();
    assert_eq!(json(&s.render(3, false).unwrap()), plain);
}

/// A mark edit through the editor's JSON, then undo: marked, then exactly the original.
#[test]
#[ignore = "run by `just golden` (release)"]
fn editor_ops_from_json_and_undo() {
    let repo = repo();
    let mut s = Session::new(&repo, Renderer::load(&repo, "hafs").unwrap(), "hijazi");
    let plain = json(&s.render(3, false).unwrap());
    let first = s.inspect(3).unwrap().clusters[0].id.to_string();
    let op: EditOp = serde_json::from_str(&format!(
        r#"{{"op":"set_width","target":"{first}","value":32}}"#
    ))
    .unwrap();
    s.edit(op).unwrap();
    let d = s.render(3, false).unwrap();
    assert!(!d.canonical && has_marker(&d));
    assert!(s.can_undo());
    s.edit(EditOp::Undo).unwrap();
    assert_eq!(json(&s.render(3, false).unwrap()), plain);
}

/// Every cluster the inspector lists is drawn on the page, and every provenance id resolves
/// to a patch, rule or user edit of the stack.
#[test]
#[ignore = "run by `just golden` (release)"]
fn inspector_and_provenance_resolve_every_id() {
    let repo = repo();
    let mut s = Session::new(&repo, Renderer::load(&repo, "warsh").unwrap(), "maghribi");
    let insp = s.inspect(3).unwrap();
    assert!(!insp.clusters.is_empty());
    for c in &insp.clusters {
        for p in &c.provenance {
            assert!(
                p.status != "unknown" && p.status != "unregistered",
                "{}: {p:?}",
                c.id
            );
        }
    }
    let prov = s.provenance();
    assert_eq!(prov.patch_files.len(), insp.patch_stack.len());
    assert!(prov.patch_files.iter().all(|f| f.patches > 0));
}

/// Only the default styles and what the T6.9 audit marks available are offered (D44).
#[test]
#[ignore = "run by `just golden` (release)"]
fn styles_offer_only_audited_combinations() {
    let repo = repo();
    let data = DataSet::from_files(&repo.data_files().unwrap()).unwrap();
    let styles = mushaf_engine::styles(&repo, &data).unwrap();
    let audit: serde_json::Value =
        serde_json::from_slice(&repo.read(mushaf_engine::STYLE_AVAILABILITY).unwrap()).unwrap();
    for r in data.profiles.keys() {
        let d = mushaf_engine::default_style(r).unwrap();
        assert!(
            styles
                .iter()
                .any(|s| s.riwayah == *r && s.style == d && s.default)
        );
    }
    for e in audit["entries"].as_array().unwrap() {
        let offered = styles
            .iter()
            .any(|s| s.riwayah == e["riwayah"] && s.style == e["style"]);
        assert_eq!(offered, e["available"] == true, "{e}");
    }
}
