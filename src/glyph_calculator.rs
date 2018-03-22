use super::*;
use std::fmt;
use std::mem;
use std::sync::{Mutex, MutexGuard};

/// Common glyph layout logic.
pub trait GlyphCruncher<'font> {
    /// Returns the pixel bounding box for the input section using a custom layout.
    /// The box is a conservative whole number pixel rectangle that can contain the section.
    ///
    /// If the section is empty or would result in no drawn glyphs will return `None`
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    fn pixel_bounds_custom_layout<'a, S, L>(
        &mut self,
        section: S,
        custom_layout: &L,
    ) -> Option<Rect<i32>>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, VariedSection<'a>>>;

    /// Returns the pixel bounding box for the input section. The box is a conservative
    /// whole number pixel rectangle that can contain the section.
    ///
    /// If the section is empty or would result in no drawn glyphs will return `None`
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    fn pixel_bounds<'a, S>(&mut self, section: S) -> Option<Rect<i32>>
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let layout = section.layout;
        self.pixel_bounds_custom_layout(section, &layout)
    }

    /// Returns an iterator over the `PositionedGlyph`s of the given section with a custom layout.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    fn glyphs_custom_layout<'a, 'b, S, L>(
        &'b mut self,
        section: S,
        custom_layout: &L,
    ) -> PositionedGlyphIter<'b, 'font>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, VariedSection<'a>>>;

    /// Returns an iterator over the `PositionedGlyph`s of the given section.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    fn glyphs<'a, 'b, S>(&'b mut self, section: S) -> PositionedGlyphIter<'b, 'font>
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let layout = section.layout;
        self.glyphs_custom_layout(section, &layout)
    }
}

/// Cut down version of a [`GlyphBrush`](struct.GlyphBrush.html) that can calculate pixel bounds,
/// but is unable to actually render anything.
///
/// Build using a [`GlyphCalculatorBuilder`](struct.GlyphCalculatorBuilder.html).
///
/// # Example
///
/// ```no_run
/// # extern crate gfx;
/// # extern crate gfx_window_glutin;
/// # extern crate glutin;
/// extern crate gfx_glyph;
/// use gfx_glyph::{GlyphCruncher, GlyphCalculatorBuilder, Section};
/// # fn main() {
///
/// let dejavu: &[u8] = include_bytes!("../examples/DejaVuSans.ttf");
/// let glyphs = GlyphCalculatorBuilder::using_font_bytes(dejavu).build();
///
/// let section = Section {
///     text: "Hello gfx_glyph",
///     ..Section::default()
/// };
///
/// // create the scope, equivalent to a lock on the cache when
/// // dropped will clean unused cached calculations like a draw call
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
/// Calls to [`GlyphCalculatorGuard::pixel_bounds`](#method.pixel_bounds),
/// [`GlyphCalculatorGuard::glyphs`](#method.glyphs) calculate the positioned glyphs for a
/// section. This is cached so future calls to any of the methods for the same section are much
/// cheaper.
///
/// Unlike a [`GlyphBrush`](struct.GlyphBrush.html) there is no concept of actually drawing
/// the section to imply a section is no longer used. Instead a `GlyphCalculatorGuard`
/// is created, that provides the calculation functionality. Dropping indicates the 'cache frame'
/// is over, similar to when a `GlyphBrush` is draws. Any cached sections from previous 'frames'
/// are invalidated.
pub struct GlyphCalculator<'font> {
    fonts: HashMap<FontId, rusttype::Font<'font>>,

    // cache of section-layout hash -> computed glyphs, this avoid repeated glyph computation
    // for identical layout/sections common to repeated frame rendering
    calculate_glyph_cache: Mutex<HashMap<u64, GlyphedSection<'font>>>,
}

impl<'font> fmt::Debug for GlyphCalculator<'font> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GlyphCalculator")
    }
}

impl<'font> GlyphCalculator<'font> {
    pub fn cache_scope<'a>(&'a self) -> GlyphCalculatorGuard<'a, 'font> {
        GlyphCalculatorGuard {
            fonts: &self.fonts,
            glyph_cache: self.calculate_glyph_cache.lock().unwrap(),
            cached: HashSet::new(),
        }
    }
}

