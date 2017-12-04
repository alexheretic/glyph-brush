use super::*;
use std::fmt;
use std::mem;
use std::sync::{Mutex, MutexGuard};

/// Cut down version of a [`GlyphBrush`](struct.GlyphBrush.html) that can calculate pixel bounds,
/// but is unable to actually render anything.
///
/// Build using a [`HeadlessGlyphBrushBuilder`](struct.HeadlessGlyphBrushBuilder.html).
///
/// # Example
///
/// ```no_run
/// # extern crate gfx;
/// # extern crate gfx_window_glutin;
/// # extern crate glutin;
/// extern crate gfx_glyph;
/// use gfx_glyph::{GlyphCalculator, HeadlessGlyphBrushBuilder, Section};
/// # fn main() {
///
/// let arial: &[u8] = include_bytes!("../examples/Arial Unicode.ttf");
/// let glyphs = HeadlessGlyphBrushBuilder::using_font_bytes(arial).build();
///
/// let section = Section {
///     text: "Hello gfx_glyph",
///     ..Section::default()
/// };
///
/// let mut scope = glyphs.cache_scope();
///
/// let bounds = scope.pixel_bounds(section);
/// # let _ = bounds;
/// # let _ = glyphs;
/// # }
/// ```
///
/// # Caching behaviour
///
/// Calls to [`HeadlessGlyphCacheGuard::pixel_bounds`](#method.pixel_bounds),
/// [`HeadlessGlyphCacheGuard::glyphs`](#method.glyphs) calculate the positioned glyphs for a
/// section. This is cached so future calls to any of the methods for the same section are much
/// cheaper.
///
/// Unlike a [`GlyphBrush`](struct.GlyphBrush.html) there is no concept of actually drawing
/// the section to imply a section is no longer used. Instead a `HeadlessGlyphCacheGuard`
/// is created, that provides the calculation functionality. Dropping indicates the 'cache frame'
/// is over, similar to when a `GlyphBrush` is draws. Any cached sections from previous 'frames'
/// are invalidated.
pub struct HeadlessGlyphBrush<'font> {
    fonts: HashMap<FontId, rusttype::Font<'font>>,

    // cache of section-layout hash -> computed glyphs, this avoid repeated glyph computation
    // for identical layout/sections common to repeated frame rendering
    calculate_glyph_cache: Mutex<HashMap<u64, GlyphedSection<'font>>>,
}

impl<'font> fmt::Debug for HeadlessGlyphBrush<'font> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "HeadlessGlyphBrush")
    }
}

impl<'font> HeadlessGlyphBrush<'font> {
    pub fn cache_scope<'a>(&'a self) -> HeadlessGlyphCacheGuard<'a, 'font> {
        HeadlessGlyphCacheGuard {
            fonts: &self.fonts,
            glyph_cache: self.calculate_glyph_cache.lock().unwrap(),
            cached: HashSet::new(),
        }
    }
}

pub struct HeadlessGlyphCacheGuard<'brush, 'font: 'brush> {
    fonts: &'brush HashMap<FontId, rusttype::Font<'font>>,
    glyph_cache: MutexGuard<'brush, HashMap<u64, GlyphedSection<'font>>>,
    cached: HashSet<u64>,
}

impl<'brush, 'font> HeadlessGlyphCacheGuard<'brush, 'font> {
    /// Returns the calculate_glyph_cache key for this sections glyphs
    fn cache_glyphs<L>(&mut self, section: &VariedSection, layout: &L) -> u64
    where
        L: GlyphPositioner,
    {
        let section_hash = hash(&(section, layout));

        if let Entry::Vacant(entry) = self.glyph_cache.entry(section_hash) {
            entry.insert(GlyphedSection {
                bounds: layout.bounds_rect(section),
                glyphs: layout.calculate_glyphs(self.fonts, section),
                z: section.z,
            });
        }

        section_hash
    }
}

impl<'brush, 'font> fmt::Debug for HeadlessGlyphCacheGuard<'brush, 'font> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "HeadlessGlyphCacheGuard")
    }
}

