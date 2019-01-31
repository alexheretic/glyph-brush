use super::*;
use full_rusttype::point;
use hashbrown::hash_map::Entry;
use std::{
    borrow::Cow,
    fmt,
    hash::{BuildHasher, Hash, Hasher},
    i32, mem, slice,
    sync::{Mutex, MutexGuard},
};

/// [`PositionedGlyph`](struct.PositionedGlyph.html) iterator.
pub type PositionedGlyphIter<'a, 'font> = std::iter::Map<
    slice::Iter<'a, (rusttype::PositionedGlyph<'font>, [f32; 4], FontId)>,
    fn(
        &'a (rusttype::PositionedGlyph<'font>, [f32; 4], FontId),
    ) -> &'a rusttype::PositionedGlyph<'font>,
>;

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

    /// Returns the available fonts.
    ///
    /// The `FontId` corresponds to the index of the font data.
    fn fonts(&self) -> &[Font<'font>];
}

/// Cut down version of a [`GlyphBrush`](struct.GlyphBrush.html) that can calculate pixel bounds,
/// but is unable to actually render anything.
///
/// Build using a [`GlyphCalculatorBuilder`](struct.GlyphCalculatorBuilder.html).
///
/// # Example
///
/// ```
/// use glyph_brush::{GlyphCalculatorBuilder, GlyphCruncher, Section};
/// # fn main() {
///
/// let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
/// let glyphs = GlyphCalculatorBuilder::using_font_bytes(dejavu).build();
///
/// let section = Section {
///     text: "Hello glyph_brush",
///     ..Section::default()
/// };
///
/// // create the scope, equivalent to a lock on the cache when
/// // dropped will clean unused cached calculations like a draw call
/// let mut scope = glyphs.cache_scope();
///
/// let bounds = scope.pixel_bounds(section);
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
/// the section to imply when a section is used / no longer used. Instead a `GlyphCalculatorGuard`
/// is created, that provides the calculation functionality. Dropping indicates the 'cache frame'
/// is over, similar to when a `GlyphBrush` draws. Section calculations are cached for the next
/// 'cache frame', if not used then they will be dropped.
pub struct GlyphCalculator<'font, H = DefaultSectionHasher> {
    fonts: Vec<Font<'font>>,

    // cache of section-layout hash -> computed glyphs, this avoid repeated glyph computation
    // for identical layout/sections common to repeated frame rendering
    calculate_glyph_cache: Mutex<hashbrown::HashMap<u64, GlyphedSection<'font>>>,

    section_hasher: H,
}

impl<H> fmt::Debug for GlyphCalculator<'_, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlyphCalculator")
    }
}

impl<'font, H: BuildHasher + Clone> GlyphCalculator<'font, H> {
    pub fn cache_scope<'a>(&'a self) -> GlyphCalculatorGuard<'a, 'font, H> {
        GlyphCalculatorGuard {
            fonts: &self.fonts,
            glyph_cache: self.calculate_glyph_cache.lock().unwrap(),
            cached: hashbrown::HashSet::default(),
            section_hasher: self.section_hasher.clone(),
        }
    }

    /// Returns the available fonts.
    ///
    /// The `FontId` corresponds to the index of the font data.
    pub fn fonts(&self) -> &[Font<'_>] {
        &self.fonts
    }
}

/// [`GlyphCalculator`](struct.GlyphCalculator.html) scoped cache lock.
pub struct GlyphCalculatorGuard<'brush, 'font: 'brush, H = DefaultSectionHasher> {
    fonts: &'brush Vec<Font<'font>>,
    glyph_cache: MutexGuard<'brush, hashbrown::HashMap<u64, GlyphedSection<'font>>>,
    cached: hashbrown::HashSet<u64>,
    section_hasher: H,
}

impl<H: BuildHasher> GlyphCalculatorGuard<'_, '_, H> {
    /// Returns the calculate_glyph_cache key for this sections glyphs
    fn cache_glyphs<L>(&mut self, section: &VariedSection<'_>, layout: &L) -> u64
    where
        L: GlyphPositioner,
    {
        let section_hash = {
            let mut hasher = self.section_hasher.build_hasher();
            section.hash(&mut hasher);
            layout.hash(&mut hasher);
            hasher.finish()
        };

        if let Entry::Vacant(entry) = self.glyph_cache.entry(section_hash) {
            let geometry = SectionGeometry::from(section);
            entry.insert(GlyphedSection {
                bounds: layout.bounds_rect(&geometry),
                glyphs: layout.calculate_glyphs(self.fonts, &geometry, &section.text),
                z: section.z,
                geometry,
            });
        }

        section_hash
    }
}

impl fmt::Debug for GlyphCalculatorGuard<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlyphCalculatorGuard")
    }
}

