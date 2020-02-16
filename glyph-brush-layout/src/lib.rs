//! Text layout for [rusttype](https://gitlab.redox-os.org/redox-os/rusttype).
//!
//! # Example
//!
//! ```
//! use glyph_brush_layout::{rusttype::*, *};
//! # fn main() -> Result<(), rusttype::Error> {
//!
//! let dejavu = Font::from_bytes(&include_bytes!("../../fonts/DejaVuSans.ttf")[..])?;
//! let garamond = Font::from_bytes(&include_bytes!("../../fonts/GaramondNo8-Reg.ttf")[..])?;
//!
//! // Simple vec font mapping: FontId(0) -> deja vu sans, FontId(1) -> garamond
//! let fonts = vec![dejavu, garamond];
//!
//! // Layout "hello glyph_brush_layout" on an unbounded line with the second
//! // word suitably bigger, greener and serif-ier.
//! let glyphs = Layout::default().calculate_glyphs(
//!     &fonts,
//!     &SectionGeometry {
//!         screen_position: (150.0, 50.0),
//!         ..SectionGeometry::default()
//!     },
//!     &[
//!         SectionText {
//!             text: "hello ",
//!             scale: Scale::uniform(20.0),
//!             ..SectionText::default()
//!         },
//!         SectionText {
//!             text: "glyph_brush_layout",
//!             scale: Scale::uniform(25.0),
//!             font_id: FontId(1),
//!             color: [0.0, 1.0, 0.0, 1.0],
//!         },
//!     ],
//! );
//!
//! assert_eq!(glyphs.len(), 23);
//!
//! let (o_glyph, glyph_4_color, glyph_4_font) = &glyphs[4];
//! assert_eq!(o_glyph.id(), fonts[0].glyph('o').id());
//! assert_eq!(*glyph_4_color, [0.0, 0.0, 0.0, 1.0]);
//! assert_eq!(*glyph_4_font, FontId(0));
//!
//! let (s_glyph, glyph_14_color, glyph_14_font) = &glyphs[14];
//! assert_eq!(s_glyph.id(), fonts[1].glyph('s').id());
//! assert_eq!(*glyph_14_color, [0.0, 1.0, 0.0, 1.0]);
//! assert_eq!(*glyph_14_font, FontId(1));
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

pub use self::{builtin::*, font::*, linebreak::*, section::*};
use std::borrow::Cow;

/// Re-exported rusttype types.
pub mod rusttype {
    pub use full_rusttype::{
        point, Error, Font, Glyph, GlyphId, HMetrics, Point, PositionedGlyph, Rect, Scale,
        ScaledGlyph, SharedBytes, VMetrics,
    };
}

use crate::rusttype::*;
use std::hash::Hash;

/// Logic to calculate glyph positioning using [`Font`](struct.Font.html),
/// [`SectionGeometry`](struct.SectionGeometry.html) and
/// [`SectionText`](struct.SectionText.html).
pub trait GlyphPositioner: Hash {
    /// Calculate a sequence of positioned glyphs to render. Custom implementations should
    /// return the same result when called with the same arguments to allow layout caching.
    fn calculate_glyphs<'font, F: FontMap<'font>>(
        &self,
        fonts: &F,
        geometry: &SectionGeometry,
        sections: &[SectionText<'_>],
    ) -> Vec<(PositionedGlyph<'font>, Color, FontId)>;

    /// Return a screen rectangle according to the requested render position and bounds
    /// appropriate for the glyph layout.
    fn bounds_rect(&self, geometry: &SectionGeometry) -> Rect<f32>;

    /// Recalculate a glyph sequence after a change.
    ///
    /// The default implementation simply calls `calculate_glyphs` so must be implemented
    /// to provide benefits as such benefits are spefic to the internal layout logic.
    fn recalculate_glyphs<'font, F>(
        &self,
        previous: Cow<'_, Vec<(PositionedGlyph<'font>, Color, FontId)>>,
        change: GlyphChange,
        fonts: &F,
        geometry: &SectionGeometry,
        sections: &[SectionText<'_>],
    ) -> Vec<(PositionedGlyph<'font>, Color, FontId)>
    where
        F: FontMap<'font>,
    {
        let _ = (previous, change);
        self.calculate_glyphs(fonts, geometry, sections)
    }
}

// #[non_exhaustive] TODO use when stable
#[derive(Debug)]
pub enum GlyphChange {
    /// Only the geometry has changed, contains the old geometry
    Geometry(SectionGeometry),
    /// Only the colors have changed (including alpha)
    Color,
    /// Only the alpha has changed
    Alpha,
    Unknown,
    #[doc(hidden)]
    __Nonexhaustive,
}
