use super::*;
use glyph_brush::delegate_glyph_brush_builder_fns;

/// Builder for a [`GlyphBrush`](struct.GlyphBrush.html).
///
/// # Example
///
/// ```no_run
/// use gfx_glyph::GlyphBrushBuilder;
/// # use old_school_gfx_glutin_ext::*;
/// # let event_loop = glutin::event_loop::EventLoop::new();
/// # let window_builder = glutin::window::WindowBuilder::new();
/// # let (_window, _device, mut gfx_factory, gfx_color, gfx_depth) =
/// #     glutin::ContextBuilder::new()
/// #         .build_windowed(window_builder, &event_loop)
/// #         .unwrap()
/// #         .init_gfx::<gfx::format::Srgba8, gfx::format::Depth>();
///
/// let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
/// let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build(gfx_factory.clone());
/// # let _ = glyph_brush;
/// ```
pub struct GlyphBrushBuilder<F, H = DefaultSectionHasher> {
    inner: glyph_brush::GlyphBrushBuilder<F, H>,
    depth_test: gfx::state::Depth,
    texture_filter_method: texture::FilterMethod,
}

impl GlyphBrushBuilder<()> {
    /// Specifies the default font data used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    #[inline]
    pub fn using_font_bytes<'a>(font_0_data: &'a [u8]) -> GlyphBrushBuilder<FontRef<'a>> {
        Self::using_fonts_bytes(std::iter::once(font_0_data))
    }

    #[inline]
    pub fn using_fonts_bytes<'a, V>(font_data: V) -> GlyphBrushBuilder<FontRef<'a>>
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
    ///
    /// # Example
    /// ```
    /// # use glyph_brush::{*, ab_glyph::*};
    /// # type Vertex = ();
    /// # let open_sans = FontRef::try_from_slice(&include_bytes!("../../fonts/DejaVuSans.ttf")[..]).unwrap();
    /// # let deja_vu_sans = open_sans.clone();
    /// let two_font_brush: GlyphBrush<FontRef<'static>, Vertex>
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
        let new_inner = self.inner.replace_fonts(font_fn);
        GlyphBrushBuilder {
            inner: new_inner,
            depth_test: self.depth_test,
            texture_filter_method: self.texture_filter_method,
        }
    }
}

impl<'a, H: BuildHasher> GlyphBrushBuilder<FontRef<'a>, H> {
    pub fn add_font_bytes(&mut self, font_data: &'a [u8]) -> FontId {
        self.inner.add_font_bytes(font_data)
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
    /// # let some_font: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// GlyphBrushBuilder::using_font_bytes(some_font)
    ///     .depth_test(gfx::preset::depth::PASS_WRITE)
    ///     // ...
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
    /// # let some_font: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// GlyphBrushBuilder::using_font_bytes(some_font)
    ///     .texture_filter_method(gfx::texture::FilterMethod::Scale)
    ///     // ...
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
    /// # let some_font: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// # type SomeOtherBuildHasher = std::collections::hash_map::RandomState;
    /// GlyphBrushBuilder::using_font_bytes(some_font)
    ///     .section_hasher(SomeOtherBuildHasher::default())
    ///     // ...
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
    pub fn build<R, GF>(self, mut factory: GF) -> GlyphBrush<F, R, GF, H>
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
