use crate::{DefaultSectionHasher, Font, FontId, GlyphBrush, SharedBytes};
use full_rusttype::gpu_cache::{Cache, CacheBuilder};
use std::hash::BuildHasher;

/// Builder for a [`GlyphBrush`](struct.GlyphBrush.html).
///
/// # Example
///
/// ```
/// use glyph_brush::{GlyphBrush, GlyphBrushBuilder};
/// # type Vertex = ();
///
/// let dejavu: &[u8] = include_bytes!("../../../fonts/DejaVuSans.ttf");
/// let mut glyph_brush: GlyphBrush<'_, Vertex> =
///     GlyphBrushBuilder::using_font_bytes(dejavu).build();
/// ```
pub struct GlyphBrushBuilder<'a, H = DefaultSectionHasher> {
    pub font_data: Vec<Font<'a>>,
    pub cache_glyph_positioning: bool,
    pub cache_glyph_drawing: bool,
    pub section_hasher: H,
    pub gpu_cache_builder: CacheBuilder,
    _private_construction: (),
}

impl<'a> GlyphBrushBuilder<'a> {
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
    /// **Warning:** A [`GlyphBrush`] built without fonts will panic if you try to use it as it
    /// will have no default `FontId(0)` to use.
    /// Use [`GlyphBrush.add_font`] before queueing any text sections in order to avoid panicking.
    ///
    /// [`GlyphBrush`]: struct.GlyphBrush.html
    /// [`GlyphBrush.add_font`]: ../glyph_brush/struct.GlyphBrush.html#method.add_font
    pub fn without_fonts() -> Self {
        GlyphBrushBuilder {
            font_data: Vec::new(),
            cache_glyph_positioning: true,
            cache_glyph_drawing: true,
            section_hasher: DefaultSectionHasher::default(),
            gpu_cache_builder: Cache::builder()
                .dimensions(256, 256)
                .scale_tolerance(0.5)
                .position_tolerance(0.1)
                .align_4x4(false),
            _private_construction: (),
        }
    }
}

impl<'a, H: BuildHasher> GlyphBrushBuilder<'a, H> {
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
    /// [`GlyphBrush::to_builder`](struct.GlyphBrush.html#method.to_builder).
    ///
    /// # Example
    /// ```
    /// # use glyph_brush::{*, rusttype::*};
    /// # type Vertex = ();
    /// # let open_sans = Font::from_bytes(&include_bytes!("../../../fonts/DejaVuSans.ttf")[..]).unwrap();
    /// # let deja_vu_sans = open_sans.clone();
    /// let two_font_brush: GlyphBrush<'_, Vertex>
    ///     = GlyphBrushBuilder::using_fonts(vec![open_sans, deja_vu_sans]).build();
    ///
    /// let one_font_brush: GlyphBrush<'_, Vertex> = two_font_brush
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

    /// Sets whether perform the calculation of glyph positioning according to the layout
    /// every time, or use a cached result if the input `Section` and `GlyphPositioner` are the
    /// same hash as a previous call.
    ///
    /// Improves performance. Should only disable if using a custom GlyphPositioner that is
    /// impure according to it's inputs, so caching a previous call is not desired. Disabling
    /// also disables [`cache_glyph_drawing`](#method.cache_glyph_drawing).
    ///
    /// Defaults to `true`
    pub fn cache_glyph_positioning(mut self, cache: bool) -> Self {
        self.cache_glyph_positioning = cache;
        self
    }

    /// Sets optimising drawing by reusing the last draw requesting an identical draw queue.
    ///
    /// Improves performance. Is disabled if
    /// [`cache_glyph_positioning`](#method.cache_glyph_positioning) is disabled.
    ///
    /// Defaults to `true`
    pub fn cache_glyph_drawing(mut self, cache: bool) -> Self {
        self.cache_glyph_drawing = cache;
        self
    }

    /// Sets the section hasher. `GlyphBrush` cannot handle absolute section hash collisions
    /// so use a good hash algorithm.
    ///
    /// This hasher is used to distinguish sections, rather than for hashmap internal use.
    ///
    /// Defaults to [xxHash](https://docs.rs/twox-hash).
    ///
    /// # Example
    /// ```
    /// # use glyph_brush::GlyphBrushBuilder;
    /// # let some_font: &[u8] = include_bytes!("../../../fonts/DejaVuSans.ttf");
    /// # type SomeOtherBuildHasher = glyph_brush::DefaultSectionHasher;
    /// GlyphBrushBuilder::using_font_bytes(some_font)
    ///     .section_hasher(SomeOtherBuildHasher::default())
    ///     // ...
    /// # ;
    /// ```
    pub fn section_hasher<T: BuildHasher>(self, section_hasher: T) -> GlyphBrushBuilder<'a, T> {
        GlyphBrushBuilder {
            section_hasher,
            font_data: self.font_data,
            cache_glyph_positioning: self.cache_glyph_positioning,
            cache_glyph_drawing: self.cache_glyph_drawing,
            gpu_cache_builder: self.gpu_cache_builder,
            _private_construction: (),
        }
    }

    /// Builds a `GlyphBrush` using the input gfx factory
    pub fn build<V>(self) -> GlyphBrush<'a, V, H> {
        GlyphBrush {
            fonts: self.font_data,
            texture_cache: self.gpu_cache_builder.build(),

            last_draw: <_>::default(),
            section_buffer: <_>::default(),
            calculate_glyph_cache: <_>::default(),

            last_frame_seq_id_sections: <_>::default(),
            frame_seq_id_sections: <_>::default(),

            keep_in_cache: <_>::default(),

            cache_glyph_positioning: self.cache_glyph_positioning,
            cache_glyph_drawing: self.cache_glyph_drawing && self.cache_glyph_positioning,

            section_hasher: self.section_hasher,

            last_pre_positioned: <_>::default(),
            pre_positioned: <_>::default(),
        }
    }

    /// Rebuilds an existing `GlyphBrush` with this builder's properties. This will clear all
    /// caches and queues.
    ///
    /// # Example
    /// ```
    /// # use glyph_brush::{*, rusttype::*};
    /// # let sans = Font::from_bytes(&include_bytes!("../../../fonts/DejaVuSans.ttf")[..]).unwrap();
    /// # type Vertex = ();
    /// let mut glyph_brush: GlyphBrush<'_, Vertex> = GlyphBrushBuilder::using_font(sans).build();
    /// assert_eq!(glyph_brush.texture_dimensions(), (256, 256));
    ///
    /// // Use a new builder to rebuild the brush with a smaller initial cache size
    /// glyph_brush.to_builder().initial_cache_size((64, 64)).rebuild(&mut glyph_brush);
    /// assert_eq!(glyph_brush.texture_dimensions(), (64, 64));
    /// ```
    pub fn rebuild<V>(self, brush: &mut GlyphBrush<'a, V, H>) {
        std::mem::replace(brush, self.build());
    }
}

