//! Review sheet for the owner (G-102, D47): every cluster form of Warsh and Qālūn that the
//! Ḥijāzī style cannot draw yet, with a proposal per form for the owner to accept or reject.
//!
//!     cargo run --release --example hijazi_review -- <repo>
//!
//! Writes `build/review/phase8/hijazi-forms.html` (untracked) and `.json`. Nothing is decided
//! here: each word is drawn in DigitalKhatt exactly as written (no letter or mark replaced),
//! next to the word in its own Maghribi font and the Ḥafṣ word, and checked against rule 6.
//! Forms the owner accepts become rows of `data/encodings/hafs-v1.hijazi-forms.json` (D47) only
//! on the owner's instruction.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;

use mushaf_engine::Source;
use mushaf_engine::session::Session;
use mushaf_model::{ClusterRef, Cps, DataSet, ItemKind};
use mushaf_render::Renderer;

struct Place {
    cluster: ClusterRef,
    word: Vec<Vec<char>>,
    hafs: Option<Vec<char>>,
    hafs_word: Vec<Vec<char>>,
    page: Option<u32>,
}

fn cps(c: &[char]) -> String {
    Cps(c.to_vec()).to_string()
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;")
}

fn main() {
    let repo = std::env::args().nth(1).expect("usage: hijazi_review <repo>");
    let repo = Path::new(&repo);
    let data = Arc::new(DataSet::from_files(&repo.data_files().unwrap()).unwrap());
    let hafs = Session::new(
        repo,
        Renderer::load_with(repo, data.clone(), "hafs", "hijazi").unwrap(),
        "hijazi",
    );
    let spine = data.spines["hafs/v1"].clone();
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>Hijazi forms review</title><style>\
         :root{color-scheme:light}body{font:14px system-ui;margin:24px;max-width:1400px;background:#fff;color:#000}svg{color:#000}table{border-collapse:collapse}\
         td,th{border:1px solid #ccc;padding:4px 8px;vertical-align:middle}svg{height:56px;width:auto}\
         .bad{color:#b00}.ok{color:#070}code{font-size:12px}</style>\
         <h1>Ḥijāzī style: forms that cannot be drawn yet (G-102)</h1>\
         <p>Per form: the places; the example word in its own Maghribi font (KFGQPC); the same \
         word drawn in DigitalKhatt exactly as written (clusters with a conversion row converted, \
         the blocked cluster given to the font as its own codepoints; no letter or mark \
         replaced); and the Ḥafṣ word for comparison. Rule 6: clean means no .notdef, dotted \
         circle or unattached mark; anything else is how the font fails on it. Pages: blocked \
         pages that contain this form. Answer per form: draw it as shown, or keep blocked (or \
         describe another drawing).</p>",
    );
    let mut report = serde_json::Map::new();
    for riwayah in ["warsh", "qalun"] {
        let own = Session::new(
            repo,
            Renderer::load_with(repo, data.clone(), riwayah, "maghribi").unwrap(),
            "maghribi",
        );
        let hij = Session::new(
            repo,
            Renderer::load_with(repo, data.clone(), riwayah, "hijazi").unwrap(),
            "hijazi",
        );
        let r = &hij.renderer;
        // the page each word is on, from the riwāyah's own layout
        let mut page_of: BTreeMap<String, u32> = BTreeMap::new();
        let resolved = r.resolve(None).unwrap();
        let mapping = r.map(&resolved).unwrap();
        for l in &mapping.lines {
            for i in l.items.clone() {
                page_of.insert(resolved.items[i].id.to_string(), l.page);
            }
        }
        let mut forms: BTreeMap<Vec<char>, Vec<Place>> = BTreeMap::new();
        for it in r.base.items.iter().filter(|i| i.kind == ItemKind::Word) {
            let word: Vec<Vec<char>> = it.clusters.iter().map(|c| c.cps.0.clone()).collect();
            let refs: Vec<&[char]> = word.iter().map(Vec::as_slice).collect();
            if r.table.convert(&refs).is_ok() {
                continue;
            }
            for c in &it.clusters {
                if r.table.convert(&[c.cps.0.as_slice()]).is_ok() {
                    continue;
                }
                let hafs_word = spine
                    .item(&c.id.item)
                    .map(|w| w.clusters.iter().map(|x| x.cps.0.clone()).collect())
                    .unwrap_or_default();
                forms.entry(c.cps.0.clone()).or_default().push(Place {
                    cluster: c.id.clone(),
                    word: word.clone(),
                    hafs: spine.cluster(&c.id).map(|h| h.cps.0.clone()),
                    hafs_word,
                    page: page_of.get(&it.id.to_string()).copied(),
                });
            }
        }
        let blocked: BTreeSet<u32> = forms.values().flatten().filter_map(|p| p.page).collect();
        let total: usize = forms.values().map(Vec::len).sum();
        let _ = write!(
            html,
            "<h2>{riwayah}: {} forms, {total} places, {} blocked pages</h2><table>\
             <tr><th>#</th><th>Form</th><th>Places</th><th>Ḥafṣ at those places</th>\
             <th>Example</th><th>Maghribi (own font)</th><th>Ḥijāzī: as written, in DigitalKhatt</th>\
             <th>Rule 6</th><th>Ḥafṣ (for comparison)</th><th>Pages</th></tr>",
            forms.len(),
            blocked.len()
        );
        let mut rows = Vec::new();
        let mut sorted: Vec<_> = forms.iter().collect();
        sorted.sort_by_key(|(f, ps)| (std::cmp::Reverse(ps.len()), (*f).clone()));
        for (n, (form, places)) in sorted.iter().enumerate() {
            let mut hafs_forms: BTreeMap<String, usize> = BTreeMap::new();
            for p in places.iter() {
                *hafs_forms
                    .entry(p.hafs.as_deref().map_or("(inserted)".into(), cps))
                    .or_default() += 1;
            }
            let ex = &places[0];
            let svg = |s: &Session, w: &[Vec<char>]| -> Result<String, String> {
                let refs: Vec<&[char]> = w.iter().map(Vec::as_slice).collect();
                s.draw_word(&refs).map(|d| d.to_svg()).map_err(|e| e.to_string())
            };
            let own_svg = svg(&own, &ex.word).unwrap_or_else(|e| format!("<i>{}</i>", esc(&e)));
            // The word as written, in DigitalKhatt: each cluster through its conversion row
            // where it has one, the blocked clusters as their own codepoints. Nothing is
            // replaced; rule 6 says whether the font draws it cleanly.
            let (prop_svg, rule6) = {
                let mut chars = Vec::new();
                for c in &ex.word {
                    match r.table.convert(&[c.as_slice()]) {
                        Ok(conv) => chars.extend(conv.chars),
                        Err(_) => chars.extend(c.iter().copied()),
                    }
                }
                let shaper = r.font.shaper();
                let g = shaper.shape(&chars, &[]).unwrap();
                let v = mushaf_shape::check_word(&r.font, &shaper, &chars, &[], &g).unwrap();
                let mut glyphs = BTreeMap::new();
                let mut placed = Vec::new();
                let mut x = 0i64;
                for s in &g {
                    if let Ok(o) = r.outlines.get(s.glyph) {
                        glyphs.entry(s.glyph).or_insert(o.to_vec());
                        placed.push(mushaf_engine::session::Placed {
                            glyph: s.glyph,
                            x: x + i64::from(s.x_offset),
                            y: i64::from(s.y_offset),
                        });
                    }
                    x += i64::from(s.x_advance);
                }
                let w = mushaf_engine::session::DrawWord {
                    upem: i64::from(r.font.units_per_em()),
                    advance: x,
                    glyphs,
                    placed,
                };
                (
                    w.to_svg(),
                    if v.is_empty() {
                        "<span class=ok>clean</span>".to_string()
                    } else {
                        format!("<span class=bad>{}</span>", esc(&format!("{v:?}")))
                    },
                )
            };
            let hafs_svg = svg(&hafs, &ex.hafs_word).unwrap_or_default();
            let pages: BTreeSet<u32> = places.iter().filter_map(|p| p.page).collect();
            let ids: Vec<String> = places.iter().map(|p| p.cluster.to_string()).collect();
            let _ = write!(
                html,
                "<tr><td>{}</td><td><code>{}</code></td><td>{} <details><summary>ids</summary>\
                 <code>{}</code></details></td><td><code>{}</code></td><td><code>{}</code></td>\
                 <td>{own_svg}</td><td>{prop_svg}</td><td>{rule6}</td><td>{hafs_svg}</td><td>{}</td></tr>",
                n + 1,
                cps(form),
                places.len(),
                ids.join(" "),
                esc(&format!("{hafs_forms:?}")),
                ex.cluster,
                pages.len()
            );
            rows.push(serde_json::json!({
                "n": n + 1, "form": cps(form), "places": ids, "hafs": hafs_forms,
                "pages": pages, "rule6_clean": rule6.contains("clean"),
            }));
        }
        html.push_str("</table>");
        report.insert(riwayah.into(), serde_json::json!({"blocked_pages": blocked, "forms": rows}));
    }
    let out = repo.join("build/review/phase8");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("hijazi-forms.html"), html).unwrap();
    std::fs::write(
        out.join("hijazi-forms.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", out.join("hijazi-forms.html").display());
}
