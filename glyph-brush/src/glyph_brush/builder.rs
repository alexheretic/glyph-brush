use crate::{DefaultSectionHasher, Font, FontId, GlyphBrush};
use glyph_brush_draw_cache::*;
use glyph_brush_layout::ab_glyph::*;
use std::hash::BuildHasher;

/// Builder for a [`GlyphBrush`].
///
/// # Example
/// ```
/// use glyph_brush::{ab_glyph::FontArc, GlyphBrush, GlyphBrushBuilder};
/// # type Vertex = ();
///
/// let dejavu = FontArc::try_from_slice(include_bytes!("../../../fonts/DejaVuSans.ttf")).unwrap();
/// let mut glyph_brush: GlyphBrush<Vertex> = GlyphBrushBuilder::using_font(dejavu).build();
/// ```
#[non_exhaustive]
pub struct GlyphBrushBuilder<F = FontArc, H = DefaultSectionHasher> {
    pub font_data: Vec<F>,
    pub cache_glyph_positioning: bool,
    pub cache_redraws: bool,
    pub section_hasher: H,
    pub outline_draw_cache_builder: DrawCacheBuilder<OutlinedGlyph>,
    pub emoji_draw_cache_builder: DrawCacheBuilder<ImageGlyph>,
}

impl GlyphBrushBuilder<()> {
    /// Create a new builder with multiple fonts.
    pub fn using_fonts<F: Font>(fonts: Vec<F>) -> GlyphBrushBuilder<F> {
        Self::without_fonts().replace_fonts(|_| fonts)
    }

    /// Create a new builder with multiple fonts.
    #[inline]
    pub fn using_font<F: Font>(font: F) -> GlyphBrushBuilder<F> {
        Self::using_fonts(vec![font])
    }

    /// Create a new builder without any fonts.
    pub fn without_fonts() -> GlyphBrushBuilder<()> {
        GlyphBrushBuilder {
            font_data: Vec::new(),
            cache_glyph_positioning: true,
            cache_redraws: true,
            section_hasher: DefaultSectionHasher::default(),
            outline_draw_cache_builder: DrawCache::builder()
                .dimensions(256, 256)
                .scale_tolerance(0.5)
                .position_tolerance(0.1)
                .align_4x4(false),
            emoji_draw_cache_builder: DrawCache::builder()
                .dimensions(256, 256)
                .scale_tolerance(0.5)
                .position_tolerance(0.1)
                .multithread(false)
                .align_4x4(false),
        }
    }
}

impl<F, H> GlyphBrushBuilder<F, H> {
    /// Consume all builder fonts a replace with new fonts returned by the input function.
    ///
    /// Generally only makes sense when wanting to change fonts after calling
    /// [`GlyphBrush::to_builder`](struct.GlyphBrush.html#method.to_builder).
    ///
    /// # Example
    /// ```
    /// # use glyph_brush::{*, ab_glyph::*};
    /// # type Vertex = ();
    /// # let open_sans = FontArc::try_from_slice(&include_bytes!("../../../fonts/DejaVuSans.ttf")[..]).unwrap();
    /// # let deja_vu_sans = open_sans.clone();
    /// let two_font_brush: GlyphBrush<Vertex>
    ///     = GlyphBrushBuilder::using_fonts(vec![open_sans, deja_vu_sans]).build();
    ///
    /// let one_font_brush: GlyphBrush<FontRef<'static>, Vertex> = two_font_brush
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
    pub fn replace_fonts<F2: Font, V, NF>(self, font_fn: NF) -> GlyphBrushBuilder<F2, H>
    where
        V: Into<Vec<F2>>,
        NF: FnOnce(Vec<F>) -> V,
    {
        let font_data = font_fn(self.font_data).into();
        GlyphBrushBuilder {
            font_data,
            cache_glyph_positioning: self.cache_glyph_positioning,
            cache_redraws: self.cache_redraws,
            section_hasher: self.section_hasher,
            outline_draw_cache_builder: self.outline_draw_cache_builder,
            emoji_draw_cache_builder: self.emoji_draw_cache_builder,
        }
    }
}