impl<'brush, 'font> GlyphCalculator<'font> for HeadlessGlyphCacheGuard<'brush, 'font> {
    fn pixel_bounds_custom_layout<'a, S, L>(
        &mut self,
        section: S,
        custom_layout: &L,
    ) -> Option<Rect<i32>>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let mut x = (0, 0);
        let mut y = (0, 0);
        let mut no_match = true;

        let section_hash = self.cache_glyphs(section.borrow(), custom_layout);
        self.cached.insert(section_hash);
        for g in self.glyph_cache[&section_hash]
            .glyphs
            .iter()
            .flat_map(|&GlyphedSectionText(ref g, ..)| g.iter())
        {
            if let Some(Rect { min, max }) = g.pixel_bounding_box() {
                if no_match || min.x < x.0 {
                    x.0 = min.x;
                }
                if no_match || min.y < y.0 {
                    y.0 = min.y;
                }
                if no_match || max.x > x.1 {
                    x.1 = max.x;
                }
                if no_match || max.y > y.1 {
                    y.1 = max.y;
                }

                no_match = false;
            }
        }

        if no_match {
            None
        }
        else {
            Some(Rect { min: Point { x: x.0, y: y.0 }, max: Point { x: x.1, y: y.1 } })
        }
    }

    fn glyphs_custom_layout<'a, 'b, S, L>(
        &'b mut self,
        section: S,
        custom_layout: &L,
    ) -> PositionedGlyphIter<'b, 'font>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let section_hash = self.cache_glyphs(section.borrow(), custom_layout);
        self.cached.insert(section_hash);
        self.glyph_cache[&section_hash]
            .glyphs
            .iter()
            .flat_map(|&GlyphedSectionText(ref g, ..)| g.iter())
    }
}

impl<'a, 'b> Drop for HeadlessGlyphCacheGuard<'a, 'b> {
    fn drop(&mut self) {
        let mut cached = HashSet::new();
        mem::swap(&mut cached, &mut self.cached);
        self.glyph_cache.retain(|key, _| cached.contains(key));
    }
}

/// Builder for a [`HeadlessGlyphBrush`](struct.HeadlessGlyphBrush.html).
///
/// # Example
///
/// ```no_run
/// # extern crate gfx;
/// # extern crate gfx_window_glutin;
/// # extern crate glutin;
/// extern crate gfx_glyph;
/// use gfx_glyph::HeadlessGlyphBrushBuilder;
/// # fn main() {
///
/// let arial: &[u8] = include_bytes!("../examples/Arial Unicode.ttf");
/// let mut glyphs = HeadlessGlyphBrushBuilder::using_font_bytes(arial).build();
/// # let _ = glyphs;
/// # }
/// ```
pub struct HeadlessGlyphBrushBuilder<'a> {
    font_data: Vec<Font<'a>>,
}

impl<'a> HeadlessGlyphBrushBuilder<'a> {
    /// Specifies the default font data used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font_bytes<B: Into<SharedBytes<'a>>>(font_0_data: B) -> Self {
        Self::using_font(font(font_0_data).unwrap())
    }


    pub fn using_fonts_bytes<B, V>(font_data: V) -> Self
    where
        B: Into<SharedBytes<'a>>,
        V: Into<Vec<B>>,
    {
        Self::using_fonts(
            font_data.into().into_iter().map(|data| font(data).unwrap()).collect::<Vec<_>>(),
        )
    }

    /// Specifies the default font used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font(font_0_data: Font<'a>) -> Self {
        Self { font_data: vec![font_0_data] }
    }

    pub fn using_fonts<V: Into<Vec<Font<'a>>>>(fonts: V) -> Self {
        Self { font_data: fonts.into() }
    }

    /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
    /// [`using_font_bytes`](#method.using_font_bytes).
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font_bytes<B: Into<SharedBytes<'a>>>(&mut self, font_data: B) -> FontId {
        self.font_data.push(font(font_data.into()).unwrap());
        FontId(self.font_data.len() - 1)
    }

    /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
    /// [`using_font_bytes`](#method.using_font_bytes).
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font(&mut self, font_data: Font<'a>) -> FontId {
        self.font_data.push(font_data);
        FontId(self.font_data.len() - 1)
    }

    /// Builds a `HeadlessGlyphBrush`
    pub fn build(self) -> HeadlessGlyphBrush<'a> {
        let fonts = self.font_data
            .into_iter()
            .enumerate()
            .map(|(idx, data)| (FontId(idx), data))
            .collect();

        HeadlessGlyphBrush { fonts, calculate_glyph_cache: Mutex::new(HashMap::new()) }
    }
}
