//! WebAssembly bindings (T8.1, D5, D56). The web app calls these and draws what they return
//! (rule 7): every answer is JSON from the same serde code the CLI uses, so a page's draw list
//! is byte-identical to `mushaf-render`'s `.json`. The data come from the static bundle (D57).
//!
//! `Api` is plain Rust (tested natively); `Engine` only wraps it for JavaScript.

use std::collections::BTreeMap;
use std::sync::Arc;

use mushaf_engine::bundle::Bundle;
use mushaf_engine::session::{EditOp, Session};
use mushaf_engine::{Renderer, Source};
use mushaf_model::DataSet;
use serde::Serialize;
use wasm_bindgen::prelude::*;

type Res<T> = Result<T, String>;

fn json<T: Serialize>(t: &T) -> Res<String> {
    serde_json::to_string(t).map_err(|e| e.to_string())
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
struct View<'a> {
    page: u32,
    riwayah: &'a str,
    canonical: bool,
    /// Page height and each line's baseline (26.6), to place margin notes by their line.
    height: i64,
    baselines: Vec<(u32, i64)>,
    svg: String,
    highlights: &'a [mushaf_draw::Highlight],
}

#[derive(Serialize)]
struct EditState {
    edits: usize,
    can_undo: bool,
    can_redo: bool,
}

pub struct Api {
    bundle: Bundle,
    data: Arc<DataSet>,
    session: Option<Session>,
}

impl Api {
    pub fn new(bytes: &[u8]) -> Res<Api> {
        let bundle = Bundle::parse(bytes).map_err(err)?;
        let data = Arc::new(DataSet::from_files(&bundle.data_files().map_err(err)?).map_err(err)?);
        Ok(Api {
            bundle,
            data,
            session: None,
        })
    }

    fn s(&mut self) -> Res<&mut Session> {
        self.session
            .as_mut()
            .ok_or_else(|| "no riwayah selected".to_string())
    }

    pub fn styles(&self) -> Res<String> {
        json(&mushaf_engine::styles(&self.bundle, &self.data).map_err(err)?)
    }

    /// Opens a riwāyah in a style; refused unless `styles()` offers it (D44).
    pub fn select(&mut self, riwayah: &str, style: &str) -> Res<()> {
        let r =
            Renderer::load_with(&self.bundle, self.data.clone(), riwayah, style).map_err(err)?;
        self.session = Some(Session::new(&self.bundle, r, style));
        Ok(())
    }

    pub fn pages(&mut self) -> Res<u32> {
        Ok(self.s()?.pages())
    }

    pub fn render(&mut self, page: u32, highlighted: bool) -> Res<String> {
        json(&self.s()?.render(page, highlighted).map_err(err)?)
    }

    pub fn option_groups(&mut self) -> Res<String> {
        json(&self.s()?.option_groups())
    }

    pub fn set_choices(&mut self, choices: &str) -> Res<()> {
        let c: BTreeMap<String, String> = serde_json::from_str(choices).map_err(err)?;
        self.s()?.set_choices(c).map_err(err)
    }

    /// Margin notes; each note's `word` is the core-drawn word as SVG (D59, D60).
    pub fn margin_notes(&mut self, page: u32) -> Res<String> {
        let notes = self.s()?.margin_notes(page).map_err(err)?;
        let mut out = Vec::new();
        for n in &notes {
            let mut v = serde_json::to_value(n).map_err(err)?;
            v["word"] = n
                .word
                .as_ref()
                .map_or(serde_json::Value::Null, |w| w.to_svg().into());
            out.push(v);
        }
        json(&out)
    }

    /// What the page view shows (D59): the core's SVG with cluster attributes, and the page's
    /// highlights for tap-for-detail. The draw list is the one `render` returns.
    pub fn view(&mut self, page: u32, highlighted: bool) -> Res<String> {
        let d = self.s()?.render(page, highlighted).map_err(err)?;
        json(&View {
            page: d.page,
            riwayah: &d.riwayah,
            canonical: d.canonical,
            height: d.height,
            baselines: d.lines.iter().map(|l| (l.line, l.baseline)).collect(),
            svg: mushaf_draw::to_svg_interactive(&d),
            highlights: &d.highlights,
        })
    }