impl<F: Font, H: BuildHasher> GlyphBrushBuilder<F, H> {
    /// Adds additional fonts to the one added in [`using_font`](#method.using_font).
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font<I: Into<F>>(&mut self, font_data: I) -> FontId {
        self.font_data.push(font_data.into());
        FontId(self.font_data.len() - 1)
    }

    /// Initial size of 2D texture used as a gpu cache, pixels (width, height).
    /// The GPU cache will dynamically quadruple in size whenever the current size
    /// is insufficient.
    ///
    /// Defaults to `(256, 256)`
    pub fn initial_cache_size(mut self, (w, h): (u32, u32)) -> Self {
        self.outline_draw_cache_builder = self.outline_draw_cache_builder.dimensions(w, h);
        self.emoji_draw_cache_builder = self.emoji_draw_cache_builder.dimensions(w, h);
        self
    }

    /// Sets the maximum allowed difference in scale used for judging whether to reuse an
    /// existing glyph in the GPU cache.
    ///
    /// Defaults to `0.5`
    ///
    /// See docs for `glyph_brush_draw_cache::DrawCache`
    pub fn outline_draw_cache_scale_tolerance(mut self, tolerance: f32) -> Self {
        self.outline_draw_cache_builder = self.outline_draw_cache_builder.scale_tolerance(tolerance);
        self
    }

    pub fn emoji_draw_cache_scale_tolerance(mut self, tolerance: f32) -> Self {
        self.emoji_draw_cache_builder = self.emoji_draw_cache_builder.scale_tolerance(tolerance);
        self
    }

    /// Sets the maximum allowed difference in subpixel position used for judging whether
    /// to reuse an existing glyph in the GPU cache. Anything greater than or equal to
    /// 1.0 means "don't care".
    ///
    /// Defaults to `0.1`
    ///
    /// See docs for `glyph_brush_draw_cache::DrawCache`
    pub fn outline_draw_cache_position_tolerance(mut self, tolerance: f32) -> Self {
        self.outline_draw_cache_builder = self.outline_draw_cache_builder.position_tolerance(tolerance);
        self
    }

    pub fn emoji_draw_cache_position_tolerance(mut self, tolerance: f32) -> Self {
        self.emoji_draw_cache_builder = self.emoji_draw_cache_builder.position_tolerance(tolerance);
        self
    }
    /// When multiple CPU cores are available spread draw-cache work across all cores.
    ///
    /// Defaults to `true`.
    pub fn outline_multithread(mut self, multithread: bool) -> Self {
        self.outline_draw_cache_builder = self.outline_draw_cache_builder.multithread(multithread);
        self
    }

    pub fn emoji_multithread(mut self, multithread: bool) -> Self {
        self.emoji_draw_cache_builder = self.emoji_draw_cache_builder.multithread(multithread);
        self
    }

    /// Align glyphs in texture cache to 4x4 texel boundaries.
    ///
    /// If your backend requires texture updates to be aligned to 4x4 texel
    /// boundaries (e.g. WebGL), this should be set to `true`.
    ///
    /// Defaults to `false`
    ///
    /// See docs for `glyph_brush_draw_cache::DrawCache`
    pub fn outline_draw_cache_align_4x4(mut self, align: bool) -> Self {
        self.outline_draw_cache_builder = self.outline_draw_cache_builder.align_4x4(align);
        self
    }

    pub fn emoji_draw_cache_align_4x4(mut self, align: bool) -> Self {
        self.emoji_draw_cache_builder = self.emoji_draw_cache_builder.align_4x4(align);
        self
    }

    /// Sets whether perform the calculation of glyph positioning according to the layout
    /// every time, or use a cached result if the input `Section` and `GlyphPositioner` are the
    /// same hash as a previous call.
    ///
    /// Improves performance. Should only disable if using a custom GlyphPositioner that is
    /// impure according to it's inputs, so caching a previous call is not desired. Disabling
    /// also disables [`cache_redraws`](#method.cache_redraws).
    ///
    /// Defaults to `true`
    pub fn cache_glyph_positioning(mut self, cache: bool) -> Self {
        self.cache_glyph_positioning = cache;
        self
    }

