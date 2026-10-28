//! Reading a data root's files from disk (D12, D56). Plain `std::fs`, so it builds for every
//! target; on the web the same files come from the static bundle instead (D57).

use std::fs;
use std::path::Path;

use crate::{DATA_DIRS, DataFiles, Error};

fn io(path: &Path, e: impl std::fmt::Display) -> Error {
    Error::new(format!("{}: {e}", path.display()))
}

fn walk(root: &Path, dir: &Path, out: &mut DataFiles) -> Result<(), Error> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| io(dir, e))?
        .map(|e| e.map(|e| e.path()).map_err(|e| io(dir, e)))
        .collect::<Result<_, _>>()?;
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(root, &p, out)?;
        } else {
            let rel = p.strip_prefix(root).map_err(|e| io(&p, e))?;
            let key = rel
                .iter()
                .map(|c| c.to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.insert(key, fs::read_to_string(&p).map_err(|e| io(&p, e))?);
        }
    }
    Ok(())
}

/// Every file under the data root's data directories (`spine/`, `layouts/`, ...).
pub fn read_tree(root: &Path) -> Result<DataFiles, Error> {
    let mut files = DataFiles::new();
    for d in DATA_DIRS {
        let dir = root.join(d);
        if dir.is_dir() {
            walk(root, &dir, &mut files)?;
        }
    }
    Ok(files)
}
