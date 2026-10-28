//! `mushaf-bundle [OUT_DIR]` (T8.2, D57): writes the web app's static data file
//! `<OUT_DIR>/mushaf.bundle` (default `build/web/data`, untracked) and `manifest.json` with its
//! SHA-256. See `bundle_files` for what goes in.

use std::path::PathBuf;
use std::process::ExitCode;

use mushaf_render::bundle::Bundle;
use mushaf_render::bundle_files;
use sha2::{Digest, Sha256};

fn run() -> Result<(), String> {
    let repo = std::env::current_dir().map_err(|e| e.to_string())?;
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("build/web/data"));
    let files = bundle_files(&repo).map_err(|e| e.to_string())?;
    let bytes = Bundle { files }.to_bytes().map_err(|e| e.to_string())?;
    let sha = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    std::fs::write(out.join("mushaf.bundle"), &bytes).map_err(|e| e.to_string())?;
    let manifest =
        serde_json::json!({"file": "mushaf.bundle", "bytes": bytes.len(), "sha256": sha});
    std::fs::write(
        out.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{}: {} bytes, sha256 {sha}",
        out.join("mushaf.bundle").display(),
        bytes.len()
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mushaf-bundle: {e}");
            ExitCode::FAILURE
        }
    }
}
