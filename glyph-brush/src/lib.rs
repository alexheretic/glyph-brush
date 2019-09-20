//! ```
//! use glyph_brush::{BrushAction, BrushError, GlyphBrushBuilder, Section};
//!
//! # fn main() -> Result<(), glyph_brush::BrushError> {
//! let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
//! let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build();
//! # let some_other_section = Section { text: "another", ..Section::default() };
//!
//! glyph_brush.queue(Section {
//!     text: "Hello glyph_brush",
//!     ..Section::default()
//! });
//! glyph_brush.queue(some_other_section);
//!
//! # fn update_texture(_: glyph_brush::rusttype::Rect<u32>, _: &[u8]) {}
//! # let into_vertex = |_| ();
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
mod glyph_brush;

pub use crate::glyph_brush::*;
pub use glyph_brush_next::{
    rusttype, BrushAction, BrushError, BuiltInLineBreaker, Color, DefaultSectionHasher, FontId,
    FontMap, GlyphBrush, GlyphBrushBuilder as GlyphBrushBuilderNext, GlyphCalculator,
    GlyphCalculatorBuilder, GlyphCalculatorGuard, GlyphChange, GlyphCruncher, GlyphPositioner,
    GlyphVertex, HorizontalAlign, Layout, LineBreak, LineBreaker, OwnedSectionText,
    OwnedVariedSection, PositionedGlyphIter, Section, SectionGeometry, SectionText, VariedSection,
    VerticalAlign,
};
