//! Font loading, KFGQPC -> font encoding conversion, and shaping (T3.1; D2, D3, D14).

mod encoding;
mod font;

pub use encoding::{Converted, Digest, EncodingTable, Span, Unmapped};
pub use font::{FeatureRange, FontData, ShapedGlyph, Shaper};
