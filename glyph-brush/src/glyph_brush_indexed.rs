mod builder;

pub use self::builder::*;

use super::*;
use full_rusttype::gpu_cache::{Cache, CachedBy};
use log::error;
use std::{borrow::Cow, fmt, i32};

/// Similar to [GlyphBrush] but lets the user control when to cache and un-cache a section.
///
/// Each section is [push_section](GlyphBrushIndexed::push_section)ed and [pop_section](GlyphBrushIndexed::pop_section), alternatively [remove_section](GlyphBrushIndexed::remove_section) is also available to remove
/// sections from the middle of the section list.
pub struct GlyphBrushIndexed<'font, V> {
    fonts: Vec<Font<'font>>,
    texture_cache: Cache<'font>,

    glyph_store: Vec<Glyphed<'font, V>>,

    glyphs_custom_layout: Option<Glyphed<'font, V>>,
}

impl<V> fmt::Debug for GlyphBrushIndexed<'_, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlyphBrushIndexed")
    }
}

impl<'font, V> GlyphCruncher<'font> for GlyphBrushIndexed<'font, V>
where
    V: Clone + 'static,
{
    fn pixel_bounds_custom_layout<'a, S, L>(&mut self, section: S, layout: &L) -> Option<Rect<i32>>
    where
        L: GlyphPositioner,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();

        match section {
            Cow::Borrowed(section) => {
                let geometry = SectionGeometry::from(section);

                Glyphed::<'font, V>::new(GlyphedSection {
                    bounds: layout.bounds_rect(&geometry),
                    glyphs: layout.calculate_glyphs(&self.fonts, &geometry, &section.text),
                    z: section.z,
                })
                .positioned
                .pixel_bounds()
            }
            Cow::Owned(section) => {
                let geometry = SectionGeometry::from(&section);

                Glyphed::<'font, V>::new(GlyphedSection {
                    bounds: layout.bounds_rect(&geometry),
                    glyphs: layout.calculate_glyphs(&self.fonts, &geometry, &section.text),
                    z: section.z,
                })
                .positioned
                .pixel_bounds()
            }
        }
    }

    fn glyphs_custom_layout<'a, 'b, S, L>(
        &'b mut self,
        section: S,
        layout: &L,
    ) -> PositionedGlyphIter<'b, 'font>
    where
        L: GlyphPositioner,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();

        match section {
            Cow::Borrowed(section) => {
                let geometry = SectionGeometry::from(section);

                self.glyphs_custom_layout = Some(Glyphed::<'font, V>::new(GlyphedSection {
                    bounds: layout.bounds_rect(&geometry),
                    glyphs: layout.calculate_glyphs(&self.fonts, &geometry, &section.text),
                    z: section.z,
                }));
            }
            Cow::Owned(section) => {
                let geometry = SectionGeometry::from(&section);

                self.glyphs_custom_layout = Some(Glyphed::<'font, V>::new(GlyphedSection {
                    bounds: layout.bounds_rect(&geometry),
                    glyphs: layout.calculate_glyphs(&self.fonts, &geometry, &section.text),
                    z: section.z,
                }));
            }
        }

        self.glyphs_custom_layout
            .as_ref()
            .unwrap()
            .positioned
            .glyphs()
    }

    fn fonts(&self) -> &[Font<'font>] {
        &self.fonts
    }
}

