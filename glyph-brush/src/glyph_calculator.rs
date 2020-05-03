use super::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    borrow::Cow,
    collections::hash_map::Entry,
    fmt,
    hash::{BuildHasher, Hash, Hasher},
    iter, mem, slice,
    sync::{Mutex, MutexGuard},
};

/// `SectionGlyph` iterator.
pub type SectionGlyphIter<'a> = iter::Map<
    slice::Iter<'a, (SectionGlyph, Color)>,
    fn(&'a (SectionGlyph, Color)) -> &'a SectionGlyph,
>;

/// Common glyph layout logic.
///
/// # Example
/// ```no_run
/// # use glyph_brush::GlyphBrush;
/// use glyph_brush::GlyphCruncher;
///
/// # let glyph_brush: GlyphBrush<'_, ()> = unimplemented!();
/// let default_font = glyph_brush.fonts()[0];
/// ```
pub trait GlyphCruncher<F: Font> {
    // /// Returns the pixel bounding box for the input section using a custom layout.
    // /// The box is a conservative whole number pixel rectangle that can contain the section.
    // ///
    // /// If the section is empty or would result in no drawn glyphs will return `None`.
    // ///
    // /// [`glyphs_custom_layout`](#method.glyphs_custom_layout) should be preferred if the
    // /// bounds are to be used to inform further layout logic.
    // ///
    // /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    // fn pixel_bounds_custom_layout<'a, S, L>(
    //     &mut self,
    //     section: S,
    //     custom_layout: &L,
    // ) -> Option<Rectangle<i32>>
    // where
    //     L: GlyphPositioner + Hash,
    //     S: Into<Cow<'a, VariedSection<'a>>>;
    //
    // /// Returns the pixel bounding box for the input section. The box is a conservative
    // /// whole number pixel rectangle that can contain the section.
    // ///
    // /// If the section is empty or would result in no drawn glyphs will return `None`.
    // ///
    // /// [`glyph_bounds`](#method.glyph_bounds) should be preferred if the bounds are to be
    // /// used to inform further layout logic.
    // ///
    // /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    // #[inline]
    // fn pixel_bounds<'a, S>(&mut self, section: S) -> Option<Rect<i32>>
    // where
    //     S: Into<Cow<'a, VariedSection<'a>>>,
    // {
    //     let section = section.into();
    //     let layout = section.layout;
    //     self.pixel_bounds_custom_layout(section, &layout)
    // }

    /// Returns an iterator over the `PositionedGlyph`s of the given section with a custom layout.
    ///
    /// Generally only drawable glyphs will be returned as invisible glyphs, like spaces,
    /// are discarded during layout.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    fn glyphs_custom_layout<'a, 'b, S, L>(
        &'b mut self,
        section: S,
        custom_layout: &L,
    ) -> SectionGlyphIter<'b>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, VariedSection<'a>>>;

    /// Returns an iterator over the `PositionedGlyph`s of the given section.
    ///
    /// Generally only drawable glyphs will be returned as invisible glyphs, like spaces,
    /// are discarded during layout.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    fn glyphs<'a, 'b, S>(&'b mut self, section: S) -> SectionGlyphIter<'b>
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
    fn fonts(&self) -> &[F];

    /// Returns a bounding box for the section glyphs calculated using each glyph's
    /// vertical & horizontal metrics.
    ///
    /// If the section is empty or would result in no drawn glyphs will return `None`.
    ///
    /// Invisible glyphs, like spaces, are discarded during layout so trailing ones will
    /// not affect the bounds.
    ///
    /// The bounds will always lay within the specified layout bounds, ie that returned
    /// by the layout's `bounds_rect` function.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    fn glyph_bounds_custom_layout<'a, S, L>(
        &mut self,
        section: S,
        custom_layout: &L,
    ) -> Option<Rect>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, VariedSection<'a>>>;

    /// Returns a bounding box for the section glyphs calculated using each glyph's
    /// vertical & horizontal metrics.
    ///
    /// If the section is empty or would result in no drawn glyphs will return `None`.
    ///
    /// Invisible glyphs, like spaces, are discarded during layout so trailing ones will
    /// not affect the bounds.
    ///
    /// The bounds will always lay within the specified layout bounds, ie that returned
    /// by the layout's `bounds_rect` function.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    fn glyph_bounds<'a, S>(&mut self, section: S) -> Option<Rect>
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let layout = section.layout;
        self.glyph_bounds_custom_layout(section, &layout)
    }
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
pub struct GlyphCalculator<F: Font, H = DefaultSectionHasher> {
    fonts: Vec<F>,

