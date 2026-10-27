//! Review aid: every word of a Maghribi riwāyah after the Hijazi conventions (D46), as
//! `id<TAB>cluster|cluster|...` with clusters written `U+XXXX U+XXXX`.
//!
//!     cargo run --release --example hijazi_dump -- <repo> <riwayah>

use std::path::Path;

use mushaf_model::{Cps, ItemKind};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [repo, riwayah] = args.as_slice() else {
        eprintln!("usage: hijazi_dump <repo> <riwayah>");
        std::process::exit(2);
    };
    let r = mushaf_render::Renderer::load_style(Path::new(repo), riwayah, "hijazi").unwrap();
    for it in r.base.items.iter().filter(|i| i.kind == ItemKind::Word) {
        let cl: Vec<String> = it
            .clusters
            .iter()
            .map(|c| Cps(c.cps.0.clone()).to_string())
            .collect();
        println!("{}\t{}", it.id, cl.join("|"));
    }
}
