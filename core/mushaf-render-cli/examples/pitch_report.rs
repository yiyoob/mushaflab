//! Ink collisions between adjacent lines on all 604 pages, and the tightest safe line pitch
//! (D15). `cargo run --release -p mushaf-render-cli --example pitch_report`
use mushaf_render::Renderer;
use mushaf_render::collide::line_pairs;

fn main() {
    let r = Renderer::load(&std::env::current_dir().unwrap(), "hafs").unwrap();
    let pitch = r.profile.geometry.line_pitch;
    let res = r.resolve(None).unwrap();
    let map = r.map(&res).unwrap();
    let mut all = Vec::new();
    for page in 1..=604 {
        let d = r.draw(&r.fit(&res, &map, page).unwrap()).unwrap();
        all.extend(line_pairs(&d, pitch));
    }
    let colliding: Vec<_> = all.iter().filter(|p| p.collides).collect();
    println!(
        "line pitch {pitch}: {} adjacent pairs, {} collide",
        all.len(),
        colliding.len()
    );
    for p in &colliding {
        println!(
            "  collides: page {} lines {}-{} (free from {})",
            p.page, p.upper, p.lower, p.tightest_pitch
        );
    }
    let mut t: Vec<_> = all
        .iter()
        .map(|p| (p.tightest_pitch, p.page, p.upper))
        .collect();
    t.sort();
    let n = t.len();
    println!(
        "tightest collision-free pitch per pair: p50 {} p90 {} p99 {} max {}",
        t[n / 2].0,
        t[n * 9 / 10].0,
        t[n * 99 / 100].0,
        t[n - 1].0
    );
    println!("binding pairs:");
    for x in t.iter().rev().take(8) {
        println!("  page {} lines {}-{}: needs {}", x.1, x.2, x.2 + 1, x.0);
    }
    for cand in [1700, 1750, 1780, 1800, 1850, 1900] {
        println!(
            "pitch {cand}: {} pairs would collide",
            t.iter().filter(|x| x.0 > cand).count()
        );
    }
}
