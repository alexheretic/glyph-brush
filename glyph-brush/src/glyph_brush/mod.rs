mod builder;

pub use self::builder::*;

use super::*;
use full_rusttype::gpu_cache::Cache;
use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use std::{fmt, i32};

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
/// [`GlyphBrush::draw_queued`](#method.draw_queued) call when that section has not been used since
/// the previous draw call.
pub struct GlyphBrush<'font, H = DefaultSectionHasher> {
    fonts: Vec<Font<'font>>,
    texture_cache: Cache<'font>,
    draw_cache: Option<DrawnGlyphBrush>,

    // cache of section-layout hash -> computed glyphs, this avoid repeated glyph computation
    // for identical layout/sections common to repeated frame rendering
    calculate_glyph_cache: FxHashMap<SectionHash, GlyphedSection<'font>>,

    // buffer of section-layout hashs (that must exist in the calculate_glyph_cache)
    // to be rendered on the next `draw_queued` call
    section_buffer: Vec<SectionHash>,

    // Set of section hashs to keep in the glyph cache this frame even if they haven't been drawn
    keep_in_cache: FxHashSet<SectionHash>,

    // config
    cache_glyph_positioning: bool,
    cache_glyph_drawing: bool,

    section_hasher: H,
}

impl<'font, H> fmt::Debug for GlyphBrush<'font, H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    }

    /// Queues a section/layout to be processed by the next call of
    /// [`process_queued`](struct.GlyphBrush.html#method.process_queued). Can be called multiple
    /// times to queue multiple sections for drawing.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    pub fn queue<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let layout = section.layout;
        self.queue_custom_layout(section, &layout)
    }

    fn hash<T: Hash>(&self, hashable: &T) -> SectionHash {
        let mut s = self.section_hasher.build_hasher();
        hashable.hash(&mut s);
        s.finish()
    }

    /// Returns the calculate_glyph_cache key for this sections glyphs
    fn cache_glyphs<L>(&mut self, section: &VariedSection, layout: &L) -> SectionHash
    where
        L: GlyphPositioner,
    {
        let section_hash = self.hash(&(section, layout));

        if self.cache_glyph_positioning {
            if let Entry::Vacant(entry) = self.calculate_glyph_cache.entry(section_hash) {
                let geometry = SectionGeometry::from(section);
                entry.insert(GlyphedSection {
                    bounds: layout.bounds_rect(&geometry),
                    glyphs: layout.calculate_glyphs(&self.fonts, &geometry, &section.text),
                    z: section.z,
                });
            }
        } else {
            let geometry = SectionGeometry::from(section);
            self.calculate_glyph_cache.insert(
                section_hash,
                GlyphedSection {
                    bounds: layout.bounds_rect(&geometry),
                    glyphs: layout.calculate_glyphs(&self.fonts, &geometry, &section.text),
                    z: section.z,
                },
            );
        }
        section_hash
    }

    /// Processes all queued sections, calling texture update logic when necessary &
    /// returning a `BrushAction`.
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// ```no_run
    /// # extern crate glyph_brush;
    /// # use glyph_brush::*;
    /// # fn main() -> Result<(), BrushError> {
    /// # let dejavu: &[u8] = include_bytes!("../../../fonts/DejaVuSans.ttf");
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
        let current_text_state = self.hash(&(&self.section_buffer, screen_w, screen_h));

        let result = if !self.cache_glyph_drawing
            || self.draw_cache.is_none()
            || self.draw_cache.as_ref().unwrap().last_text_state != current_text_state
        {
            let mut no_text = true;

            for section_hash in &self.section_buffer {
                let GlyphedSection { ref glyphs, .. } = self.calculate_glyph_cache[section_hash];
                for &(ref glyph, _, font_id) in glyphs {
                    self.texture_cache.queue_glyph(font_id.0, glyph.clone());
                    no_text = false;
                }
            }

            if no_text {
                self.clear_section_buffer();
                return Ok(BrushAction::Draw(vec![]));
            }

            if self.texture_cache.cache_queued(update_texture).is_err() {
                let (width, height) = self.texture_cache.dimensions();
                return Err(BrushError::TextureTooSmall {
                    current: (width, height),
                    suggested: (width * 2, height * 2),
                });
            }

            let verts: Vec<V> = {
                let sections: Vec<_> = self
                    .section_buffer
                    .iter()
                    .map(|hash| &self.calculate_glyph_cache[hash])
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
                } in sections
                {
                    verts.extend(glyphs.into_iter().filter_map(|(glyph, color, font_id)| {
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
            };

            self.draw_cache = Some(
                self.draw_cache
                    .take()
                    .map(|mut cache| {
                        cache.last_text_state = current_text_state;
                        cache
                    }).unwrap_or_default(),
            );

            BrushAction::Draw(verts)
        } else {
            BrushAction::ReDraw
        };

        self.clear_section_buffer();
        Ok(result)
    }

    /// Rebuilds the texture cache with new dimensions. Should be avoided if possible.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use glyph_brush::GlyphBrushBuilder;
    /// # let dejavu: &[u8] = include_bytes!("../../../fonts/DejaVuSans.ttf");
    /// # let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build();
    /// glyph_brush.resize_texture(512, 512);
    /// ```
    pub fn resize_texture(&mut self, new_width: u32, new_height: u32) {
        self.texture_cache
            .to_builder()
            .dimensions(new_width, new_height)
            .rebuild(&mut self.texture_cache);

        self.draw_cache.take();
    }

    /// Returns the available fonts.
    pub fn fonts(&self) -> &[Font<'font>] {
        &self.fonts
    }

    fn clear_section_buffer(&mut self) {
        if self.cache_glyph_positioning {
            // clear section_buffer & trim calculate_glyph_cache to active sections
            let mut active =
                HashSet::with_capacity(self.section_buffer.len() + self.keep_in_cache.len());

            for h in self.section_buffer.drain(..) {
                active.insert(h);
            }
            for h in self.keep_in_cache.drain() {
                active.insert(h);
            }
            self.calculate_glyph_cache
                .retain(|key, _| active.contains(key));
        } else {
            self.section_buffer.clear();
            self.calculate_glyph_cache.clear();
            self.keep_in_cache.clear();
        }
    }

    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    ///
    /// # Example
    ///
    /// ```no_run
    /// extern crate glyph_brush;
    /// use glyph_brush::{GlyphBrushBuilder, Section};
    /// # fn main() {
    ///
    /// // dejavu is built as default `FontId(0)`
    /// let dejavu: &[u8] = include_bytes!("../../../fonts/DejaVuSans.ttf");
    /// let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build();
    ///
    /// // some time later, add another font referenced by a new `FontId`
    /// let open_sans_italic: &[u8] = include_bytes!("../../../fonts/OpenSans-Italic.ttf");
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
}

#[derive(Debug, Default)]
struct DrawnGlyphBrush {
    last_text_state: u64,
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
pub enum BrushAction<V> {
    /// Draw new/changed vertix data.
    Draw(Vec<V>),
    /// Re-draw last frame's vertices unmodified.
    ReDraw,
}

#[derive(Debug)]
pub enum BrushError {
    /// Texture is too small to cache queued glyphs
    ///
    /// A larger suggested size is included.
    TextureTooSmall {
        current: (u32, u32),
        suggested: (u32, u32),
    },
}
impl fmt::Display for BrushError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