    /// Sets optimising vertex drawing by reusing the last draw requesting an identical draw queue.
    /// Will result in the usage of [`BrushAction::ReDraw`](enum.BrushAction.html#variant.ReDraw).
    ///
    /// Improves performance. Is disabled if
    /// [`cache_glyph_positioning`](#method.cache_glyph_positioning) is disabled.
    ///
    /// Defaults to `true`
    pub fn cache_redraws(mut self, cache_redraws: bool) -> Self {
        self.cache_redraws = cache_redraws;
        self
    }

    /// Sets the section hasher. [`GlyphBrush`] cannot handle absolute section hash collisions
    /// so use a good hash algorithm.
    ///
    /// This hasher is used to distinguish sections, rather than for hashmap internal use.
    ///
    /// Defaults to [xxHash](https://docs.rs/twox-hash).
    ///
    /// # Example
    /// ```
    /// # use glyph_brush::{ab_glyph::*, GlyphBrushBuilder};
    /// # let some_font = FontArc::try_from_slice(include_bytes!("../../../fonts/DejaVuSans.ttf")).unwrap();
    /// # type SomeOtherBuildHasher = glyph_brush::DefaultSectionHasher;
    /// GlyphBrushBuilder::using_font(some_font)
    ///     .section_hasher(SomeOtherBuildHasher::default())
    ///     // ...
    /// # ;
    /// ```
    pub fn section_hasher<T: BuildHasher>(self, section_hasher: T) -> GlyphBrushBuilder<F, T> {
        GlyphBrushBuilder {
            section_hasher,
            font_data: self.font_data,
            cache_glyph_positioning: self.cache_glyph_positioning,
            cache_redraws: self.cache_redraws,
            outline_draw_cache_builder: self.outline_draw_cache_builder,
            emoji_draw_cache_builder: self.emoji_draw_cache_builder,
        }
    }

    /// Builds a [`GlyphBrush`].
    ///
    /// If type inference fails try declaring the types `V` & `X`.
    /// See [`GlyphBrush` generic types](struct.GlyphBrush.html#generic-types).
    /// ```
    /// # use glyph_brush::{ab_glyph::*, GlyphBrushBuilder};
    /// # let some_font = FontArc::try_from_slice(include_bytes!("../../../fonts/DejaVuSans.ttf")).unwrap();
    /// # type SomeOtherBuildHasher = glyph_brush::DefaultSectionHasher;
    /// # type Vertex = ();
    /// let glyph_brush = GlyphBrushBuilder::using_font(some_font)
    ///     .build::<Vertex, glyph_brush::Extra>();
    /// ```
    pub fn build<V, X>(self) -> GlyphBrush<V, X, F, H> {
        GlyphBrush {
            fonts: self.font_data,
            outline_cache: self.outline_draw_cache_builder.build(),
            emoji_cache: self.emoji_draw_cache_builder.build(),

            last_draw: <_>::default(),
            section_buffer: <_>::default(),
            calculate_glyph_cache: <_>::default(),

            last_frame_seq_id_sections: <_>::default(),
            frame_seq_id_sections: <_>::default(),

            keep_in_cache: <_>::default(),

            cache_glyph_positioning: self.cache_glyph_positioning,
            cache_redraws: self.cache_redraws && self.cache_glyph_positioning,

            section_hasher: self.section_hasher,

            last_pre_positioned: <_>::default(),
            pre_positioned: <_>::default(),
        }
    }

    /// Rebuilds an existing [`GlyphBrush`] with this builder's properties. This will clear all
    /// caches and queues.
    ///
    /// # Example
    /// ```
    /// # use glyph_brush::{*, ab_glyph::*};
    /// # let sans = FontArc::try_from_slice(include_bytes!("../../../fonts/DejaVuSans.ttf")).unwrap();
    /// # type Vertex = ();
    /// let mut glyph_brush: GlyphBrush<Vertex> = GlyphBrushBuilder::using_font(sans).build();
    /// assert_eq!(glyph_brush.texture_dimensions(), (256, 256));
    ///
    /// // Use a new builder to rebuild the brush with a smaller initial cache size
    /// glyph_brush.to_builder().initial_cache_size((64, 64)).rebuild(&mut glyph_brush);
    /// assert_eq!(glyph_brush.texture_dimensions(), (64, 64));
    /// ```
    pub fn rebuild<V, X>(self, brush: &mut GlyphBrush<V, X, F, H>) {
        *brush = self.build();
    }
}

