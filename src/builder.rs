use super::*;

/// Builder for a [`GlyphBrush`](struct.GlyphBrush.html).
///
/// # Example
///
/// ```no_run
/// # extern crate gfx;
/// # extern crate gfx_window_glutin;
/// # extern crate glutin;
/// extern crate gfx_glyph;
/// use gfx_glyph::GlyphBrushBuilder;
/// # fn main() {
/// # let events_loop = glutin::EventsLoop::new();
/// # let (_window, _device, gfx_factory, _gfx_target, _main_depth) =
/// #     gfx_window_glutin::init::<gfx::format::Srgba8, gfx::format::Depth>(
/// #         glutin::WindowBuilder::new(),
/// #         glutin::ContextBuilder::new(),
/// #         &events_loop);
///
/// let arial: &[u8] = include_bytes!("../examples/Arial Unicode.ttf");
/// let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(arial)
///     .build(gfx_factory.clone());
/// # let _ = glyph_brush;
/// # }
/// ```
pub struct GlyphBrushBuilder<'a> {
    font_data: Vec<Font<'a>>,
    initial_cache_size: (u32, u32),
    gpu_cache_scale_tolerance: f32,
    gpu_cache_position_tolerance: f32,
    cache_glyph_positioning: bool,
    cache_glyph_drawing: bool,
    depth_test: gfx::state::Depth,
}

impl<'a> GlyphBrushBuilder<'a> {
    /// Specifies the default font data used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font_bytes<B: Into<SharedBytes<'a>>>(font_0_data: B) -> Self {
        Self::using_font(font(font_0_data).unwrap())
    }

    /// Specifies the default font used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font(font_0_data: Font<'a>) -> Self {
        GlyphBrushBuilder {
            font_data: vec![font_0_data],
            initial_cache_size: (256, 256),
            gpu_cache_scale_tolerance: 0.5,
            gpu_cache_position_tolerance: 0.1,
            cache_glyph_positioning: true,
            cache_glyph_drawing: true,
            depth_test: gfx::preset::depth::PASS_TEST,
        }
    }

    /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
    /// [`using_font_bytes`](#method.using_font_bytes).
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font_bytes<B: Into<SharedBytes<'a>>>(&mut self, font_data: B) -> FontId {
        self.font_data.push(font(font_data.into()).unwrap());
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

    /// Sets the depth test to use on the text section **z** values.
    ///
    /// Defaults to: *Always pass the depth test, never write to the depth buffer write*
    ///
    /// # Example
    ///
    /// ```no_run
    /// # extern crate gfx;
    /// # extern crate gfx_glyph;
    /// # use gfx_glyph::GlyphBrushBuilder;
    /// # fn main() {
    /// # let some_font: &[u8] = include_bytes!("../examples/Arial Unicode.ttf");
    /// GlyphBrushBuilder::using_font_bytes(some_font)
    ///     .depth_test(gfx::preset::depth::LESS_EQUAL_WRITE)
    ///     // ...
    /// # ;
    /// # }
    /// ```
    pub fn depth_test(mut self, depth_test: gfx::state::Depth) -> Self {
        self.depth_test = depth_test;
        self
    }

    /// Builds a `GlyphBrush` using the input gfx factory
    pub fn build<R, F>(self, mut factory: F) -> GlyphBrush<'a, R, F>
        where R: gfx::Resources,
              F: gfx::Factory<R>
    {
        let (cache_width, cache_height) = self.initial_cache_size;
        let font_cache_tex = create_texture(&mut factory, cache_width, cache_height).unwrap();

        GlyphBrush {
            fonts: self.font_data.into_iter().enumerate()
                .map(|(idx, data)| (FontId(idx), data))
                .collect(),
            font_cache: Cache::new(
                cache_width,
                cache_height,
                self.gpu_cache_scale_tolerance,
                self.gpu_cache_position_tolerance,
            ),
            font_cache_tex,

            factory,
            draw_cache: None,
            section_buffer: Vec::new(),
            calculate_glyph_cache: HashMap::new(),
            keep_in_cache: HashSet::new(),

            gpu_cache_scale_tolerance: self.gpu_cache_scale_tolerance,
            gpu_cache_position_tolerance: self.gpu_cache_position_tolerance,
            cache_glyph_positioning: self.cache_glyph_positioning,
            cache_glyph_drawing: self.cache_glyph_drawing && self.cache_glyph_positioning,

            depth_test: self.depth_test,
        }
    }
}