    pub fn inspect(&mut self, page: u32) -> Res<String> {
        json(&self.s()?.inspect(page).map_err(err)?)
    }

    pub fn provenance(&mut self) -> Res<String> {
        json(&self.s()?.provenance())
    }

    pub fn edit(&mut self, op: &str) -> Res<()> {
        let op: EditOp = serde_json::from_str(op).map_err(err)?;
        self.s()?.edit(op).map_err(err)
    }

    pub fn edit_state(&mut self) -> Res<String> {
        let s = self.s()?;
        let edits = s.edit_count();
        json(&EditState {
            edits,
            can_undo: s.can_undo(),
            can_redo: s.can_redo(),
        })
    }

    pub fn export_edits(&mut self) -> Res<String> {
        self.s()?.export_edits().map_err(err)
    }

    pub fn import_edits(&mut self, text: &str) -> Res<()> {
        self.s()?.import_edits(text).map_err(err)
    }

    pub fn lint(&mut self) -> Res<String> {
        json(&self.s()?.lint().map_err(err)?)
    }

    /// The page as SVG for export: the CLI's own SVG bytes (the edited marker included).
    pub fn svg(&mut self, page: u32, highlighted: bool) -> Res<String> {
        Ok(mushaf_draw::to_svg(
            &self.s()?.render(page, highlighted).map_err(err)?,
        ))
    }
}

/// The JavaScript face of `Api`.
#[wasm_bindgen]
pub struct Engine(Api);

fn js(e: String) -> JsError {
    JsError::new(&e)
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Result<Engine, JsError> {
        Api::new(bytes).map(Engine).map_err(js)
    }
    pub fn styles(&self) -> Result<String, JsError> {
        self.0.styles().map_err(js)
    }
    pub fn select(&mut self, riwayah: &str, style: &str) -> Result<(), JsError> {
        self.0.select(riwayah, style).map_err(js)
    }
    pub fn pages(&mut self) -> Result<u32, JsError> {
        self.0.pages().map_err(js)
    }
    pub fn render(&mut self, page: u32, highlighted: bool) -> Result<String, JsError> {
        self.0.render(page, highlighted).map_err(js)
    }
    pub fn option_groups(&mut self) -> Result<String, JsError> {
        self.0.option_groups().map_err(js)
    }
    pub fn set_choices(&mut self, choices: &str) -> Result<(), JsError> {
        self.0.set_choices(choices).map_err(js)
    }
    pub fn margin_notes(&mut self, page: u32) -> Result<String, JsError> {
        self.0.margin_notes(page).map_err(js)
    }
    pub fn inspect(&mut self, page: u32) -> Result<String, JsError> {
        self.0.inspect(page).map_err(js)
    }
    pub fn provenance(&mut self) -> Result<String, JsError> {
        self.0.provenance().map_err(js)
    }
    pub fn edit(&mut self, op: &str) -> Result<(), JsError> {
        self.0.edit(op).map_err(js)
    }
    pub fn edit_state(&mut self) -> Result<String, JsError> {
        self.0.edit_state().map_err(js)
    }
    pub fn export_edits(&mut self) -> Result<String, JsError> {
        self.0.export_edits().map_err(js)
    }
    pub fn import_edits(&mut self, text: &str) -> Result<(), JsError> {
        self.0.import_edits(text).map_err(js)
    }
    pub fn lint(&mut self) -> Result<String, JsError> {
        self.0.lint().map_err(js)
    }
    pub fn svg(&mut self, page: u32, highlighted: bool) -> Result<String, JsError> {
        self.0.svg(page, highlighted).map_err(js)
    }
    pub fn view(&mut self, page: u32, highlighted: bool) -> Result<String, JsError> {
        self.0.view(page, highlighted).map_err(js)
    }
}
