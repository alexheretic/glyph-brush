use crate::{Font, FontId, GlyphBrushIndexed, SharedBytes};
use full_rusttype::gpu_cache::{Cache, CacheBuilder};

/// Builder for a [`GlyphBrushIndexed`](struct.GlyphBrushIndexed.html).
///
/// # Example
///
/// ```
/// use glyph_brush::{GlyphBrushIndexed, GlyphBrushIndexedBuilder};
/// # type Vertex = ();
///
/// let dejavu: &[u8] = include_bytes!("../../../fonts/DejaVuSans.ttf");
/// let mut glyph_brush: GlyphBrushIndexed<'_, Vertex> =
///     GlyphBrushIndexedBuilder::using_font_bytes(dejavu).build();
/// ```
pub struct GlyphBrushIndexedBuilder<'a> {
    pub font_data: Vec<Font<'a>>,
    pub gpu_cache_builder: CacheBuilder,
    _private_construction: (),
}

impl<'a> GlyphBrushIndexedBuilder<'a> {
    /// Create a new builder with a single font's data that will be used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font_bytes<B: Into<SharedBytes<'a>>>(font_0_data: B) -> Self {
        Self::using_font(Font::from_bytes(font_0_data).unwrap())
    }

    /// Create a new builder with multiple fonts' data.
    pub fn using_fonts_bytes<B, V>(font_data: V) -> Self
    where
        B: Into<SharedBytes<'a>>,
        V: IntoIterator<Item = B>,
    {
        Self::using_fonts(
            font_data
                .into_iter()
                .map(|data| Font::from_bytes(data).unwrap())
                .collect::<Vec<_>>(),
        )
    }

    /// Create a new builder with a single font that will be used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font(font_0: Font<'a>) -> Self {
        Self::using_fonts(vec![font_0])
    }

    /// Create a new builder with multiple fonts.
    pub fn using_fonts<V: Into<Vec<Font<'a>>>>(fonts: V) -> Self {
        let mut builder = Self::without_fonts();
        builder.font_data = fonts.into();
        builder
    }

    /// Create a new builder without any fonts.
    ///
    /// **Warning:** A [`GlyphBrushIndexed`] built without fonts will panic if you try to use it as it
    /// will have no default `FontId(0)` to use.
    /// Use [`GlyphBrushIndexed.add_font`] before queueing any text sections in order to avoid panicking.
    ///
    /// [`GlyphBrushIndexed`]: struct.GlyphBrushIndexed.html
    /// [`GlyphBrushIndexed.add_font`]: ../glyph_brush/struct.GlyphBrushIndexed.html#method.add_font
    pub fn without_fonts() -> Self {
        GlyphBrushIndexedBuilder {
            font_data: Vec::new(),
            gpu_cache_builder: Cache::builder()
                .dimensions(256, 256)
                .scale_tolerance(0.5)
                .position_tolerance(0.1)
                .align_4x4(false),
            _private_construction: (),
        }
    }
}

impl<'a> GlyphBrushIndexedBuilder<'a> {
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

