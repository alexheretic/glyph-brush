//! Text layout for [rusttype](https://gitlab.redox-os.org/redox-os/rusttype).
//!
//! # Example
//!
//! ```
//! use glyph_brush_layout::{ab_glyph::*, *};
//! # fn main() -> Result<(), InvalidFont> {
//!
//! let dejavu = FontRef::try_from_slice(&include_bytes!("../../fonts/DejaVuSans.ttf")[..])?;
//! let garamond = FontRef::try_from_slice(&include_bytes!("../../fonts/GaramondNo8-Reg.ttf")[..])?;
//!
//! // Simple vec font mapping: FontId(0) -> deja vu sans, FontId(1) -> garamond
//! let fonts = &[dejavu, garamond];
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
//!             scale: PxScale::from(20.0),
//!             ..SectionText::default()
//!         },
//!         SectionText {
//!             text: "glyph_brush_layout",
//!             scale: PxScale::from(25.0),
//!             font_id: FontId(1),
//!             color: [0.0, 1.0, 0.0, 1.0],
//!         },
//!     ],
//! );
//!
//! assert_eq!(glyphs.len(), 24);
//!
//! let (o_glyph, glyph_4_color, glyph_4_font) = &glyphs[4];
//! assert_eq!(o_glyph.id, fonts[0].glyph_id('o'));
//! assert_eq!(*glyph_4_color, [0.0, 0.0, 0.0, 1.0]);
//! assert_eq!(*glyph_4_font, FontId(0));
//!
//! let (u_glyph, glyph_14_color, glyph_14_font) = &glyphs[14];
//! assert_eq!(u_glyph.id, fonts[1].glyph_id('u'));
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
use ::ab_glyph::*;

/// Re-exported rusttype types.
pub mod ab_glyph {
    pub use ab_glyph::*;
}

use std::hash::Hash;

/// Logic to calculate glyph positioning using [`Font`](struct.Font.html),
/// [`SectionGeometry`](struct.SectionGeometry.html) and
/// [`SectionText`](struct.SectionText.html).
pub trait GlyphPositioner: Hash {
    /// Calculate a sequence of positioned glyphs to render. Custom implementations should
    /// return the same result when called with the same arguments to allow layout caching.
    fn calculate_glyphs<F: Font, FM: FontMap<F>>(
        &self,
        fonts: &FM,
        geometry: &SectionGeometry,
        sections: &[SectionText<'_>],
    ) -> Vec<(Glyph, Color, FontId)>;

    /// Return a screen rectangle according to the requested render position and bounds
    /// appropriate for the glyph layout.
    fn bounds_rect(&self, geometry: &SectionGeometry) -> Rect;

    /// Recalculate a glyph sequence after a change.
    ///
    /// The default implementation simply calls `calculate_glyphs` so must be implemented
    /// to provide benefits as such benefits are spefic to the internal layout logic.
    fn recalculate_glyphs<F: Font, FM: FontMap<F>>(
        &self,
        previous: Cow<'_, Vec<(Glyph, Color, FontId)>>,
        change: GlyphChange,
        fonts: &FM,
        geometry: &SectionGeometry,
        sections: &[SectionText<'_>],
    ) -> Vec<(Glyph, Color, FontId)> {
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
