mod builder;

pub use self::builder::*;

use super::*;
use glyph_brush_draw_cache::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    borrow::Cow,
    fmt,
    hash::{BuildHasher, Hash, Hasher},
    mem,
};

/// A hash of `Section` data
type SectionHash = u64;

/// Object allowing glyph drawing, containing cache state. Manages glyph positioning cacheing,
/// glyph draw caching & efficient GPU texture cache updating.
///
/// Build using a [`GlyphBrushBuilder`](struct.GlyphBrushBuilder.html).
///
/// Also see [`GlyphCruncher`](trait.GlyphCruncher.html) trait which providers extra functionality,
/// such as [`glyph_bounds`](trait.GlyphCruncher.html#method.glyph_bounds).
///
/// # Caching behaviour
///
/// Calls to [`GlyphBrush::queue`](#method.queue),
/// [`GlyphBrush::pixel_bounds`](#method.pixel_bounds), [`GlyphBrush::glyphs`](#method.glyphs)
/// calculate the positioned glyphs for a section.
/// This is cached so future calls to any of the methods for the same section are much
/// cheaper. In the case of [`GlyphBrush::queue`](#method.queue) the calculations will also be
/// used for actual drawing.
///
/// The cache for a section will be **cleared** after a
/// [`GlyphBrush::process_queued`](#method.process_queued) call when that section has not been used
/// since the previous call.
///
/// # Texture caching behaviour
/// Note the gpu/draw cache may contain multiple versions of the same glyph at different
/// subpixel positions.
/// This is required for high quality text as a glyph's positioning is always exactly aligned
/// to it's draw positioning.
///
/// This behaviour can be adjusted with
/// [`GlyphBrushBuilder::gpu_cache_position_tolerance`]
/// (struct.GlyphBrushBuilder.html#method.gpu_cache_position_tolerance).
pub struct GlyphBrush<F, V, H = DefaultSectionHasher> {
    fonts: Vec<F>,
    texture_cache: DrawCache,
    last_draw: LastDrawInfo,

    // cache of section-layout hash -> computed glyphs, this avoid repeated glyph computation
    // for identical layout/sections common to repeated frame rendering
    calculate_glyph_cache: FxHashMap<SectionHash, Glyphed<V>>,

    last_frame_seq_id_sections: Vec<SectionHashDetail>,
    frame_seq_id_sections: Vec<SectionHashDetail>,

    // buffer of section-layout hashs (that must exist in the calculate_glyph_cache)
    // to be used on the next `process_queued` call
    section_buffer: Vec<SectionHash>,

    // Set of section hashs to keep in the glyph cache this frame even if they haven't been drawn
    keep_in_cache: FxHashSet<SectionHash>,

    // config
    cache_glyph_positioning: bool,
    cache_glyph_drawing: bool,

    section_hasher: H,

    last_pre_positioned: Vec<Glyphed<V>>,
    pre_positioned: Vec<Glyphed<V>>,
}

impl<F, V, H> fmt::Debug for GlyphBrush<F, V, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlyphBrush")
    }
}

