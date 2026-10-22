//! `just bench` (T3.6): time the core per page on this machine, release build.
//!
//! Resolve (whole spine, G-035) and line mapping run once; then every page is fitted (shape +
//! justify) and drawn, first with a cold word cache per page, then with one cache shared by
//! all pages, then again on a warm cache. Timing uses the host clock here only; nothing in
//! `core/` reads a clock (rule 8). The 16 ms/page budget is checked on a device in Phase 8.

use std::time::{Duration, Instant};

use mushaf_render::Renderer;

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn stats(label: &str, mut v: Vec<Duration>) {
    v.sort();
    let p = |q: usize| ms(v[(v.len() - 1) * q / 100]);
    println!(
        "| {label} | {:.2} | {:.2} | {:.2} | {:.2} |",
        p(50),
        p(95),
        p(100),
        ms(v.iter().sum::<Duration>()) / v.len() as f64
    );
}

fn main() {
    let repo = std::env::current_dir().unwrap();
    let t = Instant::now();
    let r = Renderer::load(&repo, "hafs").unwrap();
    let load = t.elapsed();
    let t = Instant::now();
    let resolved = r.resolve(None).unwrap();
    let resolve = t.elapsed();
    let t = Instant::now();
    let mapping = r.map(&resolved).unwrap();
    let map = t.elapsed();
    println!(
        "load {:.0} ms (data, font, all outlines); resolve whole spine {:.1} ms; map lines {:.1} ms",
        ms(load),
        ms(resolve),
        ms(map)
    );
    println!();
    println!("| per page (604 pages) | median ms | p95 ms | max ms | mean ms |");
    println!("|---|---|---|---|---|");
    let pages = 1..=604u32;
    let mut cold_fit = Vec::new();
    let mut draw = Vec::new();
    for page in pages.clone() {
        let shaper = r.font.shaper();
        let t = Instant::now();
        let fit = mushaf_render::fit_with(&r, &shaper, &resolved, &mapping, page).unwrap();
        cold_fit.push(t.elapsed());
        let t = Instant::now();
        let d = r.draw(&fit).unwrap();
        let _svg = mushaf_render::svg(&d);
        draw.push(t.elapsed());
    }
    stats("fit (shape + justify), cold cache", cold_fit);
    let shaper = r.font.shaper();
    let mut shared = Vec::new();
    for page in pages.clone() {
        let t = Instant::now();
        mushaf_render::fit_with(&r, &shaper, &resolved, &mapping, page).unwrap();
        shared.push(t.elapsed());
    }
    stats("fit, one cache across pages", shared);
    let mut warm = Vec::new();
    for page in pages {
        let t = Instant::now();
        mushaf_render::fit_with(&r, &shaper, &resolved, &mapping, page).unwrap();
        warm.push(t.elapsed());
    }
    stats("fit, warm cache (page seen before)", warm);
    stats("draw + SVG", draw);
    let s = shaper.cache_stats();
    println!();
    println!(
        "word cache after two passes: {} hits, {} misses",
        s.hits, s.misses
    );
}
