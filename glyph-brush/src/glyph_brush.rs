mod builder;

pub use self::builder::*;

use super::*;
use full_rusttype::gpu_cache::Cache;
use log::error;
use std::{
    borrow::Cow,
    fmt,
    hash::{BuildHasher, BuildHasherDefault, Hash, Hasher},
    i32, mem,
};

/// A hash of `Section` data
type SectionHash = u64;

/// A "practically collision free" `Section` hasher
type DefaultSectionHasher = BuildHasherDefault<seahash::SeaHasher>;

/// Object allowing glyph drawing, containing cache state. Manages glyph positioning cacheing,
/// glyph draw caching & efficient GPU texture cache updating.
///
/// Build using a [`GlyphBrushBuilder`](struct.GlyphBrushBuilder.html).
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
pub struct GlyphBrush<'font, H = DefaultSectionHasher> {
    fonts: Vec<Font<'font>>,
    texture_cache: Cache<'font>,
    last_draw: LastDrawInfo,

    // cache of section-layout hash -> computed glyphs, this avoid repeated glyph computation
    // for identical layout/sections common to repeated frame rendering
    calculate_glyph_cache: hashbrown::HashMap<SectionHash, GlyphedSection<'font>>,

    last_frame_seq_id_sections: Vec<SectionHashDetail>,
    frame_seq_id_sections: Vec<SectionHashDetail>,

    // buffer of section-layout hashs (that must exist in the calculate_glyph_cache)
    // to be used on the next `process_queued` call
    section_buffer: Vec<SectionHash>,

    // Set of section hashs to keep in the glyph cache this frame even if they haven't been drawn
    keep_in_cache: hashbrown::HashSet<SectionHash>,

    // config
    cache_glyph_positioning: bool,
    cache_glyph_drawing: bool,

    section_hasher: H,

    last_pre_positioned: Vec<GlyphedSection<'font>>,
    pre_positioned: Vec<GlyphedSection<'font>>,
}

impl<H> fmt::Debug for GlyphBrush<'_, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlyphBrush")
    }
}

impl<'font, H: BuildHasher> GlyphCruncher<'font> for GlyphBrush<'font, H> {
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
        self.keep_in_cache.insert(section_hash);
        self.calculate_glyph_cache[&section_hash].pixel_bounds()
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
        self.keep_in_cache.insert(section_hash);
        self.calculate_glyph_cache[&section_hash].glyphs()
    }

    fn fonts(&self) -> &[Font<'font>] {
        &self.fonts
    }
}