    /// Consume all builder fonts a replace with new fonts returned by the input function.
    ///
    /// Generally only makes sense when wanting to change fonts after calling
    /// [`GlyphBrushIndexed::to_builder`](struct.GlyphBrushIndexed.html#method.to_builder).
    ///
    /// # Example
    /// ```
    /// # use glyph_brush::{*, rusttype::*};
    /// # type Vertex = ();
    /// # let open_sans = Font::from_bytes(&include_bytes!("../../../fonts/DejaVuSans.ttf")[..]).unwrap();
    /// # let deja_vu_sans = open_sans.clone();
    /// let two_font_brush: GlyphBrushIndexed<'_, Vertex>
    ///     = GlyphBrushIndexedBuilder::using_fonts(vec![open_sans, deja_vu_sans]).build();
    ///
    /// let one_font_brush: GlyphBrushIndexed<'_, Vertex> = two_font_brush
    ///     .to_builder()
    ///     .replace_fonts(|mut fonts| {
    ///         // remove open_sans, leaving just deja_vu as FontId(0)
    ///         fonts.remove(0);
    ///         fonts
    ///     })
    ///     .build();
    ///
    /// assert_eq!(one_font_brush.fonts().len(), 1);
    /// assert_eq!(two_font_brush.fonts().len(), 2);
    /// ```
    pub fn replace_fonts<V, F>(mut self, font_fn: F) -> Self
    where
        V: Into<Vec<Font<'a>>>,
        F: FnOnce(Vec<Font<'a>>) -> V,
    {
        self.font_data = font_fn(self.font_data).into();
        self
    }

    /// Initial size of 2D texture used as a gpu cache, pixels (width, height).
    /// The GPU cache will dynamically quadruple in size whenever the current size
    /// is insufficient.
    ///
    /// Defaults to `(256, 256)`
    pub fn initial_cache_size(mut self, (w, h): (u32, u32)) -> Self {
        self.gpu_cache_builder = self.gpu_cache_builder.dimensions(w, h);
        self
    }

    /// Sets the maximum allowed difference in scale used for judging whether to reuse an
    /// existing glyph in the GPU cache.
    ///
    /// Defaults to `0.5`
    ///
    /// See rusttype docs for `rusttype::gpu_cache::Cache`
    pub fn gpu_cache_scale_tolerance(mut self, tolerance: f32) -> Self {
        self.gpu_cache_builder = self.gpu_cache_builder.scale_tolerance(tolerance);
        self
    }

    /// Sets the maximum allowed difference in subpixel position used for judging whether
    /// to reuse an existing glyph in the GPU cache. Anything greater than or equal to
    /// 1.0 means "don't care".
    ///
    /// Defaults to `0.1`
    ///
    /// See rusttype docs for `rusttype::gpu_cache::Cache`
    pub fn gpu_cache_position_tolerance(mut self, tolerance: f32) -> Self {
        self.gpu_cache_builder = self.gpu_cache_builder.position_tolerance(tolerance);
        self
    }

    /// Align glyphs in texture cache to 4x4 texel boundaries.
    ///
    /// If your backend requires texture updates to be aligned to 4x4 texel
    /// boundaries (e.g. WebGL), this should be set to `true`.
    ///
    /// Defaults to `false`
    ///
    /// See rusttype docs for `rusttype::gpu_cache::Cache`
    pub fn gpu_cache_align_4x4(mut self, align: bool) -> Self {
        self.gpu_cache_builder = self.gpu_cache_builder.align_4x4(align);
        self
    }

    /// Builds a `GlyphBrushIndexed` using the input gfx factory
    pub fn build<V>(self) -> GlyphBrushIndexed<'a, V> {
        GlyphBrushIndexed {
            fonts: self.font_data,
            texture_cache: self.gpu_cache_builder.build(),

            glyph_store: <_>::default(),

            glyphs_custom_layout: None,
        }
    }

    /// Rebuilds an existing `GlyphBrushIndexed` with this builder's properties. This will clear all
    /// caches and queues.
    ///
    /// # Example
    /// ```
    /// # use glyph_brush::{*, rusttype::*};
    /// # let sans = Font::from_bytes(&include_bytes!("../../../fonts/DejaVuSans.ttf")[..]).unwrap();
    /// # type Vertex = ();
    /// let mut glyph_brush: GlyphBrushIndexed<'_, Vertex> = GlyphBrushIndexedBuilder::using_font(sans).build();
    /// assert_eq!(glyph_brush.texture_dimensions(), (256, 256));
    ///
    /// // Use a new builder to rebuild the brush with a smaller initial cache size
    /// glyph_brush.to_builder().initial_cache_size((64, 64)).rebuild(&mut glyph_brush);
    /// assert_eq!(glyph_brush.texture_dimensions(), (64, 64));
    /// ```
    pub fn rebuild<V>(self, brush: &mut GlyphBrushIndexed<'a, V>) {
        std::mem::replace(brush, self.build());
    }
}
