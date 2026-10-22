use harfrust::{Buffer, ClusterLevel, Feature, Font, ShapeOptions, ShaperFont, Tag};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let font = Font::new(std::fs::read(&a[1]).unwrap(), 0).unwrap();
    let shaper = ShaperFont::new(&font);
    let text = std::fs::read_to_string(&a[2]).unwrap();
    let cv01 = Tag::new(b"cv01");
    for (label, feats) in [
        ("none", vec![]),
        ("cv01=1 global", vec![Feature::new(cv01, 1, ..)]),
        ("cv01=1 5..6", vec![Feature::new(cv01, 1, 5..6)]),
        ("cv01=2 global", vec![Feature::new(cv01, 2, ..)]),
        ("cv01=1 5..=5", vec![Feature::new(cv01, 1, 5..=5)]),
        ("cv01=1 5..=6", vec![Feature::new(cv01, 1, 5..=6)]),
        ("cv01=1 5..7", vec![Feature::new(cv01, 1, 5..7)]),
        ("struct start5 end6", vec![Feature { tag: cv01, value: 1, start: 5, end: 6 }]),
    ] {
        let mut b = Buffer::new();
        for (i, ch) in text.chars().enumerate() { b.push(ch as u32, i as u32); }
        b.guess_segment_properties();
        b.set_cluster_level(ClusterLevel::MonotoneCharacters);
        harfrust::shape(&shaper, &mut b, ShapeOptions::new().features(&feats)).unwrap();
        let g: Vec<(u32, i32)> = b.glyph_infos().iter().zip(b.glyph_positions()).map(|(i, p)| (i.glyph_id, p.x_advance)).take(5).collect();
        println!("harfrust {label}: {g:?}");
    }
}
