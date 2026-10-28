//! Times the stages of `Renderer::load` (D57 evidence).
use std::path::Path;
use std::time::Instant;

use mushaf_engine::Source;

fn main() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let t = Instant::now();
    let files = repo.data_files().unwrap();
    let bytes: usize = files.values().map(String::len).sum();
    println!(
        "read {} files, {bytes} bytes: {:?}",
        files.len(),
        t.elapsed()
    );
    let mut by_dir: std::collections::BTreeMap<String, usize> = Default::default();
    for (k, v) in &files {
        let d: Vec<&str> = k.splitn(3, '/').collect();
        *by_dir.entry(d[..d.len().min(2)].join("/")).or_default() += v.len();
    }
    for (d, n) in by_dir.iter().filter(|(_, n)| **n > 1_000_000) {
        println!("  {d}: {n}");
    }
    for keep in ["warsh", "hafs"] {
        let sub: mushaf_model::DataFiles = files
            .iter()
            .filter(|(k, _)| !k.starts_with("patches/") || !k.starts_with("patches/raw/"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let t = Instant::now();
        let r = mushaf_model::DataSet::from_files(&sub);
        println!("parse subset {keep}: {:?} err={:?}", t.elapsed(), r.err());
    }
    let t = Instant::now();
    let ds = mushaf_model::DataSet::from_files(&files).unwrap();
    println!("parse: {:?}", t.elapsed());
    drop(ds);
    let t = Instant::now();
    let r = mushaf_engine::Renderer::load(&repo, "warsh").unwrap();
    println!("load warsh: {:?}", t.elapsed());
    let t = Instant::now();
    r.render(3, None).unwrap();
    println!("render p3: {:?}", t.elapsed());
}