impl<'font, H: BuildHasher> GlyphCruncher<'font> for GlyphCalculatorGuard<'_, 'font, H> {
    fn pixel_bounds_custom_layout<'a, S, L>(
        &mut self,
        section: S,
        custom_layout: &L,
    ) -> Option<Rect<i32>>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section_hash = self.cache_glyphs(&section.into(), custom_layout);
        self.cached.insert(section_hash);
        self.glyph_cache[&section_hash].pixel_bounds()
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
        let section_hash = self.cache_glyphs(&section.into(), custom_layout);
        self.cached.insert(section_hash);
        self.glyph_cache[&section_hash].glyphs()
    }

    fn fonts(&self) -> &[Font<'font>] {
        &self.fonts
    }
}

impl<H> Drop for GlyphCalculatorGuard<'_, '_, H> {
    fn drop(&mut self) {
        let cached = mem::replace(&mut self.cached, hashbrown::HashSet::default());
        self.glyph_cache.retain(|key, _| cached.contains(key));
    }
}

/// Builder for a [`GlyphCalculator`](struct.GlyphCalculator.html).
///
/// # Example
///
/// ```no_run
/// use glyph_brush::GlyphCalculatorBuilder;
/// # fn main() {
///
/// let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
/// let mut glyphs = GlyphCalculatorBuilder::using_font_bytes(dejavu).build();
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct GlyphCalculatorBuilder<'a, H = DefaultSectionHasher> {
    font_data: Vec<Font<'a>>,
    section_hasher: H,
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
        Self::using_fonts(vec![font_0_data])
    }

    pub fn using_fonts<V: Into<Vec<Font<'a>>>>(fonts: V) -> Self {
        Self {
            font_data: fonts.into(),
            section_hasher: DefaultSectionHasher::default(),
        }
    }
}

impl<'a, H: BuildHasher> GlyphCalculatorBuilder<'a, H> {
    /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
    /// [`using_font_bytes`](#method.using_font_bytes).
    ///
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font_bytes<B: Into<SharedBytes<'a>>>(&mut self, font_data: B) -> FontId {
        self.font_data
            .push(Font::from_bytes(font_data.into()).unwrap());
        FontId(self.font_data.len() - 1)
    }

    /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
    /// [`using_font_bytes`](#method.using_font_bytes).
    ///
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font(&mut self, font_data: Font<'a>) -> FontId {
        self.font_data.push(font_data);
        FontId(self.font_data.len() - 1)
    }

    /// Sets the section hasher. `GlyphCalculator` cannot handle absolute section hash collisions
    /// so use a good hash algorithm.
    ///
    /// This hasher is used to distinguish sections, rather than for hashmap internal use.
    ///
    /// Defaults to [seahash](https://docs.rs/seahash).
    pub fn section_hasher<T: BuildHasher>(
        self,
        section_hasher: T,
    ) -> GlyphCalculatorBuilder<'a, T> {
        GlyphCalculatorBuilder {
            font_data: self.font_data,
            section_hasher,
        }
    }

    /// Builds a `GlyphCalculator`
    pub fn build(self) -> GlyphCalculator<'a, H> {
        GlyphCalculator {
            fonts: self.font_data,
            calculate_glyph_cache: Mutex::default(),
            section_hasher: self.section_hasher,
        }
    }
}

#[derive(Clone)]
pub(crate) struct GlyphedSection<'font> {
    pub bounds: Rect<f32>,
    pub glyphs: Vec<(PositionedGlyph<'font>, Color, FontId)>,
    pub z: f32,
    pub geometry: SectionGeometry,
}

impl<'a> PartialEq<GlyphedSection<'a>> for GlyphedSection<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.bounds == other.bounds
            && self.z == other.z
            && self.glyphs.len() == other.glyphs.len()
            && self
                .glyphs
                .iter()
                .zip(other.glyphs.iter())
                .all(|(l, r)| l.2 == r.2 && l.1 == r.1 && l.0.id() == r.0.id())
    }
}

