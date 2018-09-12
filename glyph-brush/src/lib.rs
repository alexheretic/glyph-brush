//! ```
//! extern crate glyph_brush;
//!
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
//! # let screen_dimensions = (1024, 768);
//! # let update_texture = |_, _| {};
//! # let into_vertex = |_| ();
//! match glyph_brush.process_queued(
//!     screen_dimensions,
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

#[macro_use]
extern crate log;
extern crate glyph_brush_layout;
extern crate ordered_float;
extern crate rustc_hash;
extern crate rusttype as full_rusttype;
extern crate seahash;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

mod glyph_brush;
mod glyph_calculator;
mod owned_section;
mod section;

pub use glyph_brush::*;
pub use glyph_brush_layout::*;
pub use glyph_calculator::*;
pub use owned_section::*;
pub use section::*;

use glyph_brush_layout::rusttype::*;
use std::hash::BuildHasherDefault;

/// A "practically collision free" `Section` hasher
pub type DefaultSectionHasher = BuildHasherDefault<seahash::SeaHasher>;
