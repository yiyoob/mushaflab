//! M1.5 (D73): the engine draws Ḥafṣ without the legacy Ḥafṣ spine and layout files. A source
//! that hides them (and records every file asked for) draws the same page as the checkout.
//! Real data: run by `just golden` (release).

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use mushaf_engine::{Renderer, Source, load_data};
use mushaf_model::{DataFiles, Error};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The checkout without the files the corpus replaces; every `read` is recorded.
struct WithoutLegacyHafs {
    root: PathBuf,
    reads: RefCell<Vec<String>>,
}

fn legacy_hafs(data_key: &str) -> bool {
    data_key.starts_with("spine/hafs/v1/") || data_key.starts_with("layouts/uthmani-1441h.")
}

impl Source for WithoutLegacyHafs {
    fn read(&self, rel: &str) -> Result<Vec<u8>, Error> {
        self.reads.borrow_mut().push(rel.to_string());
        self.root.as_path().read(rel)
    }
    fn data_files(&self) -> Result<DataFiles, Error> {
        let mut f = self.root.as_path().data_files()?;
        f.retain(|k, _| !legacy_hafs(k));
        Ok(f)
    }
    fn corpus_files(&self) -> Result<mushaf_corpus::Files, Error> {
        self.root.as_path().corpus_files()
    }
}

#[test]
#[ignore = "run by `just golden` (release)"]
fn hafs_draws_without_the_legacy_spine_and_layout() {
    let src = WithoutLegacyHafs {
        root: repo(),
        reads: RefCell::new(Vec::new()),
    };
    let data = load_data(&src).unwrap();
    assert!(data.spines.contains_key(mushaf_corpus::legacy::SPINE_ID));
    let r = Renderer::load(&src, "hafs").unwrap();
    let full = Renderer::load(Path::new(&repo()), "hafs").unwrap();
    for page in [1, 2, 255, 604] {
        let a = serde_json::to_string(&r.render(page, None).unwrap()).unwrap();
        let b = serde_json::to_string(&full.render(page, None).unwrap()).unwrap();
        assert_eq!(a, b, "page {page}");
    }
    let legacy: Vec<String> = src
        .reads
        .borrow()
        .iter()
        .filter(|p| p.strip_prefix("data/").is_some_and(legacy_hafs))
        .cloned()
        .collect();
    assert!(legacy.is_empty(), "read {legacy:?}");
}

#[test]
fn the_checkout_loader_skips_the_files_the_corpus_replaces() {
    // load_data must not need them: hiding them changes nothing.
    let src = WithoutLegacyHafs {
        root: repo(),
        reads: RefCell::new(Vec::new()),
    };
    let a = load_data(&src).unwrap();
    let b = load_data(Path::new(&repo())).unwrap();
    assert_eq!(a.spines, b.spines);
    assert_eq!(a.layouts, b.layouts);
}