impl<F, V, H> GlyphCruncher<F> for GlyphBrush<F, V, H>
where
    F: Font,
    V: Clone + 'static,
    H: BuildHasher,
{
    // fn pixel_bounds_custom_layout<'a, S, L>(
    //     &mut self,
    //     section: S,
    //     custom_layout: &L,
    // ) -> Option<Rectangle<i32>>
    // where
    //     L: GlyphPositioner + Hash,
    //     S: Into<Cow<'a, VariedSection<'a>>>,
    // {
    //     let section_hash = self.cache_glyphs(&section.into(), custom_layout);
    //     self.keep_in_cache.insert(section_hash);
    //     self.calculate_glyph_cache[&section_hash]
    //         .positioned
    //         .pixel_bounds()
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
        self.keep_in_cache.insert(section_hash);
        self.calculate_glyph_cache[&section_hash]
            .positioned
            .glyphs()
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
        self.keep_in_cache.insert(section_hash);
        self.calculate_glyph_cache[&section_hash]
            .positioned
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

impl<'a, V, H: BuildHasher> GlyphBrush<FontRef<'a>, V, H> {
    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    ///
    /// # Example
    ///
    /// ```
    /// use glyph_brush::{ab_glyph::FontRef, GlyphBrush, GlyphBrushBuilder, Section};
    /// # type Vertex = ();
    ///
    /// // dejavu is built as default `FontId(0)`
    /// let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// let mut glyph_brush: GlyphBrush<FontRef<'static>, Vertex> =
    ///     GlyphBrushBuilder::using_font_bytes(dejavu).build();
    ///
    /// // some time later, add another font referenced by a new `FontId`
    /// let open_sans_italic: &[u8] = include_bytes!("../../fonts/OpenSans-Italic.ttf");
    /// let open_sans_italic_id = glyph_brush.add_font_bytes(open_sans_italic);
    /// ```
    pub fn add_font_bytes(&mut self, font_data: &'a [u8]) -> FontId {
        self.add_font(FontRef::try_from_slice(font_data).unwrap())
    }
}

impl<F, V, H: BuildHasher> GlyphBrush<F, V, H> {
    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font(&mut self, font_data: F) -> FontId {
        self.fonts.push(font_data);
        FontId(self.fonts.len() - 1)
    }
}

impl<F, V, H> GlyphBrush<F, V, H>
where
    F: Font,
    V: Clone + 'static,
    H: BuildHasher,
{
    /// Queues a section/layout to be processed by the next call of
    /// [`process_queued`](struct.GlyphBrush.html#method.process_queued). Can be called multiple
    /// times to queue multiple sections for drawing.
    ///
    /// Used to provide custom `GlyphPositioner` logic, if using built-in
    /// [`Layout`](enum.Layout.html) simply use [`queue`](struct.GlyphBrush.html#method.queue)
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    pub fn queue_custom_layout<'a, S, G>(&mut self, section: S, custom_layout: &G)
    where
        G: GlyphPositioner,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        if cfg!(debug_assertions) {
            for (text, ..) in &section.text {
                assert!(self.fonts.len() > text.font_id.0, "Invalid font id");
            }
        }
        let section_hash = self.cache_glyphs(&section, custom_layout);
        self.section_buffer.push(section_hash);
        self.keep_in_cache.insert(section_hash);
    }

    /// Queues a section/layout to be processed by the next call of
    /// [`process_queued`](struct.GlyphBrush.html#method.process_queued). Can be called multiple
    /// times to queue multiple sections for drawing.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    ///
    /// ```no_run
    /// # use glyph_brush::*;
    /// # let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// # let mut glyph_brush: GlyphBrush<'_, ()> = GlyphBrushBuilder::using_font_bytes(dejavu).build();
    /// glyph_brush.queue(Section {
    ///     text: "Hello glyph_brush",
    ///     ..Section::default()
    /// });
    /// ```
    pub fn queue<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let layout = section.layout;
        self.queue_custom_layout(section, &layout)
    }

    /// Queues pre-positioned glyphs to be processed by the next call of
    /// [`process_queued`](struct.GlyphBrush.html#method.process_queued). Can be called multiple
    /// times.
    pub fn queue_pre_positioned(
        &mut self,
        glyphs: Vec<(SectionGlyph, Color)>,
        bounds: Rect,
        z: f32,
    ) {
        self.pre_positioned
            .push(Glyphed::new(GlyphedSection { glyphs, bounds, z }));
    }

    /// Returns the calculate_glyph_cache key for this sections glyphs
    #[allow(clippy::map_entry)] // further borrows are required after the contains_key check
    fn cache_glyphs<L>(&mut self, section: &VariedSection<'_>, layout: &L) -> SectionHash
    where
        L: GlyphPositioner,
    {
        let section_hash = SectionHashDetail::new(&self.section_hasher, section, layout);
        // section id used to find a similar calculated layout from last frame
        let frame_seq_id = self.frame_seq_id_sections.len();
        self.frame_seq_id_sections.push(section_hash);

        if self.cache_glyph_positioning {
            if !self.calculate_glyph_cache.contains_key(&section_hash.full) {
                let geometry = SectionGeometry::from(section);

                let recalculated_glyphs = self
                    .last_frame_seq_id_sections
                    .get(frame_seq_id)
                    .cloned()
                    .and_then(|hash| {
                        let change = hash.diff(section_hash);
                        if let SectionChange::Layout(GlyphChange::Unknown) = change {
                            return None;
                        }

                        let recalced = if self.keep_in_cache.contains(&hash.full) {
                            let cached = self.calculate_glyph_cache.get(&hash.full)?;
                            change.recalculate_glyphs(
                                layout,
                                cached.positioned.glyphs.iter().cloned(),
                                &self.fonts,
                                &geometry,
                                &section.text,
                            )
                        } else {
                            let old = self.calculate_glyph_cache.remove(&hash.full)?;
                            change.recalculate_glyphs(
                                layout,
                                old.positioned.glyphs.into_iter(),
                                &self.fonts,
                                &geometry,
                                &section.text,
                            )
                        };

                        Some(recalced)
                    });

                self.calculate_glyph_cache.insert(
                    section_hash.full,
                    Glyphed::new(GlyphedSection {
                        bounds: layout.bounds_rect(&geometry),
                        glyphs: recalculated_glyphs.unwrap_or_else(|| {
                            layout
                                .calculate_glyphs(&self.fonts, &geometry, &section.text)
                                .into_iter()
                                .map(|sg| {
                                    let color = section.text[sg.section_index].1;
                                    (sg, color)
                                })
                                .collect()
                        }),
                        z: section.z,
                    }),
                );
            }
        } else {
            let geometry = SectionGeometry::from(section);
            let glyphs = layout
                .calculate_glyphs(&self.fonts, &geometry, &section.text)
                .into_iter()
                .map(|sg| {
                    let color = section.text[sg.section_index].1;
                    (sg, color)
                })
                .collect();
            self.calculate_glyph_cache.insert(
                section_hash.full,
                Glyphed::new(GlyphedSection {
                    bounds: layout.bounds_rect(&geometry),
                    glyphs,
                    z: section.z,
                }),
            );
        }
        section_hash.full
    }

    /// Rebuilds the logical texture cache with new dimensions. Should be avoided if possible.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use glyph_brush::*;
    /// # let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// # let mut glyph_brush: GlyphBrush<'_, ()> = GlyphBrushBuilder::using_font_bytes(dejavu).build();
    /// glyph_brush.resize_texture(512, 512);
    /// ```
    pub fn resize_texture(&mut self, new_width: u32, new_height: u32) {
        self.texture_cache
            .to_builder()
            .dimensions(new_width, new_height)
            .rebuild(&mut self.texture_cache);

        self.last_draw = LastDrawInfo::default();

        // invalidate any previous cache position data
        for glyphed in self.calculate_glyph_cache.values_mut() {
            glyphed.invalidate_texture_positions();
        }
    }

    /// Returns the logical texture cache pixel dimensions `(width, height)`.
    pub fn texture_dimensions(&self) -> (u32, u32) {
        self.texture_cache.dimensions()
    }

    fn cleanup_frame(&mut self) {
        if self.cache_glyph_positioning {
            // clear section_buffer & trim calculate_glyph_cache to active sections
            let active = mem::take(&mut self.keep_in_cache);
            self.calculate_glyph_cache
                .retain(|key, _| active.contains(key));
            mem::replace(&mut self.keep_in_cache, active);

            self.keep_in_cache.clear();

            self.section_buffer.clear();
        } else {
            self.section_buffer.clear();
            self.calculate_glyph_cache.clear();
            self.keep_in_cache.clear();
        }

        mem::swap(
            &mut self.last_frame_seq_id_sections,
            &mut self.frame_seq_id_sections,
        );
        self.frame_seq_id_sections.clear();

        mem::swap(&mut self.last_pre_positioned, &mut self.pre_positioned);
        self.pre_positioned.clear();
    }

    /// Retains the section in the cache as if it had been used in the last draw-frame.
    ///
    /// Should not generally be necessary, see [caching behaviour](#caching-behaviour).
    pub fn keep_cached_custom_layout<'a, S, G>(&mut self, section: S, custom_layout: &G)
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
        G: GlyphPositioner,
    {
        if !self.cache_glyph_positioning {
            return;
        }
        let section = section.into();
        if cfg!(debug_assertions) {
            for text in &section.text {
                assert!(self.fonts.len() > text.0.font_id.0, "Invalid font id");
            }
        }

        let section_hash = SectionHashDetail::new(&self.section_hasher, &section, custom_layout);
        self.keep_in_cache.insert(section_hash.full);
    }

    /// Retains the section in the cache as if it had been used in the last draw-frame.
    ///
    /// Should not generally be necessary, see [caching behaviour](#caching-behaviour).
    pub fn keep_cached<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let layout = section.layout;
        self.keep_cached_custom_layout(section, &layout);
    }
}

