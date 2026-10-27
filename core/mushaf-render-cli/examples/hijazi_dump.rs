//! Review aid: every word of a Maghribi riwāyah after the Hijazi conventions (D46), as
//! `id<TAB>cluster|cluster|...` with clusters written `U+XXXX U+XXXX`.
//!
//!     cargo run --release --example hijazi_dump -- <repo> <riwayah> [unmapped]
//!
//! With `unmapped`, prints instead each word that has no encoding row, with the cluster and
//! its id.

use std::path::Path;

use mushaf_model::{Cps, ItemKind};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (repo, riwayah, unmapped) = match args.as_slice() {
        [a, b] => (a, b, false),
        [a, b, m] if m == "unmapped" => (a, b, true),
        _ => {
            eprintln!("usage: hijazi_dump <repo> <riwayah> [unmapped]");
            std::process::exit(2);
        }
    };
    let r = mushaf_render::Renderer::load_style(Path::new(repo), riwayah, "hijazi").unwrap();
    for it in r.base.items.iter().filter(|i| i.kind == ItemKind::Word) {
        let cl: Vec<String> = it
            .clusters
            .iter()
            .map(|c| Cps(c.cps.0.clone()).to_string())
            .collect();
        if unmapped {
            let refs: Vec<&[char]> = it.clusters.iter().map(|c| c.cps.0.as_slice()).collect();
            if let Err(e) = r.table.convert_item(&it.id.to_string(), &refs) {
                // the cluster id of the first cluster with these codepoints
                let at = it
                    .clusters
                    .iter()
                    .find(|c| Cps(c.cps.0.clone()).to_string() == e.cps.to_string())
                    .map(|c| c.id.to_string())
                    .unwrap_or_default();
                println!("{}\t{}\t{}\t{at}", it.id, e.cps, cl.join("|"));
            }
            continue;
        }
        println!("{}\t{}", it.id, cl.join("|"));
    }
}
