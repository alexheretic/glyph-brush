extern crate glyph_brush_layout;
extern crate ordered_float;
extern crate rustc_hash;
extern crate rusttype as full_rusttype;
extern crate seahash;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

mod glyph_calculator;
mod owned_section;
mod section;

pub use glyph_brush_layout::rusttype;
pub use glyph_brush_layout::*;
pub use glyph_calculator::*;
pub use owned_section::*;
pub use section::*;

use glyph_brush_layout::rusttype::*;
use std::hash::BuildHasherDefault;

/// A "practically collision free" `Section` hasher
type DefaultSectionHasher = BuildHasherDefault<seahash::SeaHasher>;
