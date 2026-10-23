//! `mushaf-contact [riwayah...]` (T5.5, host only; `just contact-sheets`): every page of each
//! riwāyah, small, 24 to a sheet, right to left as a muṣḥaf is read, written to
//! `docs/review/phase5/contact/<riwayah>-NN.png` (untracked: generated images are tracked only
//! under `tests/golden/`). Sheet NN holds pages 24 x (NN - 1) + 1 to 24 x NN. A page that cannot
//! be drawn yet (G-102) is a grey tile with a cross; a page with a pinned rule-6 finding (G-101,
//! G-103) has a red frame. All logic is in `core/` crates (rule 7).

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use mushaf_model::Error;
use mushaf_render::{Renderer, fit_checked, png};
use resvg::tiny_skia::{Color, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform};

const RIWAYAT: [&str; 8] = [
    "hafs",
    "shuba",
    "warsh",
    "qalun",
    "bazzi",
    "qunbul",
    "duri_abuamr",
    "susi",
];
const COLS: u32 = 6;
const ROWS: u32 = 4;
const TILE_W: u32 = 300;
const GAP: u32 = 12;
const OUT: &str = "docs/review/phase5/contact";

fn e(msg: impl std::fmt::Display) -> Error {
    Error::new(msg.to_string())
}

fn frame(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, color: Color, width: f32) {
    let mut paint = Paint::default();
    paint.set_color(color);
    let rect = Rect::from_xywh(x, y, w, h).expect("a rectangle");
    let path = PathBuilder::from_rect(rect);
    let stroke = Stroke {
        width,
        ..Stroke::default()
    };
    pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

fn blocked_tile(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32) {
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(200, 200, 200, 255));
    pm.fill_rect(
        Rect::from_xywh(x, y, w, h).expect("a rectangle"),
        &paint,
        Transform::identity(),
        None,
    );
    let mut pb = PathBuilder::new();
    pb.move_to(x, y);
    pb.line_to(x + w, y + h);
    pb.move_to(x + w, y);
    pb.line_to(x, y + h);
    let path = pb.finish().expect("a cross");
    paint.set_color(Color::from_rgba8(120, 120, 120, 255));
    pm.stroke_path(
        &path,
        &paint,
        &Stroke {
            width: 3.0,
            ..Stroke::default()
        },
        Transform::identity(),
        None,
    );
}

fn sheets(repo: &Path, riwayah: &str) -> Result<usize, Error> {
    let r = Renderer::load(repo, riwayah)?;
    let resolved = r.resolve(None)?;
    let mapping = r.map(&resolved)?;
    let shaper = r.font.shaper();
    let per = COLS * ROWS;
    let mut tile_h = 0u32;
    let mut sheet: Option<Pixmap> = None;
    let mut written = 0;
    for page in 1..=604u32 {
        let slot = (page - 1) % per;
        let cp = fit_checked(&r, &shaper, &resolved, &mapping, page)?;
        let tile = match &cp.fit {
            Some(fit) => {
                let d = r.draw(fit)?;
                let bytes = png(&mushaf_render::svg(&d), TILE_W)?;
                Some(Pixmap::decode_png(&bytes).map_err(e)?)
            }
            None => None,
        };
        if tile_h == 0 {
            tile_h = tile.as_ref().map_or(TILE_W * 3 / 2, Pixmap::height);
        }
        if slot == 0 {
            let w = COLS * TILE_W + (COLS + 1) * GAP;
            let h = ROWS * tile_h + (ROWS + 1) * GAP;
            let mut pm = Pixmap::new(w, h).ok_or_else(|| e("empty sheet"))?;
            pm.fill(Color::WHITE);
            sheet = Some(pm);
        }
        let pm = sheet.as_mut().expect("a sheet");
        // right to left within a row
        let col = COLS - 1 - slot % COLS;
        let row = slot / COLS;
        let x = (GAP + col * (TILE_W + GAP)) as f32;
        let y = (GAP + row * (tile_h + GAP)) as f32;
        match tile {
            Some(t) => {
                pm.draw_pixmap(
                    x as i32,
                    y as i32,
                    t.as_ref(),
                    &PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
                let flagged = !cp.violations.is_empty();
                let (c, w) = if flagged {
                    (Color::from_rgba8(220, 0, 0, 255), 6.0)
                } else {
                    (Color::from_rgba8(180, 180, 180, 255), 1.0)
                };
                frame(pm, x, y, TILE_W as f32, tile_h as f32, c, w);
            }
            None => blocked_tile(pm, x, y, TILE_W as f32, tile_h as f32),
        }
        if slot == per - 1 || page == 604 {
            let n = (page - 1) / per + 1;
            let path = repo.join(OUT).join(format!("{riwayah}-{n:02}.png"));
            fs::create_dir_all(path.parent().expect("a directory")).map_err(e)?;
            let bytes = sheet.take().expect("a sheet").encode_png().map_err(e)?;
            fs::write(&path, bytes).map_err(e)?;
            written += 1;
        }
    }
    Ok(written)
}

fn run() -> Result<(), Error> {
    let repo = std::env::current_dir().map_err(e)?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let list: Vec<&str> = if args.is_empty() {
        RIWAYAT.to_vec()
    } else {
        args.iter().map(String::as_str).collect()
    };
    for riwayah in list {
        let n = sheets(&repo, riwayah)?;
        println!("{riwayah}: {n} sheets in {OUT}");
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("mushaf-contact: {err}");
            ExitCode::FAILURE
        }
    }
}