// `Font + Sync` stuff
impl<F, V, H> GlyphBrush<F, V, H>
where
    F: Font + Sync,
    V: Clone + 'static,
    H: BuildHasher,
{
    /// Processes all queued sections, calling texture update logic when necessary &
    /// returning a `BrushAction`.
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Two closures are required:
    /// * `update_texture` is called when new glyph texture data has been drawn for update in the
    ///   actual texture.
    ///   The arguments are the rect position of the data in the texture & the byte data itself
    ///   which is a single `u8` alpha value per pixel.
    /// * `to_vertex` maps a single glyph's `GlyphVertex` data into a generic vertex type. The
    ///   mapped vertices are returned in an `Ok(BrushAction::Draw(vertices))` result.
    ///   It's recommended to use a single vertex per glyph quad for best performance.
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// ```no_run
    /// # use glyph_brush::*;
    /// # fn main() -> Result<(), BrushError> {
    /// # let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// # let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build();
    /// # fn update_texture(_: glyph_brush::rusttype::Rect<u32>, _: &[u8]) {}
    /// # let into_vertex = |_| ();
    /// glyph_brush.process_queued(
    ///     |rect, tex_data| update_texture(rect, tex_data),
    ///     |vertex_data| into_vertex(vertex_data),
    /// )?
    /// # ;
    /// # Ok(())
    /// # }
    /// ```
    pub fn process_queued<Up, VF>(
        &mut self,
        update_texture: Up,
        to_vertex: VF,
    ) -> Result<BrushAction<V>, BrushError>
    where
        Up: FnMut(Rectangle<u32>, &[u8]),
        VF: Fn(GlyphVertex) -> V + Copy,
    {
        let draw_info = LastDrawInfo {
            text_state: {
                let mut s = self.section_hasher.build_hasher();
                self.section_buffer.hash(&mut s);
                s.finish()
            },
        };

        let result = if !self.cache_glyph_drawing
            || self.last_draw != draw_info
            || self.last_pre_positioned != self.pre_positioned
        {
            let mut some_text = false;
            // Everything in the section_buffer should also be here. The extras should also
            // be retained in the texture cache avoiding cache thrashing if they are rendered
            // in a 2-draw per frame style.
            for section_hash in &self.keep_in_cache {
                for &(ref sg, ..) in self
                    .calculate_glyph_cache
                    .get(section_hash)
                    .iter()
                    .flat_map(|gs| &gs.positioned.glyphs)
                {
                    self.texture_cache
                        .queue_glyph(sg.font_id.0, sg.glyph.clone());
                    some_text = true;
                }
            }

            for &(ref sg, ..) in self
                .pre_positioned
                .iter()
                .flat_map(|p| &p.positioned.glyphs)
            {
                self.texture_cache
                    .queue_glyph(sg.font_id.0, sg.glyph.clone());
                some_text = true;
            }

            if some_text {
                match self.texture_cache.cache_queued(&self.fonts, update_texture) {
                    Ok(CachedBy::Adding) => {}
                    Ok(CachedBy::Reordering) => {
                        for glyphed in self.calculate_glyph_cache.values_mut() {
                            glyphed.invalidate_texture_positions();
                        }
                    }
                    Err(_) => {
                        let (width, height) = self.texture_cache.dimensions();
                        return Err(BrushError::TextureTooSmall {
                            suggested: (width * 2, height * 2),
                        });
                    }
                }
            }

            self.last_draw = draw_info;

            BrushAction::Draw({
                let mut verts = Vec::new();

                for hash in &self.section_buffer {
                    let glyphed = self.calculate_glyph_cache.get_mut(hash).unwrap();
                    glyphed.ensure_vertices(&self.texture_cache, to_vertex);
                    verts.extend(glyphed.vertices.iter().cloned());
                }

                for glyphed in &mut self.pre_positioned {
                    // pre-positioned glyph vertices can't be cached so
                    // generate & move straight into draw vec
                    glyphed.ensure_vertices(&self.texture_cache, to_vertex);
                    verts.append(&mut glyphed.vertices);
                }

                verts
            })
        } else {
            BrushAction::ReDraw
        };

        self.cleanup_frame();
        Ok(result)
    }
}

