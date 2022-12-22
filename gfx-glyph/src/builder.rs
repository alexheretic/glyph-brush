use super::*;
use glyph_brush::delegate_glyph_brush_builder_fns;

/// Builder for a [`GlyphBrush`](struct.GlyphBrush.html).
///
/// # Example
///
/// ```no_run
/// use gfx_glyph::{ab_glyph::FontArc, GlyphBrushBuilder};
/// # let gfx_factory: gfx_device_gl::Factory = unimplemented!();
///
/// let dejavu = FontArc::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf")).unwrap();
/// let mut glyph_brush = GlyphBrushBuilder::using_font(dejavu).build(gfx_factory.clone());
/// ```
pub struct GlyphBrushBuilder<F = FontArc, H = DefaultSectionHasher> {
    inner: glyph_brush::GlyphBrushBuilder<F, H>,
    depth_test: gfx::state::Depth,
    texture_filter_method: texture::FilterMethod,
}

impl GlyphBrushBuilder<()> {
    /// Specifies the default font used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    #[inline]
    pub fn using_font<F: Font>(font_0: F) -> GlyphBrushBuilder<F> {
        Self::using_fonts(vec![font_0])
    }

    pub fn using_fonts<F: Font, V: Into<Vec<F>>>(fonts: V) -> GlyphBrushBuilder<F> {
        Self::without_fonts().replace_fonts(|_| fonts)
    }

    /// Create a new builder without any fonts.
    pub fn without_fonts() -> Self {
        GlyphBrushBuilder {
            inner: glyph_brush::GlyphBrushBuilder::without_fonts(),
            depth_test: gfx::preset::depth::LESS_EQUAL_WRITE,
            texture_filter_method: texture::FilterMethod::Bilinear,
        }
    }
}

impl<F, H> GlyphBrushBuilder<F, H> {
    /// Consume all builder fonts a replace with new fonts returned by the input function.
    ///
    /// Generally only makes sense when wanting to change fonts after calling
    /// [`GlyphBrush::to_builder`](struct.GlyphBrush.html#method.to_builder). Or on
    /// a `GlyphBrushBuilder<()>` built using `without_fonts()`.
    pub fn replace_fonts<F2: Font, V, NF>(self, font_fn: NF) -> GlyphBrushBuilder<F2, H>
    where
        V: Into<Vec<F2>>,
        NF: FnOnce(Vec<F>) -> V,
    {
        let new_inner = self.inner.replace_fonts(font_fn);
        GlyphBrushBuilder {
            inner: new_inner,
            depth_test: self.depth_test,
            texture_filter_method: self.texture_filter_method,
        }
    }
}

impl<F, H> GlyphBrushBuilder<F, H>
where
    F: Font,
    H: BuildHasher,
{
    delegate_glyph_brush_builder_fns!(inner);

    /// Sets the depth test to use on the text section **z** values.
    ///
    /// Defaults to: *Only draw when the fragment's output depth is less than or equal to
    /// the current depth buffer value, and update the buffer*.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use gfx_glyph::GlyphBrushBuilder;
    /// # let some_font: gfx_glyph::ab_glyph::FontArc = unimplemented!();
    /// GlyphBrushBuilder::using_font(some_font).depth_test(gfx::preset::depth::PASS_WRITE)
    /// // ...
    /// # ;
    /// ```
    pub fn depth_test(mut self, depth_test: gfx::state::Depth) -> Self {
        self.depth_test = depth_test;
        self
    }

    /// Sets the texture filtering method.
    ///
    /// Defaults to `Bilinear`
    ///
    /// # Example
    /// ```no_run
    /// # use gfx_glyph::GlyphBrushBuilder;
    /// # let some_font: gfx_glyph::ab_glyph::FontArc = unimplemented!();
    /// GlyphBrushBuilder::using_font(some_font)
    ///     .texture_filter_method(gfx::texture::FilterMethod::Scale)
    /// // ...
    /// # ;
    /// ```
    pub fn texture_filter_method(mut self, filter_method: texture::FilterMethod) -> Self {
        self.texture_filter_method = filter_method;
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
    /// # use gfx_glyph::GlyphBrushBuilder;
    /// # let some_font: gfx_glyph::ab_glyph::FontArc = unimplemented!();
    /// # type SomeOtherBuildHasher = std::collections::hash_map::RandomState;
    /// GlyphBrushBuilder::using_font(some_font).section_hasher(SomeOtherBuildHasher::default())
    /// // ...
    /// # ;
    /// ```
    pub fn section_hasher<T: BuildHasher>(self, section_hasher: T) -> GlyphBrushBuilder<F, T> {
        GlyphBrushBuilder {
            inner: self.inner.section_hasher(section_hasher),
            depth_test: self.depth_test,
            texture_filter_method: self.texture_filter_method,
        }
    }

    /// Builds a `GlyphBrush` using the input gfx factory
    pub fn build<R, GF>(self, mut factory: GF) -> GlyphBrush<R, GF, F, H>
    where
        R: gfx::Resources,
        GF: gfx::Factory<R>,
    {
        let inner = self.inner.build();
        let (cache_width, cache_height) = inner.texture_dimensions();
        let font_cache_tex = create_texture(&mut factory, cache_width, cache_height).unwrap();
        let program = factory
            .link_program(
                include_bytes!("shader/vert.glsl"),
                include_bytes!("shader/frag.glsl"),
            )
            .unwrap();

        GlyphBrush {
            font_cache_tex,
            texture_filter_method: self.texture_filter_method,
            glyph_brush: inner,

            factory,
            program,
            draw_cache: None,

            depth_test: self.depth_test,
        }
    }
}