/// Macro to delegate builder methods to an inner `glyph_brush::GlyphBrushBuilder`
///
/// Implements:
/// * `add_font_bytes`
/// * `add_font`
/// * `initial_cache_size`
/// * `gpu_cache_scale_tolerance`
/// * `gpu_cache_position_tolerance`
/// * `gpu_cache_align_4x4`
/// * `cache_glyph_positioning`
/// * `cache_glyph_drawing`
///
/// # Example
/// ```
/// use glyph_brush::*;
/// use std::hash::BuildHasher;
///
/// # pub struct DownstreamGlyphBrush;
/// pub struct DownstreamGlyphBrushBuilder<'a, H> {
///     inner: glyph_brush::GlyphBrushBuilder<'a, H>,
///     some_config: bool,
/// }
///
/// impl<'a, H: BuildHasher> DownstreamGlyphBrushBuilder<'a, H> {
///     delegate_glyph_brush_builder_fns!(inner);
///
///     /// Sets some downstream configuration
///     pub fn some_config(mut self, some_config: bool) -> Self {
///         self.some_config = some_config;
///         self
///     }
///
///     // Must be manually delegated
///     pub fn section_hasher<T: BuildHasher>(
///         self,
///         section_hasher: T,
///     ) -> DownstreamGlyphBrushBuilder<'a, T> {
///         DownstreamGlyphBrushBuilder {
///             inner: self.inner.section_hasher(section_hasher),
///             some_config: self.some_config,
///         }
///     }
///
///     pub fn build(self) -> DownstreamGlyphBrush {
///         // ...
///         # DownstreamGlyphBrush
///     }
/// }
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! delegate_glyph_brush_builder_fns {
    ($inner:ident) => {
        /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
        /// [`using_font_bytes`](#method.using_font_bytes).
        /// Returns a [`FontId`](struct.FontId.html) to reference this font.
        pub fn add_font_bytes<B: Into<$crate::rusttype::SharedBytes<'a>>>(&mut self, font_data: B) -> $crate::FontId {
            self.$inner.add_font_bytes(font_data)
        }

        /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
        /// [`using_font_bytes`](#method.using_font_bytes).
        /// Returns a [`FontId`](struct.FontId.html) to reference this font.
        pub fn add_font(&mut self, font_data: $crate::rusttype::Font<'a>) -> $crate::FontId {
            self.$inner.add_font(font_data)
        }

        /// Initial size of 2D texture used as a gpu cache, pixels (width, height).
        /// The GPU cache will dynamically quadruple in size whenever the current size
        /// is insufficient.
        ///
        /// Defaults to `(256, 256)`
        pub fn initial_cache_size(mut self, size: (u32, u32)) -> Self {
            self.$inner = self.$inner.initial_cache_size(size);
            self
        }

        /// Sets the maximum allowed difference in scale used for judging whether to reuse an
        /// existing glyph in the GPU cache.
        ///
        /// Defaults to `0.5`
        ///
        /// See rusttype docs for `rusttype::gpu_cache::Cache`
        pub fn gpu_cache_scale_tolerance(mut self, tolerance: f32) -> Self {
            self.$inner = self.$inner.gpu_cache_scale_tolerance(tolerance);
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
            self.$inner = self.$inner.gpu_cache_position_tolerance(tolerance);
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
        pub fn gpu_cache_align_4x4(mut self, b: bool) -> Self {
            self.$inner = self.$inner.gpu_cache_align_4x4(b);
            self
        }

        /// Sets whether perform the calculation of glyph positioning according to the layout
        /// every time, or use a cached result if the input `Section` and `GlyphPositioner` are the
        /// same hash as a previous call.
        ///
        /// Improves performance. Should only disable if using a custom GlyphPositioner that is
        /// impure according to it's inputs, so caching a previous call is not desired. Disabling
        /// also disables [`cache_glyph_drawing`](#method.cache_glyph_drawing).
        ///
        /// Defaults to `true`
        pub fn cache_glyph_positioning(mut self, cache: bool) -> Self {
            self.$inner = self.$inner.cache_glyph_positioning(cache);
            self
        }

        /// Sets optimising drawing by reusing the last draw requesting an identical draw queue.
        ///
        /// Improves performance. Is disabled if
        /// [`cache_glyph_positioning`](#method.cache_glyph_positioning) is disabled.
        ///
        /// Defaults to `true`
        pub fn cache_glyph_drawing(mut self, cache: bool) -> Self {
            self.$inner = self.$inner.cache_glyph_drawing(cache);
            self
        }
    }
}
