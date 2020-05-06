//! ```
//! use glyph_brush::{ab_glyph::FontArc, BrushAction, BrushError, GlyphBrushBuilder, Section};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let dejavu = FontArc::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?;
//! let mut glyph_brush = GlyphBrushBuilder::using_font(dejavu).build();
//! # let some_other_section = Section { text: "another", ..Section::default() };
//!
//! glyph_brush.queue(Section {
//!     text: "Hello glyph_brush",
//!     ..Section::default()
//! });
//! glyph_brush.queue(some_other_section);
//!
//! # fn update_texture(_: glyph_brush::Rectangle<u32>, _: &[u8]) {}
//! # fn into_vertex(v: glyph_brush::GlyphVertex) { () }
//! match glyph_brush.process_queued(
//!     |rect, tex_data| update_texture(rect, tex_data),
//!     |vertex_data| into_vertex(vertex_data),
//! ) {
//!     Ok(BrushAction::Draw(vertices)) => {
//!         // Draw new vertices.
//!     }
//!     Ok(BrushAction::ReDraw) => {
//!         // Re-draw last frame's vertices unmodified.
//!     }
//!     Err(BrushError::TextureTooSmall { suggested }) => {
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

    let section_a = Section {
        text: "Hovered Tile: Some((0, 0))",
        screen_position: (5.0, 60.0),
        scale: PxScale::from(20.0),
        color: [1.0, 1.0, 1.0, 1.0],
        ..<_>::default()
    };
    let section_b = Section {
        text: "Hovered Tile: Some((1, 0))",
        screen_position: (5.0, 60.0),
        scale: PxScale::from(20.0),
        color: [1.0, 1.0, 1.0, 1.0],
        ..<_>::default()
    };
    let hash = |s: &Section| {
        let s: VariedSection = s.into();
        let mut hasher = DefaultSectionHasher::default().build_hasher();
        s.hash(&mut hasher);
        hasher.finish()
    };
    assert_ne!(hash(&section_a), hash(&section_b));
}
