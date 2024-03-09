//! Text layout for [ab_glyph](https://github.com/alexheretic/ab-glyph).
//!
//! # Example
//!
//! ```
//! use glyph_brush_layout::{ab_glyph::*, *};
//! # fn main() -> Result<(), InvalidFont> {
//!
//! let dejavu = FontRef::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?;
//! let garamond = FontRef::try_from_slice(include_bytes!("../../fonts/GaramondNo8-Reg.ttf"))?;
//!
//! // Simple font mapping: FontId(0) -> deja vu sans, FontId(1) -> garamond
//! let fonts = &[dejavu, garamond];
//!
//! // Layout "hello glyph_brush_layout" on an unbounded line with the second
//! // word suitably bigger, greener and serif-ier.
//! let glyphs = Layout::default().calculate_glyphs(
//!     fonts,
//!     &SectionGeometry {
//!         screen_position: (150.0, 50.0),
//!         ..SectionGeometry::default()
//!     },
//!     &[
//!         SectionText {
//!             text: "hello ",
//!             scale: PxScale::from(20.0),
//!             font_id: FontId(0),
//!         },
//!         SectionText {
//!             text: "glyph_brush_layout",
//!             scale: PxScale::from(25.0),
//!             font_id: FontId(1),
//!         },
//!     ],
//! );
//!
//! assert_eq!(glyphs.len(), 24);
//!
//! let SectionGlyph {
//!     glyph,
//!     font_id,
//!     section_index,
//!     byte_index,
//! } = &glyphs[4];
//! assert_eq!(glyph.id, fonts[0].glyph_id('o'));
//! assert_eq!(*font_id, FontId(0));
//! assert_eq!(*section_index, 0);
//! assert_eq!(*byte_index, 4);
//!
//! let SectionGlyph {
//!     glyph,
//!     font_id,
//!     section_index,
//!     byte_index,
//! } = &glyphs[14];
//! assert_eq!(glyph.id, fonts[1].glyph_id('u'));
//! assert_eq!(*font_id, FontId(1));
//! assert_eq!(*section_index, 1);
//! assert_eq!(*byte_index, 8);
//!
//! # Ok(())
//! # }
//! ```
mod builtin;
mod characters;
mod font;
mod linebreak;
mod lines;
mod section;
mod words;

/// Re-exported ab_glyph types.
pub mod ab_glyph {
    pub use ab_glyph::*;
}
pub use self::{builtin::*, font::*, linebreak::*, section::*};

use ::ab_glyph::*;
use std::hash::Hash;

/// Logic to calculate glyph positioning using [`Font`](struct.Font.html),
/// [`SectionGeometry`](struct.SectionGeometry.html) and
/// [`SectionText`](struct.SectionText.html).
pub trait GlyphPositioner: Hash {
    /// Calculate a sequence of positioned glyphs to render. Custom implementations should
    /// return the same result when called with the same arguments to allow layout caching.
    fn calculate_glyphs<F, S>(
        &self,
        fonts: &[F],
        geometry: &SectionGeometry,
        sections: &[S],
    ) -> Vec<SectionGlyph>
    where
        F: Font,
        S: ToSectionText;

    /// Return a screen rectangle according to the requested render position and bounds
    /// appropriate for the glyph layout.
    fn bounds_rect(&self, geometry: &SectionGeometry) -> Rect;

    /// Recalculate a glyph sequence after a change.
    ///
    /// The default implementation simply calls `calculate_glyphs` so must be implemented
    /// to provide benefits as such benefits are specific to the internal layout logic.
    fn recalculate_glyphs<F, S, P>(
        &self,
        previous: P,
        change: GlyphChange,
        fonts: &[F],
        geometry: &SectionGeometry,
        sections: &[S],
    ) -> Vec<SectionGlyph>
    where
        F: Font,
        S: ToSectionText,
        P: IntoIterator<Item = SectionGlyph>,
    {
        let _ = (previous, change);
        self.calculate_glyphs(fonts, geometry, sections)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum GlyphChange {
    /// Only the geometry has changed, contains the old geometry
    Geometry(SectionGeometry),
    Unknown,
}
