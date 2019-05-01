use super::*;
use glyph_brush::delegate_glyph_brush_builder_fns;

/// Builder for a [`GlyphBrush`](struct.GlyphBrush.html).
///
/// # Example
///
/// ```no_run
/// use gfx_glyph::GlyphBrushBuilder;
/// # fn main() {
/// # let events_loop = glutin::EventsLoop::new();
/// # let (_window, _device, gfx_factory, _gfx_target, _main_depth) =
/// #     gfx_window_glutin::init::<gfx::format::Srgba8, gfx::format::Depth>(
/// #         glutin::WindowBuilder::new(),
/// #         glutin::ContextBuilder::new(),
/// #         &events_loop).unwrap();
///
/// let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
/// let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build(gfx_factory.clone());
/// # let _ = glyph_brush;
/// # }
/// ```
pub struct GlyphBrushBuilder<'a, H = DefaultSectionHasher> {
    inner: glyph_brush::GlyphBrushBuilder<'a, H>,
    depth_test: gfx::state::Depth,
    texture_filter_method: texture::FilterMethod,
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
            inner: glyph_brush::GlyphBrushBuilder::using_fonts(fonts),
            depth_test: gfx::preset::depth::LESS_EQUAL_WRITE,
            texture_filter_method: texture::FilterMethod::Bilinear,
        }
    }
}

impl<'a, H: BuildHasher> GlyphBrushBuilder<'a, H> {
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
    /// # fn main() {
    /// # let some_font: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// GlyphBrushBuilder::using_font_bytes(some_font)
    ///     .depth_test(gfx::preset::depth::PASS_WRITE)
    ///     // ...
    /// # ;
    /// # }
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
    /// # fn main() {
    /// # let some_font: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// GlyphBrushBuilder::using_font_bytes(some_font)
    ///     .texture_filter_method(gfx::texture::FilterMethod::Scale)
    ///     // ...
    /// # ;
    /// # }
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
    /// Defaults to [seahash](https://docs.rs/seahash).
    ///
    /// # Example
    /// ```no_run
    /// # use gfx_glyph::GlyphBrushBuilder;
    /// # fn main() {
    /// # let some_font: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    /// # type SomeOtherBuildHasher = std::collections::hash_map::RandomState;
    /// GlyphBrushBuilder::using_font_bytes(some_font)
    ///     .section_hasher(SomeOtherBuildHasher::default())
    ///     // ...
    /// # ;
    /// # }
    /// ```
    pub fn section_hasher<T: BuildHasher>(self, section_hasher: T) -> GlyphBrushBuilder<'a, T> {
        GlyphBrushBuilder {
            inner: self.inner.section_hasher(section_hasher),
            depth_test: self.depth_test,
            texture_filter_method: self.texture_filter_method,
        }
    }

    /// Builds a `GlyphBrush` using the input gfx factory
    pub fn build<R, F>(self, mut factory: F) -> GlyphBrush<'a, R, F, H>
    where
        R: gfx::Resources,
        F: gfx::Factory<R>,
    {
        let (cache_width, cache_height) = self.inner.initial_cache_size;
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
            glyph_brush: self.inner.build(),

            factory,
            program,
            draw_cache: None,

            depth_test: self.depth_test,
        }
    }
}
