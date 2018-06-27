mod builtin;
pub(crate) mod characters;
pub(crate) mod lines;
pub(crate) mod words;

pub use self::builtin::*;

use super::*;
use characters::Characters;
use std::hash::BuildHasher;
use std::slice::Iter;

/// Logic to calculate glyph positioning based on [`Font`](struct.Font.html) and
/// [`VariedSection`](struct.VariedSection.html)
pub trait GlyphPositioner: Hash {
    /// Calculate a sequence of positioned glyphs to render. Custom implementations should always
    /// return the same result when called with the same arguments. If not consider disabling
    /// [`cache_glyph_positioning`](struct.GlyphBrushBuilder.html#method.cache_glyph_positioning).
    fn calculate_glyphs<'font, H: BuildHasher>(
        &self,
        &HashMap<FontId, Font<'font>, H>,
        section: &VariedSection,
    ) -> Vec<(PositionedGlyph<'font>, Color, FontId)>;

    /// Return a rectangle according to the requested render position and bounds appropriate
    /// for the glyph layout.
    fn bounds_rect(&self, section: &VariedSection) -> Rect<f32>;
}