impl<F: Font + Clone, V, H: BuildHasher + Clone> GlyphBrush<F, V, H> {
    /// Return a [`GlyphBrushBuilder`](struct.GlyphBrushBuilder.html) prefilled with the
    /// properties of this `GlyphBrush`.
    ///
    /// # Example
    ///
    /// ```
    /// # use glyph_brush::{*, ab_glyph::*};
    /// # type Vertex = ();
    /// # let sans = FontRef::try_from_slice(&include_bytes!("../../fonts/DejaVuSans.ttf")[..]).unwrap();
    /// let glyph_brush: GlyphBrush<FontRef<'static>, Vertex> = GlyphBrushBuilder::using_font(sans)
    ///     .initial_cache_size((128, 128))
    ///     .build();
    ///
    /// let new_brush: GlyphBrush<FontRef<'static>, Vertex> = glyph_brush.to_builder().build();
    /// assert_eq!(new_brush.texture_dimensions(), (128, 128));
    /// ```
    pub fn to_builder(&self) -> GlyphBrushBuilder<F, H> {
        let mut builder = GlyphBrushBuilder::using_fonts(self.fonts.clone())
            .cache_glyph_positioning(self.cache_glyph_positioning)
            .cache_glyph_drawing(self.cache_glyph_drawing)
            .section_hasher(self.section_hasher.clone());
        builder.gpu_cache_builder = self.texture_cache.to_builder();
        builder
    }
}