impl<'font, H: BuildHasher> GlyphBrush<'font, H> {
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
            for text in &section.text {
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
    /// # let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build();
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
        glyphs: Vec<(PositionedGlyph<'font>, Color, FontId)>,
        bounds: Rect<f32>,
        z: f32,
    ) {
        self.pre_positioned
            .push(GlyphedSection { glyphs, bounds, z });
    }

    /// Returns the calculate_glyph_cache key for this sections glyphs
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
                        let change = match section_hash.diff(hash) {
                            SectionHashDiff::GeometryChange => GlyphChange::Geometry(hash.geometry),
                            SectionHashDiff::ColorChange => GlyphChange::Color,
                            SectionHashDiff::AlphaChange => GlyphChange::Alpha,
                            SectionHashDiff::Different => return None,
                        };

                        let glyphs = self.calculate_glyph_cache.remove(&hash.full)?.glyphs;

                        Some(layout.recalculate_glyphs(
                            Cow::Owned(glyphs),
                            change,
                            &self.fonts,
                            &geometry,
                            &section.text,
                        ))
                    });

                self.calculate_glyph_cache.insert(
                    section_hash.full,
                    GlyphedSection {
                        bounds: layout.bounds_rect(&geometry),
                        glyphs: recalculated_glyphs.unwrap_or_else(|| {
                            layout.calculate_glyphs(&self.fonts, &geometry, &section.text)
                        }),
                        z: section.z,
                    },
                );
            }
        } else {
            let geometry = SectionGeometry::from(section);
            self.calculate_glyph_cache.insert(
                section_hash.full,
                GlyphedSection {
                    bounds: layout.bounds_rect(&geometry),
                    glyphs: layout.calculate_glyphs(&self.fonts, &geometry, &section.text),
                    z: section.z,
                },
            );
        }
        section_hash.full
    }

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
    /// # let update_texture = |_, _| {};
    /// # let into_vertex = |_| ();
    /// glyph_brush.process_queued(
    ///     (1024, 768),
    ///     |rect, tex_data| update_texture(rect, tex_data),
    ///     |vertex_data| into_vertex(vertex_data),
    /// )?
    /// # ;
    /// # Ok(())
    /// # }
    /// ```
    pub fn process_queued<V, F1, F2>(
        &mut self,
        (screen_w, screen_h): (u32, u32),
        update_texture: F1,
        to_vertex: F2,
    ) -> Result<BrushAction<V>, BrushError>
    where
        F1: FnMut(Rect<u32>, &[u8]),
        F2: Fn(GlyphVertex) -> V,
    {
        let current_text_state = {
            let mut s = self.section_hasher.build_hasher();
            self.section_buffer.hash(&mut s);
            screen_w.hash(&mut s);
            screen_h.hash(&mut s);
            s.finish()
        };

        let result = if !self.cache_glyph_drawing
            || self.last_draw.text_state != current_text_state
            || self.last_pre_positioned != self.pre_positioned
        {
            let mut some_text = false;
            // Everything in the section_buffer should also be here. The extras should also
            // be retained in the texture cache avoiding cache thrashing if they are rendered
            // in a 2-draw per frame style.
            for section_hash in &self.keep_in_cache {
                for &(ref glyph, _, font_id) in self
                    .calculate_glyph_cache
                    .get(section_hash)
                    .iter()
                    .flat_map(|gs| &gs.glyphs)
                {
                    self.texture_cache.queue_glyph(font_id.0, glyph.clone());
                    some_text = true;
                }
            }

            for &(ref glyph, _, font_id) in self.pre_positioned.iter().flat_map(|p| &p.glyphs) {
                self.texture_cache.queue_glyph(font_id.0, glyph.clone());
                some_text = true;
            }

            if some_text && self.texture_cache.cache_queued(update_texture).is_err() {
                let (width, height) = self.texture_cache.dimensions();
                return Err(BrushError::TextureTooSmall {
                    suggested: (width * 2, height * 2),
                });
            }

            let verts: Vec<V> = if some_text {
                let sections: Vec<_> = self
                    .section_buffer
                    .iter()
                    .map(|hash| &self.calculate_glyph_cache[hash])
                    .chain(self.pre_positioned.iter())
                    .collect();

                let mut verts = Vec::with_capacity(
                    sections
                        .iter()
                        .map(|section| section.glyphs.len())
                        .sum::<usize>(),
                );

                for &GlyphedSection {
                    ref glyphs,
                    bounds,
                    z,
                    ..
                } in sections
                {
                    verts.extend(glyphs.iter().filter_map(|(glyph, color, font_id)| {
                        match self.texture_cache.rect_for(font_id.0, glyph) {
                            Err(err) => {
                                error!("Cache miss?: {:?}, {:?}: {}", font_id, glyph, err);
                                None
                            }
                            Ok(None) => None,
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
                                        screen_dimensions: (screen_w as f32, screen_h as f32),
                                        color: *color,
                                        z,
                                    }))
                                }
                            }
                        }
                    }));
                }

                verts
            } else {
                vec![]
            };

            self.last_draw.text_state = current_text_state;
            BrushAction::Draw(verts)
        } else {
            BrushAction::ReDraw
        };

        self.cleanup_frame();
        Ok(result)
    }

    /// Rebuilds the logical texture cache with new dimensions. Should be avoided if possible.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use glyph_brush::GlyphBrushBuilder;
    /// # let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// # let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build();
    /// glyph_brush.resize_texture(512, 512);
    /// ```
    pub fn resize_texture(&mut self, new_width: u32, new_height: u32) {
        self.texture_cache
            .to_builder()
            .dimensions(new_width, new_height)
            .rebuild(&mut self.texture_cache);

        self.last_draw = LastDrawInfo::default();
    }

    /// Returns the logical texture cache pixel dimensions `(width, height)`.
    pub fn texture_dimensions(&self) -> (u32, u32) {
        self.texture_cache.dimensions()
    }

    fn cleanup_frame(&mut self) {
        if self.cache_glyph_positioning {
            // clear section_buffer & trim calculate_glyph_cache to active sections
            let active = mem::replace(&mut self.keep_in_cache, <_>::default());
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

    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use glyph_brush::{GlyphBrushBuilder, Section};
    /// # fn main() {
    ///
    /// // dejavu is built as default `FontId(0)`
    /// let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build();
    ///
    /// // some time later, add another font referenced by a new `FontId`
    /// let open_sans_italic: &[u8] = include_bytes!("../../fonts/OpenSans-Italic.ttf");
    /// let open_sans_italic_id = glyph_brush.add_font_bytes(open_sans_italic);
    /// # }
    /// ```
    pub fn add_font_bytes<'a: 'font, B: Into<SharedBytes<'a>>>(&mut self, font_data: B) -> FontId {
        self.add_font(Font::from_bytes(font_data.into()).unwrap())
    }

    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font<'a: 'font>(&mut self, font_data: Font<'a>) -> FontId {
        self.fonts.push(font_data);
        FontId(self.fonts.len() - 1)
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
                assert!(self.fonts.len() > text.font_id.0, "Invalid font id");
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

#[derive(Debug, Default)]
struct LastDrawInfo {
    text_state: u64,
}

// glyph: &PositionedGlyph,
// color: Color,
// font_id: FontId,
// cache: &Cache,
// bounds: Rect<f32>,
// z: f32,
// (screen_width, screen_height): (f32, f32),

/// Data used to generate vertex information for a single glyph
#[derive(Debug)]
pub struct GlyphVertex {
    pub tex_coords: Rect<f32>,
    pub pixel_coords: Rect<i32>,
    pub bounds: Rect<f32>,
    pub screen_dimensions: (f32, f32),
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
        write!(f, "{}", std::error::Error::description(self))
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
    /// hash of text
    text_no_color_alpha: SectionHash,
    // hash of text & alpha
    text_no_color: SectionHash,
    /// hash of text & colors including alpha
    text: SectionHash,
    /// hash of everything
    full: SectionHash,

    /// copy of geometry for later comparison
    geometry: SectionGeometry,
}

#[derive(Debug)]
enum SectionHashDiff {
    GeometryChange,
    ColorChange,
    AlphaChange,
    Different,
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
        let text_no_color_alpha_hash = s.finish();

        parts.hash_alpha(&mut s);
        let text_no_color_hash = s.finish();

        parts.hash_color(&mut s);
        let text_hash = s.finish();

        parts.hash_geometry(&mut s);
        parts.hash_z(&mut s);
        let full_hash = s.finish();

        Self {
            text_no_color_alpha: text_no_color_alpha_hash,
            text_no_color: text_no_color_hash,
            text: text_hash,
            // text_geometry: text_geo_hash,
            full: full_hash,
            geometry: SectionGeometry::from(section),
        }
    }

    /// They're different, but how?
    fn diff(self, other: SectionHashDetail) -> SectionHashDiff {
        if self.text == other.text {
            return SectionHashDiff::GeometryChange;
        } else if self.geometry == other.geometry {
            if self.text_no_color == other.text_no_color {
                return SectionHashDiff::ColorChange;
            } else if self.text_no_color_alpha == other.text_no_color_alpha {
                return SectionHashDiff::AlphaChange;
            }
        }
        SectionHashDiff::Different
    }
}