impl<'font> GlyphedSection<'font> {
    pub(crate) fn pixel_bounds(&self) -> Option<Rect<i32>> {
        let Self {
            ref glyphs, bounds, ..
        } = *self;

        let to_i32 = |f: f32| {
            if f > i32::MAX as f32 {
                i32::MAX
            } else if f < i32::MIN as f32 {
                i32::MIN
            } else {
                f as i32
            }
        };

        let layout_bounds = Rect {
            min: point(to_i32(bounds.min.x.floor()), to_i32(bounds.min.y.floor())),
            max: point(to_i32(bounds.max.x.ceil()), to_i32(bounds.max.y.ceil())),
        };

        let inside_layout = |rect: Rect<i32>| {
            if rect.max.x < layout_bounds.min.x
                || rect.max.y < layout_bounds.min.y
                || rect.min.x > layout_bounds.max.x
                || rect.min.y > layout_bounds.max.y
            {
                return None;
            }
            Some(Rect {
                min: Point {
                    x: rect.min.x.max(layout_bounds.min.x),
                    y: rect.min.y.max(layout_bounds.min.y),
                },
                max: Point {
                    x: rect.max.x.min(layout_bounds.max.x),
                    y: rect.max.y.min(layout_bounds.max.y),
                },
            })
        };

        let mut no_match = true;

        let mut pixel_bounds = Rect {
            min: point(0, 0),
            max: point(0, 0),
        };

        for Rect { min, max } in glyphs
            .iter()
            .filter_map(|&(ref g, ..)| g.pixel_bounding_box())
            .filter_map(inside_layout)
        {
            if no_match || min.x < pixel_bounds.min.x {
                pixel_bounds.min.x = min.x;
            }
            if no_match || min.y < pixel_bounds.min.y {
                pixel_bounds.min.y = min.y;
            }
            if no_match || max.x > pixel_bounds.max.x {
                pixel_bounds.max.x = max.x;
            }
            if no_match || max.y > pixel_bounds.max.y {
                pixel_bounds.max.y = max.y;
            }
            no_match = false;
        }

        Some(pixel_bounds).filter(|_| !no_match)
    }

    #[inline]
    pub(crate) fn glyphs(&self) -> PositionedGlyphIter<'_, 'font> {
        self.glyphs.iter().map(|(g, ..)| g)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::f32;

    lazy_static::lazy_static! {
        static ref A_FONT: Font<'static> =
            Font::from_bytes(include_bytes!("../../fonts/DejaVuSansMono.ttf") as &[u8])
                .expect("Could not create rusttype::Font");
    }

    #[test]
    fn pixel_bounds_respect_layout_bounds() {
        let glyphs = GlyphCalculatorBuilder::using_font(A_FONT.clone()).build();
        let mut glyphs = glyphs.cache_scope();

        let section = Section {
            text: "Hello\n\
                   World",
            screen_position: (0.0, 20.0),
            bounds: (f32::INFINITY, 20.0),
            scale: Scale::uniform(16.0),
            layout: Layout::default().v_align(VerticalAlign::Bottom),
            ..Section::default()
        };

        let pixel_bounds = glyphs.pixel_bounds(&section).expect("None bounds");
        let layout_bounds = Layout::default()
            .v_align(VerticalAlign::Bottom)
            .bounds_rect(&SectionGeometry::from(&VariedSection::from(section)));

        assert!(
            layout_bounds.min.y <= pixel_bounds.min.y as f32,
            "expected {} <= {}",
            layout_bounds.min.y,
            pixel_bounds.min.y
        );

        assert!(
            layout_bounds.max.y >= pixel_bounds.max.y as f32,
            "expected {} >= {}",
            layout_bounds.max.y,
            pixel_bounds.max.y
        );
    }

    #[test]
    fn pixel_bounds_handle_infinity() {
        let glyphs = GlyphCalculatorBuilder::using_font(A_FONT.clone()).build();
        let mut glyphs = glyphs.cache_scope();

        for h_align in &[
            HorizontalAlign::Left,
            HorizontalAlign::Center,
            HorizontalAlign::Right,
        ] {
            for v_align in &[
                VerticalAlign::Top,
                VerticalAlign::Center,
                VerticalAlign::Bottom,
            ] {
                let section = Section {
                    text: "Hello\n\
                           World",
                    screen_position: (0.0, 20.0),
                    bounds: (f32::INFINITY, f32::INFINITY),
                    scale: Scale::uniform(16.0),
                    layout: Layout::default().h_align(*h_align).v_align(*v_align),
                    ..Section::default()
                };

                let inf_pixel_bounds = glyphs.pixel_bounds(&section);
                let large_pixel_bounds = glyphs.pixel_bounds(Section {
                    bounds: (1000.0, 1000.0),
                    ..section
                });

                assert_eq!(
                    inf_pixel_bounds, large_pixel_bounds,
                    "h={:?}, v={:?}",
                    h_align, v_align
                );
            }
        }
    }
}
