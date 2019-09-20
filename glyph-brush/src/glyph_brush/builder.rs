use crate::{
    rusttype::{Font, SharedBytes},
    DefaultSectionHasher, FontId, GlyphBrush,
};
use std::hash::BuildHasher;

/// Builder for a [`GlyphBrush`](struct.GlyphBrush.html) (v0.5).
///
/// # Example
///
/// ```no_run
/// use glyph_brush::{GlyphBrush, GlyphBrushBuilder};
/// # type Vertex = ();
///
/// let dejavu: &[u8] = include_bytes!("../../../fonts/DejaVuSans.ttf");
/// let mut glyph_brush: GlyphBrush<'_, Vertex> =
///     GlyphBrushBuilder::using_font_bytes(dejavu).build();
/// ```
pub struct GlyphBrushBuilder<'a, H = DefaultSectionHasher> {
    pub font_data: Vec<Font<'a>>,
    pub initial_cache_size: (u32, u32),
    pub gpu_cache_scale_tolerance: f32,
    pub gpu_cache_position_tolerance: f32,
    pub gpu_cache_align_4x4: bool,
    pub cache_glyph_positioning: bool,
    pub cache_glyph_drawing: bool,
    pub section_hasher: H,
    _private_construction: (),
}

impl<'a> GlyphBrushBuilder<'a> {
    /// Specifies the default font data used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    #[inline]
    pub fn using_font_bytes<B: Into<SharedBytes<'a>>>(font_0_data: B) -> Self {
        Self::using_font(Font::from_bytes(font_0_data).unwrap())
    }

    #[inline]
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
    #[inline]
    pub fn using_font(font_0: Font<'a>) -> Self {
        Self::using_fonts(vec![font_0])
    }

    pub fn using_fonts<V: Into<Vec<Font<'a>>>>(fonts: V) -> Self {
        GlyphBrushBuilder {
            font_data: fonts.into(),
            initial_cache_size: (256, 256),
            gpu_cache_scale_tolerance: 0.5,
            gpu_cache_position_tolerance: 0.1,
            gpu_cache_align_4x4: false,
            cache_glyph_positioning: true,
            cache_glyph_drawing: true,
            section_hasher: DefaultSectionHasher::default(),
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

    /// Initial size of 2D texture used as a gpu cache, pixels (width, height).
    /// The GPU cache will dynamically quadruple in size whenever the current size
    /// is insufficient.
    ///
    /// Defaults to `(256, 256)`
    pub fn initial_cache_size(mut self, size: (u32, u32)) -> Self {
        self.initial_cache_size = size;
        self
    }

    /// Sets the maximum allowed difference in scale used for judging whether to reuse an
    /// existing glyph in the GPU cache.
    ///
    /// Defaults to `0.5`
    ///
    /// See rusttype docs for `rusttype::gpu_cache::Cache`
    pub fn gpu_cache_scale_tolerance(mut self, tolerance: f32) -> Self {
        self.gpu_cache_scale_tolerance = tolerance;
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
        self.gpu_cache_position_tolerance = tolerance;
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
        self.gpu_cache_align_4x4 = b;
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
    /// ```no_run
    /// # use glyph_brush::GlyphBrushBuilder;
    /// # fn main() {
    /// # let some_font: &[u8] = include_bytes!("../../../fonts/DejaVuSans.ttf");
    /// # type SomeOtherBuildHasher = glyph_brush::DefaultSectionHasher;
    /// GlyphBrushBuilder::using_font_bytes(some_font)
    ///     .section_hasher(SomeOtherBuildHasher::default())
    ///     // ...
    /// # ;
    /// # }
    /// ```
    pub fn section_hasher<T: BuildHasher>(self, section_hasher: T) -> GlyphBrushBuilder<'a, T> {
        GlyphBrushBuilder {
            section_hasher,
            font_data: self.font_data,
            initial_cache_size: self.initial_cache_size,
            gpu_cache_scale_tolerance: self.gpu_cache_scale_tolerance,
            gpu_cache_position_tolerance: self.gpu_cache_position_tolerance,
            gpu_cache_align_4x4: self.gpu_cache_align_4x4,
            cache_glyph_positioning: self.cache_glyph_positioning,
            cache_glyph_drawing: self.cache_glyph_drawing,
            _private_construction: (),
        }
    }

    fn to_next(self) -> glyph_brush_next::GlyphBrushBuilder<'a, H> {
        glyph_brush_next::GlyphBrushBuilder::using_fonts(self.font_data)
            .initial_cache_size(self.initial_cache_size)
            .gpu_cache_scale_tolerance(self.gpu_cache_scale_tolerance)
            .gpu_cache_position_tolerance(self.gpu_cache_position_tolerance)
            .gpu_cache_align_4x4(self.gpu_cache_align_4x4)
            .cache_glyph_positioning(self.cache_glyph_positioning)
            .cache_glyph_drawing(self.cache_glyph_drawing)
            .section_hasher(self.section_hasher)
    }

    /// Builds a `GlyphBrush` using the input gfx factory
    pub fn build<V: Clone + 'static>(self) -> GlyphBrush<'a, V, H> {
        self.to_next().build()
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
