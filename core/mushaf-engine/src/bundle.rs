//! The web app's static data file (D57): the repository files the engine reads, byte for byte,
//! in one archive. It is a `Source`, so the web runs the same loading code as the CLI.
//!
//! Format: the magic `MUSHAFB1`, a little-endian `u32` file count, then per file a `u32` path
//! length, the UTF-8 path (repository-relative, `/` separators), a `u64` length and the bytes.
//! Paths are written sorted and must be unique.

use std::collections::BTreeMap;

use mushaf_model::{DATA_DIRS, DataFiles, Error};

use crate::Source;

pub const MAGIC: &[u8; 8] = b"MUSHAFB1";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bundle {
    pub files: BTreeMap<String, Vec<u8>>,
}

fn take<'a>(b: &mut &'a [u8], n: usize) -> Result<&'a [u8], Error> {
    if b.len() < n {
        return Err(Error::new("bundle: truncated"));
    }
    let (head, rest) = b.split_at(n);
    *b = rest;
    Ok(head)
}

fn len(bytes: &[u8]) -> Result<usize, Error> {
    let mut a = [0u8; 8];
    a[..bytes.len()].copy_from_slice(bytes);
    usize::try_from(u64::from_le_bytes(a)).map_err(|_| Error::new("bundle: length overflow"))
}

impl Bundle {
    pub fn parse(mut b: &[u8]) -> Result<Bundle, Error> {
        if take(&mut b, 8)? != MAGIC {
            return Err(Error::new("bundle: bad magic"));
        }
        let count = len(take(&mut b, 4)?)?;
        let mut files = BTreeMap::new();
        for _ in 0..count {
            let n = len(take(&mut b, 4)?)?;
            let path = std::str::from_utf8(take(&mut b, n)?)
                .map_err(|e| Error::new(format!("bundle: path: {e}")))?
                .to_string();
            let n = len(take(&mut b, 8)?)?;
            let data = take(&mut b, n)?.to_vec();
            if files.insert(path.clone(), data).is_some() {
                return Err(Error::new(format!("bundle: duplicate path {path}")));
            }
        }
        if !b.is_empty() {
            return Err(Error::new("bundle: trailing bytes"));
        }
        Ok(Bundle { files })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut out = MAGIC.to_vec();
        let n32 = |n: usize| {
            u32::try_from(n)
                .map(u32::to_le_bytes)
                .map_err(|_| Error::new("bundle: too large"))
        };
        out.extend(n32(self.files.len())?);
        for (path, data) in &self.files {
            out.extend(n32(path.len())?);
            out.extend(path.as_bytes());
            out.extend((data.len() as u64).to_le_bytes());
            out.extend(data);
        }
        Ok(out)
    }
}

impl Source for Bundle {
    fn read(&self, rel: &str) -> Result<Vec<u8>, Error> {
        self.files
            .get(rel)
            .cloned()
            .ok_or_else(|| Error::new(format!("{rel}: not in the bundle")))
    }

    fn data_files(&self) -> Result<DataFiles, Error> {
        let mut out = DataFiles::new();
        for (path, data) in &self.files {
            let Some(rel) = path.strip_prefix("data/") else {
                continue;
            };
            if !DATA_DIRS.iter().any(|d| rel.starts_with(&format!("{d}/"))) {
                continue;
            }
            let text =
                String::from_utf8(data.clone()).map_err(|e| Error::new(format!("{path}: {e}")))?;
            out.insert(rel.to_string(), text);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_rejects_damage() {
        let mut b = Bundle::default();
        b.files
            .insert("data/options/x.yaml".into(), b"a: 1\n".to_vec());
        b.files.insert("data/render/p.json".into(), b"{}".to_vec());
        b.files.insert("fonts/f.otf".into(), vec![0, 1, 2, 255]);
        let bytes = b.to_bytes().unwrap();
        assert_eq!(Bundle::parse(&bytes).unwrap(), b);
        assert!(Bundle::parse(&bytes[..bytes.len() - 1]).is_err());
        let mut extra = bytes.clone();
        extra.push(0);
        assert!(Bundle::parse(&extra).is_err());
        assert!(Bundle::parse(b"NOTABUND\0\0\0\0").is_err());
        // only the data root's data directories are data files, keyed under `data/`
        let files = b.data_files().unwrap();
        assert_eq!(files.keys().collect::<Vec<_>>(), ["options/x.yaml"]);
        assert_eq!(b.read("fonts/f.otf").unwrap(), [0, 1, 2, 255]);
        assert!(b.read("fonts/missing.otf").is_err());
    }
}