impl<'font, V> GlyphBrushIndexed<'font, V>
where
    V: Clone + 'static,
{
    /// Add a section to this brush with a custom layout.
    pub fn push_section_custom_layout<'a, S, G>(&mut self, section: S, custom_layout: &G)
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
        self.cache_glyphs(&section, custom_layout);
    }

    /// Add a section to this brush.
    ///
    /// ```no_run
    /// # use glyph_brush::*;
    /// # let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// # let mut glyph_brush: GlyphBrushIndexed<'_, ()> = GlyphBrushIndexedBuilder::using_font_bytes(dejavu).build();
    /// glyph_brush.push_section(Section {
    ///     text: "Hello glyph_brush",
    ///     ..Section::default()
    /// });
    /// ```
    pub fn push_section<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let layout = section.layout;
        self.push_section_custom_layout(section, &layout)
    }

    /// Adds a section to the glyph list.
    fn cache_glyphs<L>(&mut self, section: &VariedSection<'_>, layout: &L)
    where
        L: GlyphPositioner,
    {
        let geometry = SectionGeometry::from(section);

        self.glyph_store.push(Glyphed::new(GlyphedSection {
            bounds: layout.bounds_rect(&geometry),
            glyphs: layout.calculate_glyphs(&self.fonts, &geometry, &section.text),
            z: section.z,
        }));
    }

    /// Process all sections, returning the texel changes as well as any indices which need to
    /// re-evaluate their their texture coordinates.
    pub fn process_sections<F1, F2>(
        &mut self,
        update_texture: F1,
        to_vertex: F2,
    ) -> Result<BrushActionIndexed<V>, BrushError>
    where
        F1: FnMut(Rect<u32>, &[u8]),
        F2: Fn(GlyphVertex) -> V + Copy,
    {
        for glyphs in &self.glyph_store {
            for (glyph, _, font) in &glyphs.positioned.glyphs {
                self.texture_cache.queue_glyph(font.0, glyph.clone());
            }
        }

        match self.texture_cache.cache_queued(update_texture) {
            Ok(CachedBy::Adding) => {}
            Ok(CachedBy::Reordering) => {
                for glyphed in &mut self.glyph_store {
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

        Ok(BrushActionIndexed::Draw({
            let mut verts = Vec::new();

            for (idx, glyphed) in self.glyph_store.iter_mut().enumerate() {
                if glyphed.needs_recalculation() {
                    glyphed.ensure_vertices(&self.texture_cache, to_vertex);
                    verts.push((idx, glyphed.vertices.to_vec()));
                }
            }

            verts
        }))
    }

    /// Remove a section given by some index. Note that this shifts the indices of all subsequent
    /// sections down by one.
    pub fn remove_section(&mut self, index: usize) {
        self.glyph_store.remove(index);
    }

    /// Pop the last pushed section.
    pub fn pop_section(&mut self) {
        self.glyph_store.pop();
    }

    /// Rebuilds the logical texture cache with new dimensions. Should be avoided if possible.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use glyph_brush::*;
    /// # let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// # let mut glyph_brush: GlyphBrushIndexed<'_, ()> = GlyphBrushIndexedBuilder::using_font_bytes(dejavu).build();
    /// glyph_brush.resize_texture(512, 512);
    /// ```
    pub fn resize_texture(&mut self, new_width: u32, new_height: u32) {
        self.texture_cache
            .to_builder()
            .dimensions(new_width, new_height)
            .rebuild(&mut self.texture_cache);

        // invalidate any previous cache position data
        for glyphed in &mut self.glyph_store {
            glyphed.invalidate_texture_positions();
        }
    }

    /// Returns the logical texture cache pixel dimensions `(width, height)`.
    pub fn texture_dimensions(&self) -> (u32, u32) {
        self.texture_cache.dimensions()
    }

    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    ///
    /// # Example
    ///
    /// ```
    /// use glyph_brush::{GlyphBrushIndexed, GlyphBrushIndexedBuilder, Section};
    /// # type Vertex = ();
    ///
    /// // dejavu is built as default `FontId(0)`
    /// let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// let mut glyph_brush: GlyphBrushIndexed<'_, Vertex> =
    ///     GlyphBrushIndexedBuilder::using_font_bytes(dejavu).build();
    ///
    /// // some time later, add another font referenced by a new `FontId`
    /// let open_sans_italic: &[u8] = include_bytes!("../../fonts/OpenSans-Italic.ttf");
    /// let open_sans_italic_id = glyph_brush.add_font_bytes(open_sans_italic);
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

impl<'font, V> GlyphBrushIndexed<'font, V> {
    /// Return a [`GlyphBrushIndexedBuilder`](struct.GlyphBrushIndexedBuilder.html) prefilled with the
    /// properties of this `GlyphBrushIndexed`.
    ///
    /// # Example
    ///
    /// ```
    /// # use glyph_brush::{*, rusttype::*};
    /// # type Vertex = ();
    /// # let sans = Font::from_bytes(&include_bytes!("../../fonts/DejaVuSans.ttf")[..]).unwrap();
    /// let glyph_brush: GlyphBrushIndexed<'_, Vertex> = GlyphBrushIndexedBuilder::using_font(sans)
    ///     .initial_cache_size((128, 128))
    ///     .build();
    ///
    /// let new_brush: GlyphBrushIndexed<'_, Vertex> = glyph_brush.to_builder().build();
    /// assert_eq!(new_brush.texture_dimensions(), (128, 128));
    /// ```
    pub fn to_builder(&self) -> GlyphBrushIndexedBuilder<'font> {
        let mut builder = GlyphBrushIndexedBuilder::using_fonts(self.fonts.clone());
        builder.gpu_cache_builder = self.texture_cache.to_builder();
        builder
    }
}

/// Data used to generate vertex information for a single glyph
#[derive(Debug)]
pub struct GlyphVertex {
    pub tex_coords: Rect<f32>,
    pub pixel_coords: Rect<i32>,
    pub bounds: Rect<f32>,
    pub color: Color,
    pub z: f32,
}

/// Actions that should be taken after processing push data
#[derive(Debug)]
pub enum BrushActionIndexed<V> {
    /// Draw new/changed vertex data.
    Draw(Vec<(usize, Vec<V>)>),
}

/// Container for positioned glyphs which can generate and cache vertices
struct Glyphed<'font, V> {
    positioned: GlyphedSection<'font>,
    vertices: Vec<V>,
}

impl<V> PartialEq for Glyphed<'_, V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.positioned == other.positioned
    }
}

impl<'font, V> Glyphed<'font, V> {
    #[inline]
    fn new(gs: GlyphedSection<'font>) -> Self {
        Self {
            positioned: gs,
            vertices: Vec::new(),
        }
    }

    /// Mark previous texture positions as no longer valid (vertices require re-generation)
    fn invalidate_texture_positions(&mut self) {
        self.vertices.clear();
    }

    /// Check if this glyph collection needs re-computation.
    fn needs_recalculation(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Calculate vertices if not already done
    fn ensure_vertices<F>(&mut self, texture_cache: &Cache<'font>, to_vertex: F)
    where
        F: Fn(GlyphVertex) -> V,
    {
        let GlyphedSection {
            bounds,
            z,
            ref glyphs,
        } = self.positioned;

        self.vertices.reserve(glyphs.len());
        self.vertices
            .extend(glyphs.iter().filter_map(|(glyph, color, font_id)| {
                match texture_cache.rect_for(font_id.0, glyph) {
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
                                color: *color,
                                z,
                            }))
                        }
                    }
                }
            }));
    }
}
