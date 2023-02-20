//! ```
//! use glyph_brush::{
//!     ab_glyph::FontArc, BrushAction, BrushError, GlyphBrushBuilder, Section, Text,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let dejavu = FontArc::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?;
//! let mut glyph_brush = GlyphBrushBuilder::using_font(dejavu).build();
//! # let some_other_section = Section::default();
//!
//! glyph_brush.queue(Section::default().add_text(Text::new("Hello glyph_brush")));
//! glyph_brush.queue(some_other_section);
//!
//! # fn update_texture(_: glyph_brush::Rectangle<u32>, _: &[K::Element]) {}
//! # fn into_vertex(v: glyph_brush::GlyphVertex) { () }
//! match glyph_brush.process_queued(
//!     &mut (),
//!     |rect, outline_data, _| update_texture(rect, outline_data),
//!     |rect, emoji_data, _| update_texture(rect, emoji_data),
//!     |vertex_data| into_vertex(vertex_data),
//! ) {
//!     Ok(BrushAction::Draw(outline_vertices, emoji_vertices)) => {
//!         // Draw new vertices.
//!     }
//!     Ok(BrushAction::ReDraw) => {
//!         // Re-draw last frame's vertices unmodified.
//!     }
//!     Err(BrushError::TextureTooSmall { types, suggested }) => {
//!         // Enlarge texture + glyph_brush texture cache and retry.
//!     }
//! }
//! # Ok(())
//! # }
//! ```
mod extra;
mod glyph_brush;
mod glyph_calculator;
mod owned_section;
mod section;

pub mod legacy;

pub use crate::{extra::*, glyph_brush::*, glyph_calculator::*, owned_section::*, section::*};
pub use glyph_brush_draw_cache::Rectangle;
pub use glyph_brush_layout::*;

use glyph_brush_layout::ab_glyph::*;

/// A "practically collision free" `Section` hasher
#[cfg(not(target_arch = "wasm32"))]
pub type DefaultSectionHasher = twox_hash::RandomXxHashBuilder;
// Work around for rand issues in wasm #61
#[cfg(target_arch = "wasm32")]
pub type DefaultSectionHasher = std::hash::BuildHasherDefault<twox_hash::XxHash>;

#[test]
fn default_section_hasher() {
    use std::hash::{BuildHasher, Hash, Hasher};

    let section_a = Section::default().add_text(Text::new("Hovered Tile: Some((0, 0))"));
    let section_b = Section::default().add_text(Text::new("Hovered Tile: Some((1, 0))"));
    let hash = |s: &Section| {
        let mut hasher = DefaultSectionHasher::default().build_hasher();
        s.hash(&mut hasher);
        hasher.finish()
    };
    assert_ne!(hash(&section_a), hash(&section_b));
}
