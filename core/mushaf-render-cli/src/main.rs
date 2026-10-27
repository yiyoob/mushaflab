//! `mushaf-render --page N --riwayah ID [--style STYLE] [--overlay FILE] [--out DIR] [--width PX]
//! [--highlight]`
//!
//! `--highlight` draws the clusters that differ from Ḥafṣ in red (D45).
//!
//! Writes `<out>/<riwayah>/p<NNN>.svg`, `.png` and `.json` (the draw commands, D4). The default
//! output directory is `build/render` (untracked).

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use mushaf_model::EditOverlay;
use mushaf_render::{Renderer, png};

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (mut page, mut riwayah, mut overlay, mut out, mut width) =
        (None, "hafs".to_string(), None, None, 1200u32);
    let mut style: Option<String> = None;
    let mut highlight = false;
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--page" => page = Some(val()?.parse::<u32>().map_err(|e| e.to_string())?),
            "--riwayah" => riwayah = val()?,
            "--style" => style = Some(val()?),
            "--highlight" => highlight = true,
            "--overlay" => overlay = Some(PathBuf::from(val()?)),
            "--out" => out = Some(PathBuf::from(val()?)),
            "--width" => {
                width = val()?
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?
            }
            _ => return Err(format!("unknown argument {a}")),
        }
    }
    let page = page.ok_or("--page is required")?;
    if !(1..=604).contains(&page) {
        return Err(format!("page {page} is not in 1..604"));
    }
    let repo = std::env::current_dir().map_err(|e| e.to_string())?;
    let renderer = match &style {
        Some(s) => Renderer::load_style(&repo, &riwayah, s),
        None => Renderer::load(&repo, &riwayah),
    }
    .map_err(|e| e.to_string())?;
    let overlay = match overlay {
        Some(p) => {
            let text = fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            Some(mushaf_model::from_yaml::<EditOverlay>(&text).map_err(|e| e.to_string())?)
        }
        None => None,
    };
    let draw = if highlight {
        if overlay.is_some() {
            return Err("--highlight does not take an overlay".into());
        }
        renderer.render_highlighted(page, &Default::default(), Default::default())
    } else {
        renderer.render(page, overlay.as_ref())
    }
    .map_err(|e| e.to_string())?;
    let svg = mushaf_render::svg(&draw);
    let dir = out
        .unwrap_or_else(|| repo.join("build/render"))
        .join(&riwayah);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let base = dir.join(format!("p{page:03}"));
    fs::write(base.with_extension("svg"), &svg).map_err(|e| e.to_string())?;
    fs::write(
        base.with_extension("png"),
        png(&svg, width).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        base.with_extension("json"),
        serde_json::to_string(&draw).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let squeezed = draw.lines.iter().filter(|l| l.squeeze.is_some()).count();
    println!(
        "{}: page {page}, scale {}, {} lines ({squeezed} squeezed), {}{}",
        base.with_extension("svg").display(),
        mushaf_draw::fmt_16_16(draw.scale),
        draw.lines.len(),
        if draw.canonical {
            "canonical"
        } else {
            "EDITED"
        },
        if draw.notes.is_empty() {
            String::new()
        } else {
            format!(", {} notes", draw.notes.len())
        }
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mushaf-render: {e}");
            ExitCode::FAILURE
        }
    }
}