    // cache of section-layout hash -> computed glyphs, this avoid repeated glyph computation
    // for identical layout/sections common to repeated frame rendering
    calculate_glyph_cache: Mutex<FxHashMap<u64, GlyphedSection>>,

    section_hasher: H,
}

impl<F: Font, H> fmt::Debug for GlyphCalculator<F, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlyphCalculator")
    }
}

impl<F: Font, H: BuildHasher + Clone> GlyphCalculator<F, H> {
    pub fn cache_scope(&self) -> GlyphCalculatorGuard<'_, F, H> {
        GlyphCalculatorGuard {
            fonts: &self.fonts,
            glyph_cache: self.calculate_glyph_cache.lock().unwrap(),
            cached: FxHashSet::default(),
            section_hasher: self.section_hasher.clone(),
        }
    }

    /// Returns the available fonts.
    ///
    /// The `FontId` corresponds to the index of the font data.
    pub fn fonts(&self) -> &[F] {
        &self.fonts
    }
}

/// [`GlyphCalculator`](struct.GlyphCalculator.html) scoped cache lock.
pub struct GlyphCalculatorGuard<'brush, F: 'brush, H = DefaultSectionHasher> {
    fonts: &'brush Vec<F>,
    glyph_cache: MutexGuard<'brush, FxHashMap<u64, GlyphedSection>>,
    cached: FxHashSet<u64>,
    section_hasher: H,
}

impl<F: Font, H: BuildHasher> GlyphCalculatorGuard<'_, F, H> {
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
            let glyphs = layout
                .calculate_glyphs(self.fonts, &geometry, &section.text)
                .into_iter()
                .map(|sg| {
                    let color = section.text[sg.section_index].1;
                    (sg, color)
                })
                .collect();

            entry.insert(GlyphedSection {
                bounds: layout.bounds_rect(&geometry),
                glyphs,
                z: section.z,
            });
        }

        section_hash
    }
}

impl<F: Font, H: BuildHasher> fmt::Debug for GlyphCalculatorGuard<'_, F, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlyphCalculatorGuard")
    }
}

impl<F: Font, H: BuildHasher> GlyphCruncher<F> for GlyphCalculatorGuard<'_, F, H> {
    // fn pixel_bounds_custom_layout<'a, S, L>(
    //     &mut self,
    //     section: S,
    //     custom_layout: &L,
    // ) -> Option<Rect<i32>>
    // where
    //     L: GlyphPositioner + Hash,
    //     S: Into<Cow<'a, VariedSection<'a>>>,
    // {
    //     let section_hash = self.cache_glyphs(&section.into(), custom_layout);
    //     self.cached.insert(section_hash);
    //     self.glyph_cache[&section_hash].pixel_bounds()
    // }

    fn glyphs_custom_layout<'a, 'b, S, L>(
        &'b mut self,
        section: S,
        custom_layout: &L,
    ) -> SectionGlyphIter<'b>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section_hash = self.cache_glyphs(&section.into(), custom_layout);
        self.cached.insert(section_hash);
        self.glyph_cache[&section_hash].glyphs()
    }

    fn glyph_bounds_custom_layout<'a, S, L>(
        &mut self,
        section: S,
        custom_layout: &L,
    ) -> Option<Rect>
    where
        L: GlyphPositioner + Hash,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let geometry = SectionGeometry::from(section.as_ref());

        let section_hash = self.cache_glyphs(&section, custom_layout);
        self.cached.insert(section_hash);

        self.glyph_cache[&section_hash]
            .glyphs()
            .fold(None, |b: Option<Rect>, sg| {
                let sfont = self.fonts[sg.font_id.0].as_scaled(sg.glyph.scale);
                let pos = sg.glyph.position;
                let lbound = Rect {
                    min: point(
                        pos.x - sfont.h_side_bearing(sg.glyph.id),
                        pos.y - sfont.ascent(),
                    ),
                    max: point(
                        pos.x + sfont.h_advance(sg.glyph.id),
                        pos.y - sfont.descent(),
                    ),
                };
                b.map(|b| {
                    let min_x = b.min.x.min(lbound.min.x);
                    let max_x = b.max.x.max(lbound.max.x);
                    let min_y = b.min.y.min(lbound.min.y);
                    let max_y = b.max.y.max(lbound.max.y);
                    Rect {
                        min: point(min_x, min_y),
                        max: point(max_x, max_y),
                    }
                })
                .or_else(|| Some(lbound))
            })
            .map(|mut b| {
                // cap the glyph bounds to the layout specified max bounds
                let Rect { min, max } = custom_layout.bounds_rect(&geometry);
                b.min.x = b.min.x.max(min.x);
                b.min.y = b.min.y.max(min.y);
                b.max.x = b.max.x.min(max.x);
                b.max.y = b.max.y.min(max.y);
                b
            })
    }

    #[inline]
    fn fonts(&self) -> &[F] {
        &self.fonts
    }
}