#[derive(Debug, Default, PartialEq)]
struct LastDrawInfo {
    text_state: u64,
}

/// Data used to generate vertex information for a single glyph
#[derive(Debug)]
pub struct GlyphVertex {
    pub tex_coords: Rect,
    pub pixel_coords: Rect,
    pub bounds: Rect,
    pub color: Color,
    pub z: f32,
}

/// Actions that should be taken after processing queue data
#[derive(Debug)]
pub enum BrushAction<V> {
    /// Draw new/changed vertix data.
    Draw(Vec<V>),
    /// Re-draw last frame's vertices unmodified.
    ReDraw,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BrushError {
    /// Texture is too small to cache queued glyphs
    ///
    /// A larger suggested size is included.
    TextureTooSmall { suggested: (u32, u32) },
}
impl fmt::Display for BrushError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TextureTooSmall { .. } => write!(f, "TextureTooSmall"),
        }
    }
}
impl std::error::Error for BrushError {
    fn description(&self) -> &str {
        match self {
            BrushError::TextureTooSmall { .. } => "Texture is too small to cache queued glyphs",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SectionHashDetail {
    /// hash of text (- color - alpha - geo - z)
    text: SectionHash,
    // hash of text + color (- alpha - geo - z)
    text_color: SectionHash,
    /// hash of text + color + alpha (- geo - z)
    test_alpha_color: SectionHash,
    /// hash of text  + color + alpha + geo + z
    full: SectionHash,

    /// copy of geometry for later comparison
    geometry: SectionGeometry,
}

impl SectionHashDetail {
    #[inline]
    fn new<H, L>(build_hasher: &H, section: &VariedSection<'_>, layout: &L) -> Self
    where
        H: BuildHasher,
        L: GlyphPositioner,
    {
        let parts = section.to_hashable_parts();

        let mut s = build_hasher.build_hasher();
        layout.hash(&mut s);
        parts.hash_text_no_color(&mut s);
        let text_hash = s.finish();

        parts.hash_color(&mut s);
        let text_color_hash = s.finish();

        parts.hash_alpha(&mut s);
        let test_alpha_color_hash = s.finish();

        parts.hash_geometry(&mut s);
        parts.hash_z(&mut s);
        let full_hash = s.finish();

        Self {
            text: text_hash,
            text_color: text_color_hash,
            test_alpha_color: test_alpha_color_hash,
            full: full_hash,
            geometry: SectionGeometry::from(section),
        }
    }

    /// They're different, but how?
    fn diff(self, other: SectionHashDetail) -> SectionChange {
        if self.test_alpha_color == other.test_alpha_color {
            return SectionChange::Layout(GlyphChange::Geometry(self.geometry));
        } else if self.geometry == other.geometry {
            // if self.text_color == other.text_color {
            //     return SectionChange::Alpha;
            if self.text == other.text {
                // color and alpha may have changed
                return SectionChange::Color;
            }
        }
        SectionChange::Layout(GlyphChange::Unknown)
    }
}

#[derive(Debug)]
pub(crate) enum SectionChange {
    /// Only the colors have changed (including alpha).
    Color,
    /// A `GlyphChange`.
    Layout(GlyphChange),
}

impl SectionChange {
    #[inline]
    pub(crate) fn recalculate_glyphs<F, FM, P, L>(
        self,
        layout: &L,
        previous: P,
        fonts: &FM,
        geometry: &SectionGeometry,
        sections: &[(SectionText, Color)],
    ) -> Vec<(SectionGlyph, Color)>
    where
        F: Font,
        FM: FontMap<F>,
        P: IntoIterator<Item = (SectionGlyph, Color)>,
        L: GlyphPositioner,
    {
        match self {
            SectionChange::Layout(inner) => layout
                .recalculate_glyphs(
                    previous.into_iter().map(|(sg, _)| sg),
                    inner,
                    fonts,
                    geometry,
                    sections,
                )
                .into_iter()
                .map(|sg| {
                    let color = sections[sg.section_index].1;
                    (sg, color)
                })
                .collect(),
            SectionChange::Color => previous
                .into_iter()
                .map(|(sg, _)| {
                    let new_color = sections[sg.section_index].1;
                    (sg, new_color)
                })
                .collect(),
        }
    }
}

/// Container for positioned glyphs which can generate and cache vertices
struct Glyphed<V> {
    positioned: GlyphedSection,
    vertices: Vec<V>,
}

impl<V> PartialEq for Glyphed<V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.positioned == other.positioned
    }
}

impl<V> Glyphed<V> {
    #[inline]
    fn new(gs: GlyphedSection) -> Self {
        Self {
            positioned: gs,
            vertices: Vec::new(),
        }
    }

    /// Mark previous texture positions as no longer valid (vertices require re-generation)
    fn invalidate_texture_positions(&mut self) {
        self.vertices.clear();
    }

    /// Calculate vertices if not already done
    fn ensure_vertices<F>(&mut self, texture_cache: &DrawCache, to_vertex: F)
    where
        F: Fn(GlyphVertex) -> V,
    {
        if !self.vertices.is_empty() {
            return;
        }

        let GlyphedSection {
            bounds,
            z,
            ref glyphs,
        } = self.positioned;

        self.vertices.reserve(glyphs.len());
        self.vertices
            .extend(glyphs.iter().filter_map(|(sg, color)| {
                match texture_cache.rect_for(sg.font_id.0, &sg.glyph) {
                    // `Err(_)`: no texture for this glyph. This may not be an error as some
                    // glyphs are invisible.
                    Err(_) | Ok(None) => None,
                    Ok(Some((tex_coords, pixel_coords))) => {
                        if pixel_coords.min.x as f32 > bounds.max.x
                            || pixel_coords.min.y as f32 > bounds.max.y
                            || bounds.min.x > pixel_coords.max.x as f32
                            || bounds.min.y > pixel_coords.max.y as f32
                        {
                            // glyph is totally outside the bounds
                            None
                        } else {
                            Some(to_vertex(GlyphVertex {
                                tex_coords,
                                pixel_coords,
                                bounds,
                                color: *color,
                                z,
                            }))
                        }
                    }
                }
            }));
    }
}

#[cfg(test)]
mod hash_diff_test {
    use super::*;
    use matches::assert_matches;

    fn section() -> VariedSection<'static> {
        VariedSection {
            text: vec![
                (
                    SectionText {
                        text: "Hello, ",
                        scale: PxScale::from(20.0),
                        font_id: FontId(0),
                    },
                    [1.0, 0.9, 0.8, 0.7],
                ),
                (
                    SectionText {
                        text: "World",
                        scale: PxScale::from(22.0),
                        font_id: FontId(1),
                    },
                    [0.6, 0.5, 0.4, 0.3],
                ),
            ],
            bounds: (55.5, 66.6),
            z: 0.444,
            layout: Layout::default(),
            screen_position: (999.99, 888.88),
        }
    }

    #[test]
    fn change_screen_position() {
        let build_hasher = DefaultSectionHasher::default();
        let mut section = section();
        let hash_deets = SectionHashDetail::new(&build_hasher, &section, &section.layout);

        section.screen_position.1 += 0.1;

        let diff = hash_deets.diff(SectionHashDetail::new(
            &build_hasher,
            &section,
            &section.layout,
        ));

        match diff {
            SectionChange::Layout(GlyphChange::Geometry(geo)) => {
                assert_eq!(geo, hash_deets.geometry)
            }
            _ => assert_matches!(diff, SectionChange::Layout(GlyphChange::Geometry(..))),
        }
    }

    #[test]
    fn change_color() {
        let build_hasher = DefaultSectionHasher::default();
        let mut section = section();
        let hash_deets = SectionHashDetail::new(&build_hasher, &section, &section.layout);

        section.text[1].1[2] -= 0.1;

        let diff = hash_deets.diff(SectionHashDetail::new(
            &build_hasher,
            &section,
            &section.layout,
        ));

        assert_matches!(diff, SectionChange::Color);
    }

    #[test]
    fn change_color_alpha() {
        let build_hasher = DefaultSectionHasher::default();
        let mut section = section();
        let hash_deets = SectionHashDetail::new(&build_hasher, &section, &section.layout);

        section.text[1].1[2] -= 0.1;
        section.text[0].1[0] -= 0.1;
        section.text[0].1[3] += 0.1; // alpha change too

        let diff = hash_deets.diff(SectionHashDetail::new(
            &build_hasher,
            &section,
            &section.layout,
        ));

        assert_matches!(diff, SectionChange::Color);
    }

    // #[test]
    // fn change_alpha() {
    //     let build_hasher = DefaultSectionHasher::default();
    //     let mut section = section();
    //     let hash_deets = SectionHashDetail::new(&build_hasher, &section, &section.layout);
    //
    //     section.text[1].1[3] -= 0.1;
    //
    //     let diff = hash_deets.diff(SectionHashDetail::new(
    //         &build_hasher,
    //         &section,
    //         &section.layout,
    //     ));
    //
    //     assert_matches!(diff, SectionChange::Alpha);
    // }

    #[test]
    fn change_text() {
        let build_hasher = DefaultSectionHasher::default();
        let mut section = section();
        let hash_deets = SectionHashDetail::new(&build_hasher, &section, &section.layout);

        section.text[1].0.text = "something else";

        let diff = hash_deets.diff(SectionHashDetail::new(
            &build_hasher,
            &section,
            &section.layout,
        ));

        assert_matches!(diff, SectionChange::Layout(GlyphChange::Unknown));
    }
}
