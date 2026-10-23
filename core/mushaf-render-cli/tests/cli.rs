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
fn riwayat_without_a_profile_are_refused_with_a_reason() {
    let err = Renderer::load(&repo(), "warsh").err().unwrap();
    assert!(err.to_string().contains("Phase 5"));
}