/// Macro to delegate builder methods to an inner `glyph_brush::GlyphBrushBuilder`
///
/// Implements:
/// * `add_font_bytes`
/// * `add_font`
/// * `initial_cache_size`
/// * `draw_cache_scale_tolerance`
/// * `draw_cache_position_tolerance`
/// * `draw_cache_align_4x4`
/// * `cache_glyph_positioning`
/// * `cache_redraws`
///
/// # Example
/// ```
/// use glyph_brush::{ab_glyph::*, *};
/// use std::hash::BuildHasher;
///
/// # pub struct DownstreamGlyphBrush;
/// pub struct DownstreamGlyphBrushBuilder<F, H> {
///     inner: glyph_brush::GlyphBrushBuilder<F, H>,
///     some_config: bool,
/// }
///
/// impl<F: Font, H: BuildHasher> DownstreamGlyphBrushBuilder<F, H> {
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
///     ) -> DownstreamGlyphBrushBuilder<F, T> {
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
        /// Adds additional fonts to the one added in [`using_font`](#method.using_font).
        /// Returns a [`FontId`](struct.FontId.html) to reference this font.
        pub fn add_font(&mut self, font_data: F) -> $crate::FontId {
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
        /// See docs for `glyph_brush_draw_cache::DrawCache`
        pub fn outline_draw_cache_scale_tolerance(mut self, tolerance: f32) -> Self {
            self.$inner = self.$inner.outline_draw_cache_scale_tolerance(tolerance);
            self
        }

        pub fn emoji_draw_cache_scale_tolerance(mut self, tolerance: f32) -> Self {
            self.$inner = self.$inner.emoji_draw_cache_scale_tolerance(tolerance);
            self
        }

        /// Sets the maximum allowed difference in subpixel position used for judging whether
        /// to reuse an existing glyph in the GPU cache. Anything greater than or equal to
        /// 1.0 means "don't care".
        ///
        /// Defaults to `0.1`
        ///
        /// See docs for `glyph_brush_draw_cache::DrawCache`
        pub fn outline_draw_cache_position_tolerance(mut self, tolerance: f32) -> Self {
            self.$inner = self.$inner.outline_draw_cache_position_tolerance(tolerance);
            self
        }

        pub fn emoji_draw_cache_position_tolerance(mut self, tolerance: f32) -> Self {
            self.$inner = self.$inner.emoji_draw_cache_position_tolerance(tolerance);
            self
        }

        /// Align glyphs in texture cache to 4x4 texel boundaries.
        ///
        /// If your backend requires texture updates to be aligned to 4x4 texel
        /// boundaries (e.g. WebGL), this should be set to `true`.
        ///
        /// Defaults to `false`
        ///
        /// See docs for `glyph_brush_draw_cache::DrawCache`
        pub fn outline_draw_cache_align_4x4(mut self, b: bool) -> Self {
            self.$inner = self.$inner.outline_draw_cache_align_4x4(b);
            self
        }

        pub fn emoji_draw_cache_align_4x4(mut self, b: bool) -> Self {
            self.$inner = self.$inner.emoji_draw_cache_align_4x4(b);
            self
        }

        /// Sets whether perform the calculation of glyph positioning according to the layout
        /// every time, or use a cached result if the input `Section` and `GlyphPositioner` are the
        /// same hash as a previous call.
        ///
        /// Improves performance. Should only disable if using a custom GlyphPositioner that is
        /// impure according to it's inputs, so caching a previous call is not desired. Disabling
        /// also disables [`cache_redraws`](#method.cache_redraws).
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
        pub fn cache_redraws(mut self, cache: bool) -> Self {
            self.$inner = self.$inner.cache_redraws(cache);
            self
        }
    };
}