/// [`GlyphCalculator`](struct.GlyphCalculator.html) scoped cache lock.
pub struct GlyphCalculatorGuard<'brush, 'font: 'brush> {
    fonts: &'brush HashMap<FontId, rusttype::Font<'font>>,
    glyph_cache: MutexGuard<'brush, HashMap<u64, GlyphedSection<'font>>>,
    cached: HashSet<u64>,
}

impl<'brush, 'font> GlyphCalculatorGuard<'brush, 'font> {
    /// Returns the calculate_glyph_cache key for this sections glyphs
    fn cache_glyphs<L>(&mut self, section: &VariedSection, layout: &L) -> u64
    where
        L: GlyphPositioner,
    {
        let section_hash = xxhash(&(section, layout));

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

impl<'brush, 'font> fmt::Debug for GlyphCalculatorGuard<'brush, 'font> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GlyphCalculatorGuard")
    }
}

impl<'brush, 'font> GlyphCruncher<'font> for GlyphCalculatorGuard<'brush, 'font> {
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
            Some(Rect {
                min: Point { x: x.0, y: y.0 },
                max: Point { x: x.1, y: y.1 },
            })
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

impl<'a, 'b> Drop for GlyphCalculatorGuard<'a, 'b> {
    fn drop(&mut self) {
        let mut cached = HashSet::new();
        mem::swap(&mut cached, &mut self.cached);
        self.glyph_cache.retain(|key, _| cached.contains(key));
    }
}

/// Builder for a [`GlyphCalculator`](struct.GlyphCalculator.html).
///
/// # Example
///
/// ```no_run
/// # extern crate gfx;
/// # extern crate gfx_window_glutin;
/// # extern crate glutin;
/// extern crate gfx_glyph;
/// use gfx_glyph::GlyphCalculatorBuilder;
/// # fn main() {
///
/// let dejavu: &[u8] = include_bytes!("../examples/DejaVuSans.ttf");
/// let mut glyphs = GlyphCalculatorBuilder::using_font_bytes(dejavu).build();
/// # let _ = glyphs;
/// # }
/// ```
pub struct GlyphCalculatorBuilder<'a> {
    font_data: Vec<Font<'a>>,
}

impl<'a> GlyphCalculatorBuilder<'a> {
    /// Specifies the default font data used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font_bytes<B: Into<SharedBytes<'a>>>(font_0_data: B) -> Self {
        Self::using_font(Font::from_bytes(font_0_data).unwrap())
    }

    pub fn using_fonts_bytes<B, V>(font_data: V) -> Self
    where
        B: Into<SharedBytes<'a>>,
        V: Into<Vec<B>>,
    {
        Self::using_fonts(
            font_data
                .into()
                .into_iter()
                .map(|data| Font::from_bytes(data).unwrap())
                .collect::<Vec<_>>(),
        )
    }

    /// Specifies the default font used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font(font_0_data: Font<'a>) -> Self {
        Self {
            font_data: vec![font_0_data],
        }
    }

    pub fn using_fonts<V: Into<Vec<Font<'a>>>>(fonts: V) -> Self {
        Self {
            font_data: fonts.into(),
        }
    }

    /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
    /// [`using_font_bytes`](#method.using_font_bytes).
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font_bytes<B: Into<SharedBytes<'a>>>(&mut self, font_data: B) -> FontId {
        self.font_data
            .push(Font::from_bytes(font_data.into()).unwrap());
        FontId(self.font_data.len() - 1)
    }

    /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
    /// [`using_font_bytes`](#method.using_font_bytes).
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font(&mut self, font_data: Font<'a>) -> FontId {
        self.font_data.push(font_data);
        FontId(self.font_data.len() - 1)
    }

    /// Builds a `GlyphCalculator`
    pub fn build(self) -> GlyphCalculator<'a> {
        let fonts = self.font_data
            .into_iter()
            .enumerate()
            .map(|(idx, data)| (FontId(idx), data))
            .collect();

        GlyphCalculator {
            fonts,
            calculate_glyph_cache: Mutex::new(HashMap::new()),
        }
    }
}