impl<F, H> Drop for GlyphCalculatorGuard<'_, F, H> {
    fn drop(&mut self) {
        let cached = mem::take(&mut self.cached);
        self.glyph_cache.retain(|key, _| cached.contains(key));
    }
}

/// Builder for a [`GlyphCalculator`](struct.GlyphCalculator.html).
///
/// # Example
///
/// ```no_run
/// use glyph_brush::GlyphCalculatorBuilder;
///
/// let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
/// let mut glyphs = GlyphCalculatorBuilder::using_font_bytes(dejavu).build();
/// ```
#[derive(Debug, Clone)]
pub struct GlyphCalculatorBuilder<F, H = DefaultSectionHasher> {
    font_data: Vec<F>,
    section_hasher: H,
}

impl GlyphCalculatorBuilder<()> {
    /// Specifies the default font data used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font_bytes<'a>(font_0_data: &'a [u8]) -> GlyphCalculatorBuilder<FontRef<'a>> {
        Self::using_font(FontRef::try_from_slice(font_0_data).unwrap())
    }

    pub fn using_fonts_bytes<'a, V>(font_data: V) -> GlyphCalculatorBuilder<FontRef<'a>>
    where
        V: IntoIterator<Item = &'a [u8]>,
    {
        Self::using_fonts(
            font_data
                .into_iter()
                .map(|data| FontRef::try_from_slice(data).unwrap())
                .collect::<Vec<_>>(),
        )
    }

    /// Specifies the default font used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font<F: Font>(font_0_data: F) -> GlyphCalculatorBuilder<F> {
        Self::using_fonts(vec![font_0_data])
    }

    pub fn using_fonts<F: Font, V: Into<Vec<F>>>(fonts: V) -> GlyphCalculatorBuilder<F> {
        GlyphCalculatorBuilder {
            font_data: fonts.into(),
            section_hasher: DefaultSectionHasher::default(),
        }
    }
}

impl<'a, H: BuildHasher> GlyphCalculatorBuilder<FontRef<'a>, H> {
    /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
    /// [`using_font_bytes`](#method.using_font_bytes).
    ///
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font_bytes(&mut self, font_data: &'a [u8]) -> FontId {
        self.add_font(FontRef::try_from_slice(font_data).unwrap())
    }
}

impl<F: Font, H: BuildHasher> GlyphCalculatorBuilder<F, H> {
    /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
    /// [`using_font_bytes`](#method.using_font_bytes).
    ///
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font(&mut self, font_data: F) -> FontId {
        self.font_data.push(font_data);
        FontId(self.font_data.len() - 1)
    }

    /// Sets the section hasher. `GlyphCalculator` cannot handle absolute section hash collisions
    /// so use a good hash algorithm.
    ///
    /// This hasher is used to distinguish sections, rather than for hashmap internal use.
    ///
    /// Defaults to [xxHash](https://docs.rs/twox-hash).
    pub fn section_hasher<T: BuildHasher>(self, section_hasher: T) -> GlyphCalculatorBuilder<F, T> {
        GlyphCalculatorBuilder {
            font_data: self.font_data,
            section_hasher,
        }
    }

