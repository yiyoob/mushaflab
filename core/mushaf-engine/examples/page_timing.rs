//! Times a session's page steps natively (T8.4 evidence): fit, draw, JSON, SVG.
use std::path::Path;
use std::time::Instant;

use mushaf_engine::Renderer;
use mushaf_engine::session::Session;
use mushaf_layout::fit::FitCache;

fn main() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let r = Renderer::load(&repo, "hafs").unwrap();
    let resolved = r.resolve(None).unwrap();
    let t = Instant::now();
    let mapping = r.map(&resolved).unwrap();
    println!("map: {:?}", t.elapsed());
    let mut cache = FitCache::default();
    let (mut fit_t, mut draw_t, mut json_t, mut svg_t, mut refit_t) = Default::default();
    for p in 10..40 {
        let t = Instant::now();
        let fit = r.fit_cached(&resolved, &mapping, p, &mut cache).unwrap();
        fit_t += t.elapsed();
        let t = Instant::now();
        r.fit_cached(&resolved, &mapping, p, &mut cache).unwrap();
        refit_t += t.elapsed();
        let t = Instant::now();
        let d = r.draw(&fit).unwrap();
        draw_t += t.elapsed();
        let t = Instant::now();
        let j = serde_json::to_string(&d).unwrap();
        json_t += t.elapsed();
        let t = Instant::now();
        let s = mushaf_draw::to_svg_interactive(&d);
        svg_t += t.elapsed();
        if p == 10 {
            println!("json {} bytes, svg {} bytes", j.len(), s.len());
        }
    }
    let n = 30u32;
    let (a, b, c, d, e): (
        std::time::Duration,
        std::time::Duration,
        std::time::Duration,
        std::time::Duration,
        std::time::Duration,
    ) = (fit_t, refit_t, draw_t, json_t, svg_t);
    println!(
        "per page: fit {:?} refit {:?} draw {:?} json {:?} svg {:?}",
        a / n,
        b / n,
        c / n,
        d / n,
        e / n
    );
    let mut s = Session::new(&repo, r, "hijazi");
    s.render(1, false).unwrap();
    let t = Instant::now();
    for p in 10..40 {
        s.margin_notes(p).unwrap();
    }
    println!("margin per page: {:?}", t.elapsed() / n);
}
