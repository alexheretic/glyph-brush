mod builtin;
pub(crate) mod characters;
pub(crate) mod linebreak;
pub(crate) mod lines;
pub(crate) mod words;

pub use self::builtin::*;

use super::*;
use characters::Characters;
use std::ops;
use std::slice::Iter;
use vec_map::VecMap;

/// Logic to calculate glyph positioning based on [`Font`](struct.Font.html) and
/// [`VariedSection`](struct.VariedSection.html)
pub trait GlyphPositioner: Hash {
    /// Calculate a sequence of positioned glyphs to render. Custom implementations should always
    /// return the same result when called with the same arguments. If not consider disabling
    /// [`cache_glyph_positioning`](struct.GlyphBrushBuilder.html#method.cache_glyph_positioning).
    fn calculate_glyphs<'font>(
        &self,
        &FontMap<'font>,
        section: &VariedSection,
    ) -> Vec<(PositionedGlyph<'font>, Color, FontId)>;

    /// Return a screen rectangle according to the requested render position and bounds
    /// appropriate for the glyph layout.
    fn bounds_rect(&self, section: &VariedSection) -> Rect<f32>;
}

/// Map of [`FontId`](struct.FontId.html) → [`Font`](struct.Font.html).
pub type FontMap<'font> = VecMap<Font<'font>>;

impl<'font> ops::Index<FontId> for FontMap<'font> {
    type Output = Font<'font>;
    #[inline]
    fn index(&self, i: FontId) -> &Self::Output {
        &self[i.0]
    }
}
impl<'a, 'font> ops::Index<&'a FontId> for FontMap<'font> {
    type Output = Font<'font>;
    #[inline]
    fn index(&self, i: &'a FontId) -> &Self::Output {
        &self[*i]
    }
}