    /// Builds a `GlyphCalculator`
    pub fn build(self) -> GlyphCalculator<F, H> {
        GlyphCalculator {
            fonts: self.font_data,
            calculate_glyph_cache: Mutex::default(),
            section_hasher: self.section_hasher,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GlyphedSection {
    pub bounds: Rect,
    pub glyphs: Vec<(SectionGlyph, Color)>,
    pub z: f32,
}

impl GlyphedSection {
    // pub(crate) fn pixel_bounds(&self) -> Option<Rect<i32>> {
    //     let Self {
    //         ref glyphs, bounds, ..
    //     } = *self;
    //
    //     let to_i32 = |f: f32| {
    //         if f > i32::MAX as f32 {
    //             i32::MAX
    //         } else if f < i32::MIN as f32 {
    //             i32::MIN
    //         } else {
    //             f as i32
    //         }
    //     };
    //
    //     let section_bounds = Rect {
    //         min: point(to_i32(bounds.min.x.floor()), to_i32(bounds.min.y.floor())),
    //         max: point(to_i32(bounds.max.x.ceil()), to_i32(bounds.max.y.ceil())),
    //     };
    //
    //     let inside_layout = |rect: Rect<i32>| {
    //         if rect.max.x < section_bounds.min.x
    //             || rect.max.y < section_bounds.min.y
    //             || rect.min.x > section_bounds.max.x
    //             || rect.min.y > section_bounds.max.y
    //         {
    //             return None;
    //         }
    //         Some(Rect {
    //             min: Point {
    //                 x: rect.min.x.max(section_bounds.min.x),
    //                 y: rect.min.y.max(section_bounds.min.y),
    //             },
    //             max: Point {
    //                 x: rect.max.x.min(section_bounds.max.x),
    //                 y: rect.max.y.min(section_bounds.max.y),
    //             },
    //         })
    //     };
    //
    //     let mut no_match = true;
    //
    //     let mut pixel_bounds = Rect {
    //         min: point(0, 0),
    //         max: point(0, 0),
    //     };
    //
    //     for Rect { min, max } in glyphs
    //         .iter()
    //         .filter_map(|&(ref g, ..)| g.pixel_bounding_box())
    //         .filter_map(inside_layout)
    //     {
    //         if no_match || min.x < pixel_bounds.min.x {
    //             pixel_bounds.min.x = min.x;
    //         }
    //         if no_match || min.y < pixel_bounds.min.y {
    //             pixel_bounds.min.y = min.y;
    //         }
    //         if no_match || max.x > pixel_bounds.max.x {
    //             pixel_bounds.max.x = max.x;
    //         }
    //         if no_match || max.y > pixel_bounds.max.y {
    //             pixel_bounds.max.y = max.y;
    //         }
    //         no_match = false;
    //     }
    //
    //     Some(pixel_bounds).filter(|_| !no_match)
    // }

    #[inline]
    pub(crate) fn glyphs(&self) -> SectionGlyphIter<'_> {
        self.glyphs.iter().map(|(sg, ..)| sg)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::*;
    use once_cell::sync::Lazy;
    use std::f32;

    static MONO_FONT: Lazy<FontRef<'static>> = Lazy::new(|| {
        FontRef::try_from_slice(include_bytes!("../../fonts/DejaVuSansMono.ttf") as &[u8])
            .expect("Could not create rusttype::Font")
    });
    static OPEN_SANS_LIGHT: Lazy<FontRef<'static>> = Lazy::new(|| {
        FontRef::try_from_slice(include_bytes!("../../fonts/OpenSans-Light.ttf") as &[u8])
            .expect("Could not create rusttype::Font")
    });

    // #[test]
    // fn pixel_bounds_respect_layout_bounds() {
    //     let glyphs = GlyphCalculatorBuilder::using_font(MONO_FONT.clone()).build();
    //     let mut glyphs = glyphs.cache_scope();
    //
    //     let section = Section {
    //         text: "Hello\n\
    //                World",
    //         screen_position: (0.0, 20.0),
    //         bounds: (f32::INFINITY, 20.0),
    //         scale: PxScale::from(16.0),
    //         layout: Layout::default().v_align(VerticalAlign::Bottom),
    //         ..Section::default()
    //     };
    //
    //     let pixel_bounds = glyphs.pixel_bounds(&section).expect("None bounds");
    //     let layout_bounds = Layout::default()
    //         .v_align(VerticalAlign::Bottom)
    //         .bounds_rect(&SectionGeometry::from(&VariedSection::from(section)));
    //
    //     assert!(
    //         layout_bounds.min.y <= pixel_bounds.min.y as f32,
    //         "expected {} <= {}",
    //         layout_bounds.min.y,
    //         pixel_bounds.min.y
    //     );
    //
    //     assert!(
    //         layout_bounds.max.y >= pixel_bounds.max.y as f32,
    //         "expected {} >= {}",
    //         layout_bounds.max.y,
    //         pixel_bounds.max.y
    //     );
    // }
    //
    // #[test]
    // fn pixel_bounds_handle_infinity() {
    //     let glyphs = GlyphCalculatorBuilder::using_font(MONO_FONT.clone()).build();
    //     let mut glyphs = glyphs.cache_scope();
    //
    //     for h_align in &[
    //         HorizontalAlign::Left,
    //         HorizontalAlign::Center,
    //         HorizontalAlign::Right,
    //     ] {
    //         for v_align in &[
    //             VerticalAlign::Top,
    //             VerticalAlign::Center,
    //             VerticalAlign::Bottom,
    //         ] {
    //             let section = Section {
    //                 text: "Hello\n\
    //                        World",
    //                 screen_position: (0.0, 20.0),
    //                 bounds: (f32::INFINITY, f32::INFINITY),
    //                 scale: PxScale::from(16.0),
    //                 layout: Layout::default().h_align(*h_align).v_align(*v_align),
    //                 ..Section::default()
    //             };
    //
    //             let inf_pixel_bounds = glyphs.pixel_bounds(&section);
    //             let large_pixel_bounds = glyphs.pixel_bounds(Section {
    //                 bounds: (1000.0, 1000.0),
    //                 ..section
    //             });
    //
    //             assert_eq!(
    //                 inf_pixel_bounds, large_pixel_bounds,
    //                 "h={:?}, v={:?}",
    //                 h_align, v_align
    //             );
    //         }
    //     }
    // }

    #[test]
    fn glyph_bounds() {
        let glyphs = GlyphCalculatorBuilder::using_font(MONO_FONT.clone()).build();
        let mut glyphs = glyphs.cache_scope();

        let scale = PxScale::from(16.0);
        let section = Section {
            text: "Hello World",
            screen_position: (0.0, 0.0),
            scale,
            ..<_>::default()
        };

        let g_bounds = glyphs.glyph_bounds(&section).expect("None bounds");

        for sg in glyphs.glyphs(&section) {
            eprintln!("{:?}", sg.glyph.position);
        }

        let sfont = MONO_FONT.as_scaled(scale);
        assert_relative_eq!(g_bounds.min.y, 0.0);
        assert_relative_eq!(g_bounds.max.y, sfont.ascent() - sfont.descent());

        // no left-side bearing expected
        assert_relative_eq!(g_bounds.min.x, 0.0);

        // the width should be to 11 * any glyph advance width as this font is monospaced
        let g_width = sfont.h_advance(MONO_FONT.glyph_id('W'));
        assert_relative_eq!(g_bounds.max.x, g_width * 11.0, epsilon = f32::EPSILON);
    }

    #[test]
    fn glyph_bounds_respect_layout_bounds() {
        let glyphs = GlyphCalculatorBuilder::using_font(MONO_FONT.clone()).build();
        let mut glyphs = glyphs.cache_scope();

        let section = Section {
            text: "Hello\n\
                   World",
            screen_position: (0.0, 20.0),
            bounds: (f32::INFINITY, 20.0),
            scale: PxScale::from(16.0),
            ..<_>::default()
        };

        let g_bounds = glyphs.glyph_bounds(&section).expect("None bounds");
        let bounds_rect =
            Layout::default().bounds_rect(&SectionGeometry::from(&VariedSection::from(section)));

        assert!(
            bounds_rect.min.y <= g_bounds.min.y as f32,
            "expected {} <= {}",
            bounds_rect.min.y,
            g_bounds.min.y
        );

        assert!(
            bounds_rect.max.y >= g_bounds.max.y as f32,
            "expected {} >= {}",
            bounds_rect.max.y,
            g_bounds.max.y
        );
    }

    #[test]
    fn glyphed_section_eq() {
        let glyph = MONO_FONT
            .glyph_id('a')
            .with_scale_and_position(16.0, point(50.0, 60.0));
        let color = [1.0, 0.9, 0.8, 0.7];

        let a = GlyphedSection {
            bounds: Rect {
                min: point(1.0, 2.0),
                max: point(300.0, 400.0),
            },
            z: 0.5,
            glyphs: vec![(
                SectionGlyph {
                    section_index: 0,
                    byte_index: 0,
                    glyph: glyph.clone(),
                    font_id: FontId(0),
                },
                color,
            )],
        };
        let mut b = GlyphedSection {
            bounds: Rect {
                min: point(1.0, 2.0),
                max: point(300.0, 400.0),
            },
            z: 0.5,
            glyphs: vec![(
                SectionGlyph {
                    section_index: 0,
                    byte_index: 0,
                    glyph: glyph,
                    font_id: FontId(0),
                },
                color,
            )],
        };

        assert_eq!(a, b);

        b.glyphs[0].0.glyph.position = point(50.0, 61.0);

        assert_ne!(a, b);
    }

    /// Issue #87
    #[test]
    fn glyph_bound_section_bound_consistency() {
        let calc = GlyphCalculatorBuilder::using_font(OPEN_SANS_LIGHT.clone()).build();
        let mut calc = calc.cache_scope();

        let section = Section {
            text: "Eins Zwei Drei Vier Funf",
            scale: PxScale::from(20.0),
            ..<_>::default()
        };

        let glyph_bounds = calc.glyph_bounds(&section).expect("None bounds");

        // identical section with bounds that should be wide enough
        let bounded_section = Section {
            bounds: (glyph_bounds.width(), glyph_bounds.height()),
            ..section
        };

        let glyphs: Vec<_> = calc.glyphs(&section).cloned().collect();
        let bounded_glyphs: Vec<_> = calc.glyphs(&bounded_section).collect();

        assert_eq!(glyphs.len(), bounded_glyphs.len());

        for (sg, bounded_sg) in glyphs.iter().zip(bounded_glyphs.into_iter()) {
            assert_relative_eq!(sg.glyph.position.x, bounded_sg.glyph.position.x);
            assert_relative_eq!(sg.glyph.position.y, bounded_sg.glyph.position.y);
        }
    }

    /// Issue #87
    #[test]
    fn glyph_bound_section_bound_consistency_trailing_space() {
        let calc = GlyphCalculatorBuilder::using_font(OPEN_SANS_LIGHT.clone()).build();
        let mut calc = calc.cache_scope();

        let section = Section {
            text: "Eins Zwei Drei Vier Funf ",
            scale: PxScale::from(20.0),
            ..<_>::default()
        };

        let glyph_bounds = calc.glyph_bounds(&section).expect("None bounds");

        // identical section with bounds that should be wide enough
        let bounded_section = Section {
            bounds: (glyph_bounds.width(), glyph_bounds.height()),
            ..section
        };

        let glyphs: Vec<_> = calc.glyphs(&section).cloned().collect();
        let bounded_glyphs: Vec<_> = calc.glyphs(&bounded_section).collect();

        assert_eq!(glyphs.len(), bounded_glyphs.len());

        for (sg, bounded_sg) in glyphs.iter().zip(bounded_glyphs.into_iter()) {
            assert_relative_eq!(sg.glyph.position.x, bounded_sg.glyph.position.x);
            assert_relative_eq!(sg.glyph.position.y, bounded_sg.glyph.position.y);
        }
    }

    /// Similar to `glyph_bound_section_bound_consistency` but produces a floating point
    /// error between the calculated glyph_bounds bounds & those used during layout.
    #[test]
    fn glyph_bound_section_bound_consistency_floating_point() {
        let calc = GlyphCalculatorBuilder::using_font(MONO_FONT.clone()).build();
        let mut calc = calc.cache_scope();

        let section = Section {
            text: "Eins Zwei Drei Vier Funf",
            ..<_>::default()
        };

        let glyph_bounds = calc.glyph_bounds(&section).expect("None bounds");

        // identical section with bounds that should be wide enough
        let bounded_section = Section {
            bounds: (glyph_bounds.width(), glyph_bounds.height()),
            ..section
        };

        let glyphs: Vec<_> = calc.glyphs(&section).cloned().collect();
        let bounded_glyphs: Vec<_> = calc.glyphs(&bounded_section).collect();

        assert_eq!(glyphs.len(), bounded_glyphs.len());

        for (sg, bounded_sg) in glyphs.iter().zip(bounded_glyphs.into_iter()) {
            assert_relative_eq!(sg.glyph.position.x, bounded_sg.glyph.position.x);
            assert_relative_eq!(sg.glyph.position.y, bounded_sg.glyph.position.y);
        }
    }
}
