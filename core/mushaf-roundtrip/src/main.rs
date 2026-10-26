//! T4.6 round-trip (D22): resolves each riwāyah through its profile (T6.4: its rule files and
//! its patch stack; before T6.4 the raw patch file) on the Ḥafṣ spine in
//! `Mode::Data` (any orphan, unresolved, collision or other report entry is an error) and
//! writes the resolved items as JSON lines for `pipeline.roundtrip`, which compares them with
//! the pinned KFGQPC text codepoint for codepoint. Host only: reads the data root from disk.
//!
//!     mushaf-roundtrip <data root> <out dir> <riwayah>...

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mushaf_model::{Cps, DataSet, ItemKind, ItemRef, to_json};
use mushaf_resolve::{Mode, ResolveError, Selection, resolve_profile};
use serde::Serialize;

/// Report entries shown when a resolution fails; the count is always shown.
const SHOWN: usize = 20;

#[derive(Serialize)]
struct Line<'a> {
    id: &'a ItemRef,
    kind: ItemKind,
    /// The item's full text (separators and clusters) as `U+XXXX` codepoints.
    text: Cps,
}

fn describe(e: &ResolveError) -> String {
    match e {
        ResolveError::Strict(report) => {
            let mut s = format!("{} report entries in data mode", report.len());
            for d in report.iter().take(SHOWN) {
                let _ = write!(s, "\n  {d:?}");
            }
            s
        }
        other => other.to_string(),
    }
}

fn run(ds: &DataSet, riwayah: &str, out: &Path) -> Result<usize, String> {
    let sel = Selection {
        riwayah: riwayah.into(),
        ..Selection::default()
    };
    let r = resolve_profile(ds, &sel, None, Mode::Data)
        .map_err(|e| format!("profile {riwayah}: {}", describe(&e)))?;
    let mut text = String::new();
    for item in &r.items {
        let line = Line {
            id: &item.id,
            kind: item.kind,
            text: Cps::from_text(&item.text()),
        };
        text += &to_json(&line).map_err(|e| e.to_string())?;
        text.push('\n');
    }
    let target = out.join(format!("{riwayah}.jsonl"));
    std::fs::write(&target, text).map_err(|e| format!("{}: {e}", target.display()))?;
    Ok(r.items.len())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [root, out, riwayat @ ..] = args.as_slice() else {
        eprintln!("usage: mushaf-roundtrip <data root> <out dir> <riwayah>...");
        return ExitCode::from(2);
    };
    if riwayat.is_empty() {
        eprintln!("usage: mushaf-roundtrip <data root> <out dir> <riwayah>...");
        return ExitCode::from(2);
    }
    let ds = match mushaf_db::load_dir(Path::new(root)) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("loading {root}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let out = PathBuf::from(out);
    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("{}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    let mut ok = true;
    for r in riwayat {
        match run(&ds, r, &out) {
            Ok(n) => println!("{r}: resolved {n} items in data mode"),
            Err(e) => {
                eprintln!("{r}: {e}");
                ok = false;
            }
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
