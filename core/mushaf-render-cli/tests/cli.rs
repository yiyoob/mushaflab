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
