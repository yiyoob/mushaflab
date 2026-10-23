//! Font loading, KFGQPC -> font encoding conversion, and shaping (T3.1; D2, D3, D14).

mod encoding;
mod font;
mod integrity;

pub use encoding::{Converted, Digest, EncodingTable, Span, Unmapped};
pub use font::{CacheStats, FeatureRange, FontData, ShapedGlyph, Shaper};
pub use integrity::{Violation, check_word};
