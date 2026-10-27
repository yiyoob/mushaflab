//! T3.4: the reference renderer runs end to end; its SVG parses with usvg and rasterises.

use std::path::{Path, PathBuf};

use mushaf_render::{Renderer, png, svg};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn pages_render_to_svg_that_parses_and_to_png() {
    let r = Renderer::load(&repo(), "hafs").unwrap();
    for page in [1, 604] {
        let d = r.render(page, None).unwrap();
        let s = svg(&d);
        let tree = resvg::usvg::Tree::from_str(&s, &resvg::usvg::Options::default()).unwrap();
        assert!(tree.root().has_children());
        let bytes = png(&s, 300).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
    }
}

#[test]
fn an_unknown_riwayah_is_refused_with_a_reason() {
    let err = Renderer::load(&repo(), "nope").err().unwrap();
    assert!(err.to_string().contains("unknown riwayah"));
}

/// T5.2: a riwāyah on its native layout, in its own KFGQPC font (D27, D29).
#[test]
fn warsh_renders_on_its_native_layout() {
    let r = Renderer::load(&repo(), "warsh").unwrap();
    let d = r.render(1, None).unwrap();
    assert_eq!(d.layout_id, "uthmani-warsh-kfgqpc");
    assert!(d.canonical);
    assert_eq!(&png(&svg(&d), 300).unwrap()[1..4], b"PNG");
}

/// D36/D44/D46: render profiles are keyed by (riwāyah, style). A non-default style is refused,
/// with the T6.9 audit's counts, unless it is offered (Warsh and Qālūn in Ḥijāzī, D46).
#[test]
fn render_profiles_are_keyed_by_riwayah_and_style() {
    use mushaf_render::{default_style, profile_path};
    for (r, style) in [
        ("hafs", "hijazi"),
        ("shuba", "hijazi"),
        ("warsh", "maghribi"),
        ("qalun", "maghribi"),
        ("bazzi", "hijazi"),
        ("qunbul", "hijazi"),
        ("duri_abuamr", "hijazi"),
        ("susi", "hijazi"),
    ] {
        assert_eq!(default_style(r).unwrap(), style);
        let p = mushaf_layout::fit::Profile::from_json(
            &std::fs::read_to_string(repo().join(profile_path(r, style).unwrap())).unwrap(),
        )
        .unwrap();
        assert_eq!((p.riwayah.as_str(), p.style.as_str()), (r, style));
    }
    let err = Renderer::load_style(&repo(), "hafs", "maghribi")
        .err()
        .unwrap();
    assert!(err.to_string().contains("not available (D44"), "{err}");
    assert!(err.to_string().contains("fail rule 6"), "{err}");
    // D46: Warsh and Qālūn are offered in the Ḥijāzī style (their profile exists).
    for r in ["warsh", "qalun"] {
        let h = Renderer::load_style(&repo(), r, "hijazi").unwrap();
        assert_eq!(
            (h.profile.riwayah.as_str(), h.profile.style.as_str()),
            (r, "hijazi")
        );
    }
    assert!(Renderer::load_style(&repo(), "hafs", "naskh").is_err());
}
