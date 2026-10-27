//! T8.2 (D57): the web bundle holds the files the engine reads, byte for byte, and the engine
//! draws the same pages from it as from the checkout.

use std::path::PathBuf;

use mushaf_render::bundle::Bundle;
use mushaf_render::{Renderer, Source, bundle_files};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
#[ignore = "run by `just golden` (release)"]
fn bundle_matches_the_checkout_and_draws_the_same_pages() {
    let repo = repo();
    let bundle = Bundle::parse(
        &Bundle {
            files: bundle_files(&repo).unwrap(),
        }
        .to_bytes()
        .unwrap(),
    )
    .unwrap();
    // every file is the checkout's bytes
    for (path, bytes) in &bundle.files {
        assert_eq!(*bytes, std::fs::read(repo.join(path)).unwrap(), "{path}");
    }
    // data files: the checkout's, less only patch files no profile stacks
    let disk = repo.data_files().unwrap();
    let web = bundle.data_files().unwrap();
    let stacked: Vec<String> = web
        .iter()
        .filter(|(k, _)| k.starts_with("profiles/"))
        .flat_map(|(_, v)| {
            mushaf_model::from_yaml::<mushaf_model::RiwayahProfile>(v)
                .unwrap()
                .patch_stack
        })
        .collect();
    assert!(stacked.iter().all(|p| web.contains_key(p)));
    for (k, v) in &disk {
        match web.get(k) {
            Some(w) => assert_eq!(w, v, "{k}"),
            None => assert!(
                k.starts_with("patches/") && !stacked.contains(k),
                "{k} missing from the bundle"
            ),
        }
    }
    assert!(web.keys().all(|k| disk.contains_key(k)));
    for riwayah in ["hafs", "warsh", "qalun"] {
        let (a, b) = (
            Renderer::load(&repo, riwayah).unwrap(),
            Renderer::load(&bundle, riwayah).unwrap(),
        );
        for page in [1, 3, 604] {
            assert_eq!(
                serde_json::to_string(&a.render(page, None).unwrap()).unwrap(),
                serde_json::to_string(&b.render(page, None).unwrap()).unwrap(),
                "{riwayah} p{page}"
            );
        }
    }
}
